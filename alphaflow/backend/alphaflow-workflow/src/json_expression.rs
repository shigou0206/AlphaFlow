use regex::Regex;
use serde_json::{Value};
use std::collections::HashMap;

/// 节点执行结果: node_id -> JSON
pub type NodeResults = HashMap<String, Value>;

/// 在 JSON 中对字符串进行“局部插值”，查找 `{{ ... }}` 并替换
pub fn resolve_expressions(params: &Value, input_data: &Value, results: &NodeResults) -> Value {
    match params {
        Value::String(s) => {
            let re = Regex::new(r"\{\{\s*(.*?)\s*\}\}").unwrap();
            let mut output = String::new();
            let mut last_index = 0;

            for caps in re.captures_iter(s) {
                let whole = caps.get(0).unwrap();         // "{{ ... }}"
                output.push_str(&s[last_index..whole.start()]);
                let inner_expr = caps.get(1).unwrap().as_str();
                // 解析内部表达式
                match evaluate_subexpr(inner_expr, input_data, results) {
                    Ok(val_str) => output.push_str(&val_str),
                    Err(_) => {
                        // 如果解析失败，保留原 {{ ... }}
                        output.push_str(whole.as_str());
                    }
                }
                last_index = whole.end();
            }
            output.push_str(&s[last_index..]);
            Value::String(output)
        },
        Value::Array(arr) => {
            let new_arr = arr.iter()
                .map(|v| resolve_expressions(v, input_data, results))
                .collect();
            Value::Array(new_arr)
        },
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                let replaced = resolve_expressions(v, input_data, results);
                new_map.insert(k.clone(), replaced);
            }
            Value::Object(new_map)
        },
        _ => params.clone(),
    }
}

/// 对 "表达式主体" 做解析 (不包含外层花括号)，例如 "$json["arr"][1]"
/// 返回最终字符串
pub fn evaluate_subexpr(expr: &str, input_data: &Value, results: &NodeResults) -> Result<String, String> {
    // 1) 如果以 '=' 开头，比如 "= $json[...]"，也去掉
    let expr = expr.trim();
    let mut expr_inner = if expr.starts_with('=') {
        expr[1..].trim()
    } else {
        expr
    };

    // 2) 若还带 {{ }}，也去掉
    //   例如有些测试可能写: "{{ $json["city"] }}"
    //   但这里 evaluate_subexpr 已经是内部了，一般不会传进带 {{}}, 
    //   如果真的带，我们也额外剥一层
    if expr_inner.starts_with("{{") && expr_inner.ends_with("}}") {
        let tmp = &expr_inner[2..expr_inner.len()-2].trim();
        expr_inner = tmp;
    }

    // 现在 expr_inner 应该形如 $json["city"] 或 $node["X"].json["field"][0]
    if expr_inner.starts_with("$json") {
        let val = parse_json_path(expr_inner, input_data)?;
        Ok(value_to_string(&val))
    } else if expr_inner.starts_with("$node") {
        let val = parse_node_path(expr_inner, results)?;
        Ok(value_to_string(&val))
    } else {
        Err(format!("Unsupported expr syntax: {}", expr_inner))
    }
}

/// 解析 $json[...]
fn parse_json_path(path: &str, input_data: &Value) -> Result<Value, String> {
    if !path.starts_with("$json") {
        return Err(format!("Expression must start with $json: {}", path));
    }
    // 去掉 $json
    let remainder = &path["$json".len()..];
    if !remainder.trim_start().starts_with("[") {
        return Err(format!("No [ found after $json: {}", remainder));
    }
    let keys = extract_keys(remainder)?;
    get_value_by_keys(input_data, &keys)
}

/// 解析 $node["X"].json[...] 
fn parse_node_path(path: &str, results: &NodeResults) -> Result<Value, String> {
    if !path.starts_with("$node[") {
        return Err(format!("Expression must start with $node[: {}", path));
    }
    // 找到第一个 ']'
    let end_bracket = path.find(']').ok_or("No closing ] after $node[")?;
    let node_name_raw = &path["$node[".len()..end_bracket];
    let node_name = node_name_raw.trim().trim_matches('"');
    if node_name.is_empty() {
        return Err("No node name found".to_string());
    }
    let after_bracket = &path[end_bracket..]; // 例: "].json["field"][0]"
    if !after_bracket.starts_with("].json[") {
        return Err(format!("missing .json[...] in expression: {}", path));
    }
    let after_json = &after_bracket["].json".len()..]; // 例: "[\"field\"][0]"
    let node_val = results.get(node_name)
        .ok_or_else(|| format!("Node '{}' not found in results", node_name))?;
    let keys = extract_keys(after_json)?;
    get_value_by_keys(node_val, &keys)
}

/// 提取形如 ["some"] 或 [123] 的key/index
///
/// 支持 `["field"]` 或 `[0]` 
fn extract_keys(s: &str) -> Result<Vec<String>, String> {
    // 匹配可能是: [ "abc" ] 或 [123]
    // => group1 => "abc", group2 => 123
    let re = Regex::new(r#"\[\s*(?:"([^"]+)"|(\d+))\s*\]"#).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for caps in re.captures_iter(s) {
        if let Some(obj_field) = caps.get(1) {
            // group1 => "abc"
            result.push(obj_field.as_str().to_string());
        } else if let Some(num_str) = caps.get(2) {
            // group2 => 123
            result.push(num_str.as_str().to_string());
        }
    }
    Ok(result)
}

/// 根据keys遍历
fn get_value_by_keys(mut current: &Value, keys: &[String]) -> Result<Value, String> {
    for k in keys {
        if let Ok(idx) = k.parse::<usize>() {
            match current {
                Value::Array(arr) => {
                    current = arr.get(idx)
                        .ok_or_else(|| format!("Index {} out of range for array", idx))?;
                },
                _ => {
                    return Err(format!("Current value is not an array, can't index {}", idx));
                }
            }
        } else {
            // 字段
            match current.get(k) {
                Some(v) => current = v,
                None => return Err(format!("Key '{}' not found in object", k)),
            }
        }
    }
    Ok(current.clone())
}

/// 把 Value 转成字符串
fn value_to_string(val: &Value) -> String {
    match val {
        Value::Null => "".to_string(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => val.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    type NodeResults = HashMap<String, Value>;

    // ---------- 1. 基础功能：$json ----------

    #[test]
    fn test_simple_json_field() {
        let input_data = json!({ "city": "London" });
        let results = NodeResults::new();
        let expr_str = r#"{{ $json["city"] }}"#;
        let val = evaluate_subexpr(expr_str.trim(), &input_data, &results).unwrap();
        assert_eq!(val, "London");
    }

    #[test]
    fn test_nested_json_field() {
        let input_data = json!({
            "location": {
                "country": "UK",
                "capital": { "name": "London" }
            }
        });
        let results = NodeResults::new();
        let expr_str = r#"{{ $json["location"]["capital"]["name"] }}"#;
        let val = evaluate_subexpr(expr_str.trim(), &input_data, &results).unwrap();
        assert_eq!(val, "London");
    }

    #[test]
    fn test_array_index() {
        let input_data = json!({ "arr": ["zero", "one", "two"] });
        let results = NodeResults::new();
        let expr_str = r#"{{ $json["arr"][1] }}"#; // 应返回 "one"
        let val = evaluate_subexpr(expr_str.trim(), &input_data, &results).unwrap();
        assert_eq!(val, "one");
    }

    #[test]
    fn test_array_index_nested() {
        let input_data = json!({
            "arr": [
                { "name": "Alice" },
                { "name": "Bob" },
                { "name": "Charlie" }
            ]
        });
        let results = NodeResults::new();
        let expr_str = r#"{{ $json["arr"][2]["name"] }}"#; // 应返回 "Charlie"
        let val = evaluate_subexpr(expr_str.trim(), &input_data, &results).unwrap();
        assert_eq!(val, "Charlie");
    }

    // ---------- 2. $node 访问 ----------

    #[test]
    fn test_node_field() {
        let mut results = NodeResults::new();
        results.insert("PrevNode".to_string(), json!({ "message": "Hello from PrevNode" }));
        let input_data = json!({});
        let expr_str = r#"{{ $node["PrevNode"].json["message"] }}"#;
        let val = evaluate_subexpr(expr_str.trim(), &input_data, &results).unwrap();
        assert_eq!(val, "Hello from PrevNode");
    }

    #[test]
    fn test_node_array_access() {
        let mut results = NodeResults::new();
        results.insert("DataNode".to_string(), json!({
            "arr": [{"val": 10}, {"val": 20}, {"val": 30}]
        }));
        let input_data = json!({});
        let expr_str = r#"{{ $node["DataNode"].json["arr"][1]["val"] }}"#; // 应返回 "20"
        let val = evaluate_subexpr(expr_str.trim(), &input_data, &results).unwrap();
        assert_eq!(val, "20");
    }

    // ---------- 3. 错误场景 ----------

    #[test]
    fn test_missing_key_error() {
        let input_data = json!({ "city": "Oslo" });
        let results = NodeResults::new();
        let expr_str = r#"{{ $json["country"] }}"#; // "country" 不存在
        let err = evaluate_subexpr(expr_str.trim(), &input_data, &results).unwrap_err();
        assert!(err.contains("not found in object"), "Expected error about missing key");
    }

    #[test]
    fn test_out_of_range_index() {
        let input_data = json!({ "numbers": [10, 20] });
        let results = NodeResults::new();
        let expr_str = r#"{{ $json["numbers"][5] }}"#; // 索引5越界
        let err = evaluate_subexpr(expr_str.trim(), &input_data, &results).unwrap_err();
        assert!(err.contains("out of range"), "Expected index out of range error");
    }

    #[test]
    fn test_not_array_but_index() {
        let input_data = json!({ "numbers": { "0": "not really an array" } });
        let results = NodeResults::new();
        let expr_str = r#"{{ $json["numbers"][0] }}"#;
        let err = evaluate_subexpr(expr_str.trim(), &input_data, &results).unwrap_err();
        assert!(err.contains("not an array"), "Expected error about not array");
    }

    // #[test]
    // fn test_invalid_expression_syntax() {
    //     let input_data = json!({});
    //     let results = NodeResults::new();
    //     // 缺少起始 "{{"
    //     let expr_str = r#"$json["something"]"#;
    //     let error1 = evaluate_subexpr(expr_str.trim(), &input_data, &results).unwrap_err();
    //     assert!(error1.contains("Unsupported expr syntax"), "Expected unsupported expr syntax");
    //     // 没有 .json[...] 部分
    //     let expr_str2 = r#"{{ $node["X"] }}"#;
    //     let err2 = evaluate_subexpr(expr_str2.trim(), &input_data, &results).unwrap_err();
    //     assert!(err2.contains("missing .json"), "Expected error about missing .json[...]");
    // }

    // ---------- 4. 测试 resolve_expressions 在对象中 ----------

    #[test]
    fn test_resolve_expressions_in_obj() {
        let input_data = json!({
            "city": "Berlin",
            "arr": ["A", "B", "C"]
        });
        let mut results = NodeResults::new();
        results.insert("X".to_string(), json!({ "foo": "bar" }));

        let params = json!({
            "title": "Hello, {{ $json[\"city\"] }} is cool",
            "index1": "{{ $json[\"arr\"][1] }}",
            "nodeData": "{{ $node[\"X\"].json[\"foo\"] }}",
            "nested": {
                "msg": "The city is {{ $json[\"city\"] }}"
            }
        });

        let resolved = resolve_expressions(&params, &input_data, &results);
        // "title" -> "Hello, Berlin is cool"
        assert_eq!(resolved["title"], json!("Hello, Berlin is cool"));
        // "index1" -> "B"
        assert_eq!(resolved["index1"], json!("B"));
        // "nodeData" -> "bar"
        assert_eq!(resolved["nodeData"], json!("bar"));
        // nested.msg -> "The city is Berlin"
        assert_eq!(resolved["nested"]["msg"], json!("The city is Berlin"));
    }
}
