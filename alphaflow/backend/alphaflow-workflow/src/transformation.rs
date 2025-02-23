use serde_json::{json, Map, Value};
use std::error::Error;
use alphaflow_jmes;
/// 转换配置，类似于 Step Functions 的 InputPath、Parameters、ResultPath、OutputPath。
#[derive(Debug, Clone)]
pub struct TransformationConfig {
    /// 从输入数据中提取数据的 JMESPath 表达式。例如："body" 或 "body.items[0]"
    pub input_path: Option<String>,
    /// 用于重构数据的参数。可以是一个 JSON 对象，会和 input 数据进行浅合并。
    pub parameters: Option<Value>,
    /// 指定将转换结果嵌入原始输入中的路径，使用点分隔的字符串。例如："body.transformed"
    pub result_path: Option<String>,
    /// 从最终数据中提取需要输出的部分，例如 "body.transformed" 或 "transformed"（相对于 result_path）
    pub output_path: Option<String>,
}

/// 使用 jmespath 执行一个查询，并返回查询结果
fn query_json(input: &Value, query: &str) -> Result<Value, Box<dyn Error>> {
    let expr = alphaflow_jmes::compile(query)?;
    let result = expr.search(input)?;
    // 这里直接将结果转换为 serde_json::Value
    let json_value = serde_json::to_value(&*result)?;
    Ok(json_value)
}

/// 简单合并两个 JSON 对象（浅合并）：对于相同 key，参数 config 覆盖 input 的值。
fn merge_json_objects(mut base: Map<String, Value>, overlay: &Map<String, Value>) -> Map<String, Value> {
    for (k, v) in overlay {
        base.insert(k.clone(), v.clone());
    }
    base
}

/// 将一个 JSON 对象按照点号分割路径插入一个值（覆盖或嵌入）。
/// 例如，set_nested_value(&mut obj, "body.transformed", new_value) 会在 obj["body"] 内插入或更新 transformed 字段。
fn set_nested_value(obj: &mut Value, path: &str, new_value: Value) {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return;
    }
    let mut current = obj;
    for (i, key) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let Value::Object(map) = current {
                map.insert(key.to_string(), new_value.clone());
            }
        } else {
            if let Value::Object(map) = current {
                if !map.contains_key(*key) {
                    map.insert(key.to_string(), json!({}));
                }
                current = map.get_mut(*key).unwrap();
            } else {
                // 如果当前节点不是对象则中断
                break;
            }
        }
    }
}

/// 进行转换：
/// 1. 如果配置了 input_path，则使用它过滤输入数据；
/// 2. 如果配置了 parameters，则和过滤后的数据进行浅合并；
/// 3. 如果配置了 result_path，则将转换结果嵌入到原始输入中；
/// 4. 如果配置了 output_path，则对最终数据进行过滤，仅返回指定部分。
pub fn transform_data(input: &Value, config: &TransformationConfig) -> Result<Value, Box<dyn Error>> {
    // 1. InputPath：过滤输入数据
    let mut transformed = if let Some(ref input_path) = config.input_path {
        query_json(input, input_path)?
    } else {
        input.clone()
    };

    // 2. Parameters：重构数据，浅合并 parameters 对象
    if let Some(ref params) = config.parameters {
        if let Some(overlay) = params.as_object() {
            if let Some(base_obj) = transformed.as_object_mut() {
                let merged = merge_json_objects(base_obj.clone(), overlay);
                transformed = Value::Object(merged);
            }
        }
    }

    // 3. ResultPath：将转换结果嵌入原始输入数据中
    let mut final_result = input.clone();
    if let Some(ref result_path) = config.result_path {
        set_nested_value(&mut final_result, result_path, transformed.clone());
    } else {
        final_result = transformed.clone();
    }

    // 4. OutputPath：对最终结果进行过滤
    if let Some(ref output_path) = config.output_path {
        // 如果 result_path 存在且 output_path 没有点号，
        // 则认为希望直接返回转换结果
        if config.result_path.is_some() && !output_path.contains('.') {
            final_result = transformed.clone();
        } else {
            final_result = query_json(&final_result, output_path)?;
        }
    }
    Ok(final_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // 测试仅使用 InputPath，从输入中提取 "body" 部分
    #[test]
    fn test_input_path_only() {
        let input = json!({
            "body": {
                "city": "New York",
                "temperature": 28
            },
            "user": { "name": "Alice" }
        });
        let config = TransformationConfig {
            input_path: Some("body".to_string()),
            parameters: None,
            result_path: None,
            output_path: None,
        };

        let result = transform_data(&input, &config).unwrap();
        let expected = json!({
            "city": "New York",
            "temperature": 28
        });
        assert_eq!(result, expected);
    }

    // 测试使用 Parameters 合并操作：parameters 中的字段覆盖 input 中相同字段，并添加新字段
    #[test]
    fn test_parameters_merge() {
        let input = json!({
            "body": {
                "city": "Los Angeles",
                "temperature": 26
            }
        });
        let config = TransformationConfig {
            input_path: Some("body".to_string()),
            parameters: Some(json!({
                "info": "is a great city",
                "temperature": 30  // 覆盖原 temperature
            })),
            result_path: None,
            output_path: None,
        };

        let result = transform_data(&input, &config).unwrap();
        let expected = json!({
            "city": "Los Angeles",
            "temperature": 30,
            "info": "is a great city"
        });
        assert_eq!(result, expected);
    }

    // 测试 ResultPath：将转换结果嵌入原始输入中指定的位置
    #[test]
    fn test_result_path() {
        let input = json!({
            "body": {
                "city": "Chicago",
                "temperature": 20
            }
        });
        let config = TransformationConfig {
            input_path: Some("body".to_string()),
            parameters: Some(json!({ "note": "cool city" })),
            result_path: Some("body.transformed".to_string()),
            output_path: None,
        };

        let result = transform_data(&input, &config).unwrap();
        let expected = json!({
            "body": {
                "city": "Chicago",
                "temperature": 20,
                "transformed": {
                    "city": "Chicago",
                    "temperature": 20,
                    "note": "cool city"
                }
            }
        });
        assert_eq!(result, expected);
    }

    // 测试 OutputPath：仅返回经过输出过滤后的数据
    #[test]
    fn test_output_path() {
        let input = json!({
            "body": {
                "city": "Boston",
                "temperature": 15,
                "extra": "data"
            }
        });
        let config = TransformationConfig {
            input_path: Some("body".to_string()),
            parameters: None,
            result_path: None,
            output_path: Some("city".to_string()),
        };

        let result = transform_data(&input, &config).unwrap();
        // 输出应仅为 "Boston"
        assert_eq!(result, json!("Boston"));
    }

    // 测试结合所有转换功能：先过滤，再重构，嵌入原始输入，最后提取输出
    #[test]
    fn test_full_transformation() {
        let input = json!({
            "body": {
                "city": "Seattle",
                "temperature": 12
            },
            "meta": { "timestamp": "2023-01-01T12:00:00Z" }
        });
        let config = TransformationConfig {
            input_path: Some("body".to_string()),
            parameters: Some(json!({ "info": "rainy city" })),
            result_path: Some("body.transformed".to_string()),
            // 输出路径指定为 "body.transformed"，这样会提取嵌入后的数据
            output_path: Some("body.transformed".to_string()),
        };

        let result = transform_data(&input, &config).unwrap();
        let expected = json!({
            "city": "Seattle",
            "temperature": 12,
            "info": "rainy city"
        });
        assert_eq!(result, expected);
    }
}


// #[cfg(test)]
// mod tests {
//     use super::*;
//     use serde_json::json;

//     #[test]
//     fn test_input_path_only() {
//         let input = json!({
//             "body": {
//                 "city": "New York",
//                 "temperature": 28
//             },
//             "user": { "name": "Alice" }
//         });
//         let config = TransformationConfig {
//             input_path: Some("body".to_string()),
//             parameters: None,
//             result_path: None,
//             output_path: None,
//         };

//         let result = transform_data(&input, &config).unwrap();
//         let expected = json!({
//             "city": "New York",
//             "temperature": 28
//         });
//         assert_eq!(result, expected);
//     }

//     #[test]
//     fn test_parameters_merge() {
//         let input = json!({
//             "body": {
//                 "city": "Los Angeles",
//                 "temperature": 26
//             }
//         });
//         let config = TransformationConfig {
//             input_path: Some("body".to_string()),
//             parameters: Some(json!({
//                 "info": "is a great city",
//                 "temperature": 30  // 覆盖原 temperature
//             })),
//             result_path: None,
//             output_path: None,
//         };

//         let result = transform_data(&input, &config).unwrap();
//         let expected = json!({
//             "city": "Los Angeles",
//             "temperature": 30,
//             "info": "is a great city"
//         });
//         assert_eq!(result, expected);
//     }

//     #[test]
//     fn test_result_path() {
//         let input = json!({
//             "body": {
//                 "city": "Chicago",
//                 "temperature": 20
//             }
//         });
//         let config = TransformationConfig {
//             input_path: Some("body".to_string()),
//             parameters: Some(json!({ "note": "cool city" })),
//             result_path: Some("body.transformed".to_string()),
//             output_path: None,
//         };

//         let result = transform_data(&input, &config).unwrap();
//         let expected = json!({
//             "body": {
//                 "city": "Chicago",
//                 "temperature": 20,
//                 "transformed": {
//                     "city": "Chicago",
//                     "temperature": 20,
//                     "note": "cool city"
//                 }
//             }
//         });
//         assert_eq!(result, expected);
//     }

//     #[test]
//     fn test_output_path() {
//         let input = json!({
//             "body": {
//                 "city": "Boston",
//                 "temperature": 15,
//                 "extra": "data"
//             }
//         });
//         let config = TransformationConfig {
//             input_path: Some("body".to_string()),
//             parameters: None,
//             result_path: None,
//             output_path: Some("city".to_string()),
//         };

//         let result = transform_data(&input, &config).unwrap();
//         assert_eq!(result, json!("Boston"));
//     }

//     #[test]
//     fn test_full_transformation() {
//         // 结合所有转换功能：先过滤 body，再重构，嵌入原始输入，最后提取部分结果
//         let input = json!({
//             "body": {
//                 "city": "Seattle",
//                 "temperature": 12
//             },
//             "meta": { "timestamp": "2023-01-01T12:00:00Z" }
//         });
//         let config = TransformationConfig {
//             input_path: Some("body".to_string()),
//             parameters: Some(json!({ "info": "rainy city" })),
//             result_path: Some("body.transformed".to_string()),
//             // 修改 output_path 为 "body.transformed" 或直接不包含点号（这时返回转换结果）
//             output_path: Some("transformed".to_string()),
//         };

//         let result = transform_data(&input, &config).unwrap();
//         let expected = json!({
//             "city": "Seattle",
//             "temperature": 12,
//             "info": "rainy city"
//         });
//         assert_eq!(result, expected);
//     }
// }