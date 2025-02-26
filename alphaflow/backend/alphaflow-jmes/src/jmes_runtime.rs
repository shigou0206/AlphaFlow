// 在 src/jmes_runtime.rs 中

use crate::{
    functions::{ArgumentType, CustomFunction, Signature},
    Arcvar, Expression, Runtime, Variable,
};
use log::info;
use once_cell::sync::Lazy;
use serde_json::{json, Value};

pub static CUSTOM_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    let mut rt = Runtime::new();
    rt.register_builtin_functions();

    // 注册自定义函数 uppercase(s)
    rt.register_function(
        "uppercase",
        Box::new(CustomFunction::new(
            Signature::new(vec![ArgumentType::String], None),
            Box::new(|args: &[Arcvar], _ctx| {
                let s = args[0].as_string().cloned().unwrap_or_default();
                Ok(Arcvar::new(Variable::String(s.to_ascii_uppercase())))
            }),
        )),
    );

    // 注册自定义函数 split(delim, input)
    rt.register_function(
        "split",
        Box::new(CustomFunction::new(
            Signature::new(vec![ArgumentType::String, ArgumentType::String], None),
            Box::new(|args: &[Arcvar], _ctx| {
                let delim = args[0].as_string().cloned().unwrap_or_default();
                let input = args[1].as_string().cloned().unwrap_or_default();
                let splitted: Vec<Arcvar> = input
                    .split(&delim)
                    .map(|part| Arcvar::new(Variable::String(part.to_string())))
                    .collect();
                Ok(Arcvar::new(Variable::Array(splitted)))
            }),
        )),
    );

    // 注册自定义函数 concat (支持任意数量字符串)
    // 注册 concat 函数（支持任意类型参数，内部将参数转换为字符串）
    rt.register_function(
        "concat",
        Box::new(CustomFunction::new(
            // 这里要求至少一个参数为 Any，额外参数也都是 Any（而不是 String）
            Signature::new(vec![ArgumentType::Any], Some(ArgumentType::Any)),
            Box::new(|args: &[Arcvar], _ctx| {
                let concatenated = args
                    .iter()
                    .map(|arg| {
                        // 如果参数本身可以转换为字符串，就直接取
                        if let Some(s) = arg.as_string() {
                            s.clone()
                        } else {
                            // 否则尝试使用 serde_json 序列化，再去除两端的引号
                            let v = serde_json::to_string(&arg).unwrap_or_default();
                            if v.starts_with('\"') && v.ends_with('\"') && v.len() >= 2 {
                                v[1..v.len()-1].to_string()
                            } else {
                                v
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("");
                Ok(Arcvar::new(Variable::String(concatenated)))
            }),
        )),
    );

    rt.register_function(
        "gt",
        Box::new(CustomFunction::new(
            Signature::new(vec![ArgumentType::Any, ArgumentType::Any], Some(ArgumentType::Bool)),
            Box::new(|args: &[Arcvar], _ctx| {
                // 将参数转换为 JSON 值
                let left_val = serde_json::to_value(&args[0]).unwrap_or(json!(null));
                let right_val = serde_json::to_value(&args[1]).unwrap_or(json!(null));
    
                // 尝试将 left 转换为 f64：先检查是否为 number；如果不是则尝试解析字符串
                let left_num = left_val.as_f64().or_else(|| left_val.as_str().and_then(|s| s.parse::<f64>().ok()));
                // 同理转换右侧
                let right_num = right_val.as_f64().or_else(|| right_val.as_str().and_then(|s| s.parse::<f64>().ok()));
    
                let result = if let (Some(ln), Some(rn)) = (left_num, right_num) {
                    // 如果都能转换为数字，则进行数值比较
                    ln > rn
                } else if let (Some(ls), Some(rs)) = (left_val.as_str(), right_val.as_str()) {
                    // 如果转换失败但两者都是字符串，则按字典序比较
                    ls > rs
                } else {
                    false
                };
                Ok(Arcvar::new(Variable::Bool(result)))
            }),
        )),
    );

    info!("JMES runtime initialized with builtins + custom functions.");
    rt
});

#[derive(Debug, thiserror::Error)]
pub enum JmesMappingError {
    #[error("Compile error: {0}")]
    CompileError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub fn compile_and_search(expr_str: &str, input_data: &Value) -> Result<Value, JmesMappingError> {
    let expr = CUSTOM_RUNTIME
        .compile(expr_str)
        .map_err(|e| JmesMappingError::CompileError(e.to_string()))?;
    let result_var = expr
        .search(input_data)
        .map_err(|e| JmesMappingError::ExecutionError(e.to_string()))?;
    serde_json::to_value(&result_var)
        .map_err(|e| JmesMappingError::SerializationError(e.to_string()))
}

#[cfg(test)]
mod json_expr_tests {
    use super::*; // 引入 compile_and_search
    use serde_json::{json, Value};

    /// 组 1：测试对象内简单"或"运算（||）
    #[test]
    fn test_group_1_or_expressions() {
        // given 部分
        let given = json!({
            "outer": {
                "foo": "foo",
                "bar": "bar",
                "baz": "baz"
            }
        });

        // 测试用例列表：(表达式, 预期结果)
        let cases = vec![
            ("outer.foo || outer.bar", json!("foo")),
            ("outer.foo||outer.bar", json!("foo")),
            ("outer.bar || outer.baz", json!("bar")),
            ("outer.bar||outer.baz", json!("bar")),
            ("outer.bad || outer.foo", json!("foo")),
            ("outer.bad||outer.foo", json!("foo")),
            ("outer.foo || outer.bad", json!("foo")),
            ("outer.foo||outer.bad", json!("foo")),
            ("outer.bad || outer.alsobad", json!(null)),
            ("outer.bad||outer.alsobad", json!(null)),
        ];

        for (expr, expected) in cases {
            // 注意：如果默认的 JMESPath 运行时不支持 || 运算符，
            // 你可能需要在运行时注册对应的函数（如 or 函数）。
            let result = compile_and_search(expr, &given).unwrap_or(json!(null));
            assert_eq!(result, expected, "Expression: {}", expr);
        }
    }

    /// 组 2：测试包含空字符串、空数组、缺失字段的 "||" 运算
    #[test]
    fn test_group_2_or_with_empty_values() {
        let given = json!({
            "outer": {
                "foo": "foo",
                "bool": false,
                "empty_list": [],
                "empty_string": ""
            }
        });

        let cases = vec![
            ("outer.empty_string || outer.foo", json!("foo")),
            (
                "outer.nokey || outer.bool || outer.empty_list || outer.empty_string || outer.foo",
                json!("foo"),
            ),
        ];

        for (expr, expected) in cases {
            let result = compile_and_search(expr, &given).unwrap_or(json!(null));
            assert_eq!(result, expected, "Expression: {}", expr);
        }
    }

    /// 组 3：测试逻辑运算（&&, ||, !）和组合表达式
    #[test]
    fn test_group_3_logic_expressions() {
        let given = json!({
            "True": true,
            "False": false,
            "Number": 5,
            "EmptyList": [],
            "Zero": 0
        });

        let cases = vec![
            ("True && False", json!(false)),
            ("False && True", json!(false)),
            ("True && True", json!(true)),
            ("False && False", json!(false)),
            // 注意：对于 && 运算，我们期望按照 JMESPath 的逻辑返回最后一个真值或第一个假值，
            // 这取决于运行时具体实现，这里假设:
            ("True && Number", json!(5)),
            ("Number && True", json!(true)),
            ("Number && False", json!(false)),
            ("Number && EmptyList", json!([])),
            ("EmptyList && True", json!([])),
            ("EmptyList && False", json!([])),
            ("True || False", json!(true)),
            ("True || True", json!(true)),
            ("False || True", json!(true)),
            ("False || False", json!(false)),
            ("Number || EmptyList", json!(5)),
            ("Number || True", json!(5)),
            ("Number || True && False", json!(5)),
            ("(Number || True) && False", json!(false)),
            ("Number || (True && False)", json!(5)),
            ("!True", json!(false)),
            ("!False", json!(true)),
            ("!Number", json!(false)),
            ("!EmptyList", json!(true)),
            ("True && !False", json!(true)),
            ("True && !EmptyList", json!(true)),
            ("!False && !EmptyList", json!(true)),
            ("!(True && False)", json!(true)),
            ("!Zero", json!(false)),
            ("!!Zero", json!(true)),
        ];

        for (expr, expected) in cases {
            // 注意：逻辑运算符 &&, ||, ! 可能需要额外注册自定义函数（例如 and, or, not）
            let result = compile_and_search(expr, &given).unwrap_or(json!(null));
            assert_eq!(result, expected, "Expression: {}", expr);
        }
    }

    /// 组 4：测试比较运算（<, <=, ==, !=, >, >=）以及组合运算
    #[test]
    fn test_group_4_comparison_expressions() {
        let given = json!({
            "one": 1,
            "two": 2,
            "three": 3
        });

        let cases = vec![
            ("one < two", json!(true)),
            ("one <= two", json!(true)),
            ("one == one", json!(true)),
            ("one == two", json!(false)),
            ("one > two", json!(false)),
            ("one >= two", json!(false)),
            ("one != two", json!(true)),
            ("one < two && three > one", json!(true)),
            ("one < two || three > one", json!(true)),
            ("one < two || three < one", json!(true)),
            ("two < one || three < one", json!(false)),
        ];

        for (expr, expected) in cases {
            let result = compile_and_search(expr, &given).unwrap_or(json!(null));
            assert_eq!(result, expected, "Expression: {}", expr);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    /// 测试：当上下文中 "i" 为数字时，gt(@.i, 5) 应返回 true

    /// 测试：当上下文中 "i" 为字符串 "10" 时，gt(@.i, 5) 应尝试转换为数字返回 true
    #[test]
    fn test_gt_function_with_string_number() {
        let ctx = json!({ "i": 10 });
        let expr = "gt(@.i, '5')";
        let result = compile_and_search(expr, &ctx)
            .expect("gt expression should evaluate successfully");
        assert_eq!(result, json!(true), "Expected '10' > 5 to be true after conversion");
    }
}