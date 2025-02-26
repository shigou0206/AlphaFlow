// src/expression.rs

use serde_json::{json, Value};
use crate::jmes_runtime::compile_and_search;
use log::info;

/// 对映射模板进行处理，将模板中形如 "@.xxx" 的字符串替换为上下文中的值。
/// 该函数会递归处理模板中的对象和数组。
pub fn apply_template(mapping_template: &Value, ctx: &Value) -> Result<Value, String> {
    match mapping_template {
        // 如果是字符串，检查是否包含动态表达式标志，比如 '(' 或 '@'
        Value::String(s) => {
            if s.contains('(') || s.contains('@') {
                // 尝试编译求值
                compile_and_search(s, ctx).map_err(|e| format!("Mapping failed: {:?}", e))
            } else {
                // 否则认为是静态文本
                Ok(Value::String(s.clone()))
            }
        },
        // 如果是对象，则递归处理每个字段
        Value::Object(map) => {
            let mut result_map = serde_json::Map::new();
            for (key, value) in map {
                let new_value = apply_template(value, ctx)?;
                result_map.insert(key.clone(), new_value);
            }
            Ok(Value::Object(result_map))
        },
        // 如果是数组，则递归处理每个元素
        Value::Array(arr) => {
            let mut result_arr = Vec::new();
            for v in arr {
                result_arr.push(apply_template(v, ctx)?);
            }
            Ok(Value::Array(result_arr))
        },
        // 其他类型直接返回原值
        _ => Ok(mapping_template.clone()),
    }
}

/// 示例函数：给定映射模板和上游数据，返回最终的输入数据
///
/// - `merged_input`：上游数据，例如 { "c": 2 }
/// - `parameters`：动态参数（可选，作为附加上下文）
/// - `mapping_template`：前端填写的映射模板，例如 { "a": 12, "b": "@.c" }
///
/// 该函数将构造一个上下文，将 merged_input 和 parameters 扁平合并到一起，然后用 apply_template 处理模板。
pub fn apply_mapping_template(
    merged_input: Value,
    parameters: &Value,
    mapping_template: &Value,
) -> Result<Value, String> {
    // 构造上下文：将 merged_input 与 parameters 合并（扁平合并）
    let mut ctx = merged_input;
    if let Some(params) = parameters.as_object() {
        for (key, value) in params {
            ctx[key] = value.clone();
        }
    }
    info!("Constructed context: {:?}", ctx);
    // 处理模板
    apply_template(mapping_template, &ctx)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use serde_json::json;

//     /// 测试简单映射：模板中含有静态值和表达式占位符
//     ///
//     /// 给定上下文 { "c": 2 }，模板为 { "a": 12, "b": "@.c" }，
//     /// 最终期望得到 { "a": 12, "b": 2 }。
//     #[test]
//     fn test_apply_template_simple() {
//         let merged_input = json!({ "c": 2 });
//         let parameters = json!({}); // 无额外参数
//         let mapping_template = json!({
//             "a": 12,
//             "b": "@.c"
//         });
//         let result = apply_mapping_template(merged_input, &parameters, &mapping_template)
//             .expect("Mapping should succeed");
//         let expected = json!({
//             "a": 12,
//             "b": 2
//         });
//         assert_eq!(result, expected);
//     }

//     /// 测试嵌套模板：模板中嵌套对象和数组均支持表达式替换
//     #[test]
//     fn test_apply_template_nested() {
//         let merged_input = json!({
//             "c": 2,
//             "d": "hello"
//         });
//         let parameters = json!({
//             "prefix": ">>"
//         });
//         let mapping_template = json!({
//             "a": 12,
//             "nested": {
//                 "b": "@.c",
//                 "msg": "concat(prefix, @.d)"
//             },
//             "list": [
//                 "@.c",
//                 "static",
//                 "@.d"
//             ]
//         });
//         // 假设你的运行时已经注册了 concat 函数
//         let result = apply_mapping_template(merged_input, &parameters, &mapping_template)
//             .expect("Mapping should succeed");
//         // 预期： "b" 替换为 2, "msg" 使用 concat 函数将 ">>" 和 "hello" 拼接成 ">>hello"
//         let expected = json!({
//             "a": 12,
//             "nested": {
//                 "b": 2,
//                 "msg": ">>hello"
//             },
//             "list": [
//                 2,
//                 "static",
//                 "hello"
//             ]
//         });
//         assert_eq!(result, expected);
//     }

//     /// 测试参数注入：模板表达式中可直接引用 parameters 中的变量
//     #[test]
//     fn test_apply_template_with_parameters() {
//         let merged_input = json!({ "c": 2 });
//         let parameters = json!({
//             "p": 100
//         });
//         let mapping_template = json!({
//             "a": 12,
//             "b": "concat(@.c, '-', p)"  // 假设 concat 函数已注册，可将数字和参数转换为字符串拼接
//         });
//         let result = apply_mapping_template(merged_input, &parameters, &mapping_template)
//             .expect("Mapping should succeed");
//         // 假设转换后 b 的值应为 "2-100"
//         let expected = json!({
//             "a": 12,
//             "b": "2-100"
//         });
//         assert_eq!(result, expected);
//     }

//     /// 测试如果模板中没有表达式，则直接返回原值
//     #[test]
//     fn test_apply_template_no_expression() {
//         let merged_input = json!({ "c": 2 });
//         let parameters = json!({});
//         let mapping_template = json!({
//             "a": 12,
//             "b": "hello"
//         });
//         let result = apply_mapping_template(merged_input, &parameters, &mapping_template)
//             .expect("Mapping should succeed");
//         let expected = json!({
//             "a": 12,
//             "b": "hello"
//         });
//         assert_eq!(result, expected);
//     }

    
// }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::collections::BTreeMap;

    /// 测试简单映射：模板中含有静态值和动态表达式
    /// 给定上下文 { "c": 2 }，模板为 { "a": 12, "b": "@.c" }，预期输出 { "a": 12, "b": 2 }。
    #[test]
    fn test_apply_template_simple() {
        let merged_input = json!({ "c": 2 });
        let parameters = json!({}); // 无额外参数
        let mapping_template = json!({
            "a": 12,
            "b": "@.c"
        });
        let result = apply_mapping_template(merged_input, &parameters, &mapping_template)
            .expect("Mapping should succeed");
        let expected = json!({
            "a": 12,
            "b": 2
        });
        assert_eq!(result, expected);
    }

    /// 测试嵌套模板：模板中嵌套对象和数组均包含动态表达式
    /// 上下文包含 { "c": 2, "d": "hello" }，动态参数中包含 { "prefix": ">>" }，
    /// 模板为：
    /// {
    ///   "a": 12,
    ///   "nested": { "b": "@.c", "msg": "concat(prefix, @.d)" },
    ///   "list": [ "@.c", "static", "@.d" ]
    /// }
    /// 预期输出为：
    /// {
    ///   "a": 12,
    ///   "nested": { "b": 2, "msg": ">>hello" },
    ///   "list": [ 2, "static", "hello" ]
    /// }
    #[test]
    fn test_apply_template_nested() {
        let merged_input = json!({
            "c": 2,
            "d": "hello"
        });
        let parameters = json!({
            "prefix": ">>"
        });
        let mapping_template = json!({
            "a": 12,
            "nested": {
                "b": "@.c",
                "msg": "concat(prefix, @.d)"
            },
            "list": [
                "@.c",
                "static",
                "@.d"
            ]
        });
        let result = apply_mapping_template(merged_input, &parameters, &mapping_template)
            .expect("Mapping should succeed");
        let expected = json!({
            "a": 12,
            "nested": {
                "b": 2,
                "msg": ">>hello"
            },
            "list": [
                2,
                "static",
                "hello"
            ]
        });
        assert_eq!(result, expected);
    }

    /// 测试参数注入：模板表达式中可以引用 parameters 中的变量
    /// 上下文：{ "c": 2 }，参数：{ "p": 100 }，
    /// 模板：{ "a": 12, "b": "concat(@.c, '-', p)" }
    /// 预期输出：{ "a": 12, "b": "2-100" }
    #[test]
    fn test_apply_template_with_parameters() {
        let merged_input = json!({ "c": 2 });
        let parameters = json!({
            "p": 100
        });
        let mapping_template = json!({
            "a": 12,
            "b": "concat(@.c, '-', p)"
        });
        let result = apply_mapping_template(merged_input, &parameters, &mapping_template)
            .expect("Mapping should succeed");
        let expected = json!({
            "a": 12,
            "b": "2-100"
        });
        assert_eq!(result, expected);
    }

    /// 测试当模板中没有表达式时，直接返回原值
    #[test]
    fn test_apply_template_no_expression() {
        let merged_input = json!({ "c": 2 });
        let parameters = json!({});
        let mapping_template = json!({
            "a": 12,
            "b": "hello"
        });
        let result = apply_mapping_template(merged_input, &parameters, &mapping_template)
            .expect("Mapping should succeed");
        let expected = json!({
            "a": 12,
            "b": "hello"
        });
        assert_eq!(result, expected);
    }

    /// 测试多字段映射（Multi 格式）的逻辑：
    /// 这里我们直接构造一个映射模板，其中字段值都是表达式字符串，
    /// 例如 "first": "@.c", "second": "concat(prefix, @.d)", "third": "@.e"
    #[test]
    fn test_apply_template_multi_fields() {
        let merged_input = json!({
            "c": 2,
            "d": "world",
            "e": 42
        });
        let parameters = json!({
            "prefix": "Hello, "
        });
        // 此处直接将映射模板作为静态 JSON 使用，字段值为动态表达式
        let mapping_template = json!({
            "first": "@.c",
            "second": "concat(prefix, @.d)",
            "third": "@.e"
        });
        let result = apply_mapping_template(merged_input, &parameters, &mapping_template)
            .expect("Mapping should succeed");
        let expected = json!({
            "first": 2,
            "second": "Hello, world",
            "third": 42
        });
        assert_eq!(result, expected);
    }

    /// 测试缺失字段的情况：如果上下文中不存在某个表达式引用的字段，映射结果应返回 null
    #[test]
    fn test_apply_template_with_missing_field() {
        let merged_input = json!({
            "c": 2
        });
        let parameters = json!({});
        // 模板中引用了不存在的字段 "@.d"
        let mapping_template = json!({
            "a": 12,
            "b": "concat('Value is: ', @.d)"
        });
        let result = apply_mapping_template(merged_input, &parameters, &mapping_template)
            .expect("Mapping should succeed");
        // 假设当字段缺失时，表达式返回 null，通过 concat 处理后可能返回 "Value is: null"
        let expected = json!({
            "a": 12,
            "b": "Value is: null"
        });
        assert_eq!(result, expected);
    }
}