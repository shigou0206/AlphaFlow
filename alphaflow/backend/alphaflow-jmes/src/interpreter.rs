// //! Interprets JMESPath expressions.

// use std::collections::BTreeMap;

// use super::ast::Ast;
// use super::variable::Variable;
// use super::Context;
// // 将 `Rcvar` 换成 `Arcvar`
// use super::{Arcvar, ErrorReason, JmespathError, RuntimeError};

// /// Result of searching data using a JMESPath Expression.
// ///
// /// 这里由 `Result<Rcvar, JmespathError>` 改为 `Result<Arcvar, JmespathError>`.
// pub type SearchResult = Result<Arcvar, JmespathError>;

// /// Interprets the given data using an AST node.
// ///
// /// 这里将函数签名的 `data: &Rcvar` 改为 `data: &Arcvar`.
// pub fn interpret(data: &Arcvar, node: &Ast, ctx: &mut Context<'_>) -> SearchResult {
//     match *node {
//         Ast::Field { ref name, .. } => {
//             // 假设 data.get_field(name) 返回 Arcvar
//             Ok(data.get_field(name))
//         }
//         Ast::Subexpr {
//             ref lhs, ref rhs, ..
//         } => {
//             let left_result = interpret(data, lhs, ctx)?;
//             interpret(&left_result, rhs, ctx)
//         }
//         Ast::Identity { .. } => Ok(data.clone()),
//         Ast::Literal { ref value, .. } => Ok(value.clone()),
//         Ast::Index { idx, .. } => {
//             if idx >= 0 {
//                 Ok(data.get_index(idx as usize))
//             } else {
//                 Ok(data.get_negative_index((-idx) as usize))
//             }
//         }
//         Ast::Or {
//             ref lhs, ref rhs, ..
//         } => {
//             let left = interpret(data, lhs, ctx)?;
//             if left.is_truthy() {
//                 Ok(left)
//             } else {
//                 interpret(data, rhs, ctx)
//             }
//         }
//         Ast::And {
//             ref lhs, ref rhs, ..
//         } => {
//             let left = interpret(data, lhs, ctx)?;
//             if !left.is_truthy() {
//                 Ok(left)
//             } else {
//                 interpret(data, rhs, ctx)
//             }
//         }
//         Ast::Not { ref node, .. } => {
//             let result = interpret(data, node, ctx)?;
//             Ok(Arcvar::new(Variable::Bool(!result.is_truthy())))
//         }
//         // Returns the result of RHS if cond yields truthy value.
//         Ast::Condition {
//             ref predicate,
//             ref then,
//             ..
//         } => {
//             let cond_result = interpret(data, predicate, ctx)?;
//             if cond_result.is_truthy() {
//                 interpret(data, then, ctx)
//             } else {
//                 Ok(Arcvar::new(Variable::Null))
//             }
//         }
//         Ast::Comparison {
//             ref comparator,
//             ref lhs,
//             ref rhs,
//             ..
//         } => {
//             let left = interpret(data, lhs, ctx)?;
//             let right = interpret(data, rhs, ctx)?;
//             Ok(left
//                 .compare(comparator, &right)
//                 .map_or_else(
//                     || Arcvar::new(Variable::Null),
//                     |result| Arcvar::new(Variable::Bool(result))
//                 ))
//         }
//         // Converts an object into a JSON array of its values.
//         Ast::ObjectValues { ref node, .. } => {
//             let subject = interpret(data, node, ctx)?;
//             match *subject {
//                 Variable::Object(ref v) => Ok(Arcvar::new(Variable::Array(
//                     v.values().cloned().collect::<Vec<Arcvar>>(),
//                 ))),
//                 _ => Ok(Arcvar::new(Variable::Null)),
//             }
//         }
//         // Passes the results of lhs into rhs if lhs yields an array and
//         // each node of lhs that passes through rhs yields a non-null value.
//         Ast::Projection {
//             ref lhs, ref rhs, ..
//         } => match interpret(data, lhs, ctx)?.as_array() {
//             None => Ok(Arcvar::new(Variable::Null)),
//             Some(left) => {
//                 let mut collected = vec![];
//                 for element in left {
//                     let current = interpret(element, rhs, ctx)?;
//                     if !current.is_null() {
//                         collected.push(current);
//                     }
//                 }
//                 Ok(Arcvar::new(Variable::Array(collected)))
//             }
//         },
//         Ast::Flatten { ref node, .. } => match interpret(data, node, ctx)?.as_array() {
//             None => Ok(Arcvar::new(Variable::Null)),
//             Some(a) => {
//                 let mut collected: Vec<Arcvar> = vec![];
//                 for element in a {
//                     match element.as_array() {
//                         Some(array) => collected.extend(array.iter().cloned()),
//                         _ => collected.push(element.clone()),
//                     }
//                 }
//                 Ok(Arcvar::new(Variable::Array(collected)))
//             }
//         },
//         Ast::MultiList { ref elements, .. } => {
//             if data.is_null() {
//                 Ok(Arcvar::new(Variable::Null))
//             } else {
//                 let mut collected = vec![];
//                 for node in elements {
//                     collected.push(interpret(data, node, ctx)?);
//                 }
//                 Ok(Arcvar::new(Variable::Array(collected)))
//             }
//         }
//         Ast::MultiHash { ref elements, .. } => {
//             if data.is_null() {
//                 Ok(Arcvar::new(Variable::Null))
//             } else {
//                 let mut collected = BTreeMap::new();
//                 for kvp in elements {
//                     let value = interpret(data, &kvp.value, ctx)?;
//                     collected.insert(kvp.key.clone(), value);
//                 }
//                 Ok(Arcvar::new(Variable::Object(collected)))
//             }
//         }
//         Ast::Function {
//             ref name,
//             ref args,
//             offset,
//         } => {
//             let mut fn_args: Vec<Arcvar> = vec![];
//             for arg in args {
//                 fn_args.push(interpret(data, arg, ctx)?);
//             }
//             // Reset the offset so that it points to the function being evaluated.
//             ctx.offset = offset;
//             match ctx.runtime.get_function(name) {
//                 Some(f) => f.evaluate(&fn_args, ctx),
//                 None => {
//                     let reason =
//                         ErrorReason::Runtime(RuntimeError::UnknownFunction(name.to_owned()));
//                     Err(JmespathError::from_ctx(ctx, reason))
//                 }
//             }
//         }
//         Ast::Expref { ref ast, .. } => Ok(Arcvar::new(Variable::Expref(ast.clone()))),
//         Ast::Slice {
//             start,
//             stop,
//             step,
//             offset,
//         } => {
//             if step == 0 {
//                 ctx.offset = offset;
//                 let reason = ErrorReason::Runtime(RuntimeError::InvalidSlice);
//                 Err(JmespathError::from_ctx(ctx, reason))
//             } else {
//                 match data.slice(start, stop, step) {
//                     Some(array) => Ok(Arcvar::new(Variable::Array(array))),
//                     None => Ok(Arcvar::new(Variable::Null)),
//                 }
//             }
//         }
//     }
// }

//! Interprets JMESPath expressions.

use std::collections::BTreeMap;

use super::ast::Ast;
use super::variable::Variable;
use super::Context;
use super::{Arcvar, ErrorReason, JmespathError, RuntimeError};

// 引入log宏
use log::{debug, trace};

/// Result of searching data using a JMESPath Expression.
pub type SearchResult = Result<Arcvar, JmespathError>;

/// Interprets the given data using an AST node.
pub fn interpret(data: &Arcvar, node: &Ast, ctx: &mut Context<'_>) -> SearchResult {
    // 进入时先 trace 显示节点类型和关键数据
    trace!("interpret: node={:?}, data_type={:?}", node, data.get_type());

    // 执行匹配分支并得到 Result
    let result: SearchResult = match *node {
        Ast::Field { ref name, .. } => {
            trace!("  -> Ast::Field: name={:?}", name);
            Ok(data.get_field(name))
        }
        Ast::Subexpr { ref lhs, ref rhs, .. } => {
            let left_result = interpret(data, lhs, ctx)?;
            interpret(&left_result, rhs, ctx)
        }
        Ast::Identity { .. } => {
            trace!("  -> Ast::Identity => return data clone");
            Ok(data.clone())
        }
        Ast::Literal { ref value, .. } => {
            trace!("  -> Ast::Literal => value={:?}", value);
            Ok(value.clone())
        }
        Ast::Index { idx, .. } => {
            trace!("  -> Ast::Index => idx={}", idx);
            if idx >= 0 {
                Ok(data.get_index(idx as usize))
            } else {
                Ok(data.get_negative_index((-idx) as usize))
            }
        }
        Ast::Or { ref lhs, ref rhs, .. } => {
            let left = interpret(data, lhs, ctx)?;
            if left.is_truthy() {
                Ok(left)
            } else {
                interpret(data, rhs, ctx)
            }
        }
        Ast::And { ref lhs, ref rhs, .. } => {
            let left = interpret(data, lhs, ctx)?;
            if !left.is_truthy() {
                Ok(left)
            } else {
                interpret(data, rhs, ctx)
            }
        }
        Ast::Not { ref node, .. } => {
            let result = interpret(data, node, ctx)?;
            Ok(Arcvar::new(Variable::Bool(!result.is_truthy())))
        }
        Ast::Condition { ref predicate, ref then, .. } => {
            let cond_result = interpret(data, predicate, ctx)?;
            if cond_result.is_truthy() {
                interpret(data, then, ctx)
            } else {
                Ok(Arcvar::new(Variable::Null))
            }
        }
        Ast::Comparison { ref comparator, ref lhs, ref rhs, .. } => {
            let left = interpret(data, lhs, ctx)?;
            let right = interpret(data, rhs, ctx)?;
            Ok(left.compare(comparator, &right).map_or_else(
                || Arcvar::new(Variable::Null),
                |result_bool| Arcvar::new(Variable::Bool(result_bool))
            ))
        }
        Ast::ObjectValues { ref node, .. } => {
            let subject = interpret(data, node, ctx)?;
            match *subject {
                Variable::Object(ref obj_map) => {
                    Ok(Arcvar::new(Variable::Array(obj_map.values().cloned().collect())))
                }
                _ => Ok(Arcvar::new(Variable::Null)),
            }
        }
        Ast::Projection { ref lhs, ref rhs, .. } => {
            match interpret(data, lhs, ctx)?.as_array() {
                None => Ok(Arcvar::new(Variable::Null)),
                Some(left_arr) => {
                    let mut collected = vec![];
                    for element in left_arr {
                        let current = interpret(element, rhs, ctx)?;
                        if !current.is_null() {
                            collected.push(current);
                        }
                    }
                    Ok(Arcvar::new(Variable::Array(collected)))
                }
            }
        }
        Ast::Flatten { ref node, .. } => {
            match interpret(data, node, ctx)?.as_array() {
                None => Ok(Arcvar::new(Variable::Null)),
                Some(a) => {
                    let mut collected: Vec<Arcvar> = vec![];
                    for element in a {
                        match element.as_array() {
                            Some(subarray) => collected.extend(subarray.iter().cloned()),
                            _ => collected.push(element.clone()),
                        }
                    }
                    Ok(Arcvar::new(Variable::Array(collected)))
                }
            }
        }
        Ast::MultiList { ref elements, .. } => {
            if data.is_null() {
                Ok(Arcvar::new(Variable::Null))
            } else {
                let mut collected = vec![];
                for subnode in elements {
                    collected.push(interpret(data, subnode, ctx)?);
                }
                Ok(Arcvar::new(Variable::Array(collected)))
            }
        }
        Ast::MultiHash { ref elements, .. } => {
            if data.is_null() {
                Ok(Arcvar::new(Variable::Null))
            } else {
                let mut collected = BTreeMap::new();
                for kvp in elements {
                    let val = interpret(data, &kvp.value, ctx)?;
                    collected.insert(kvp.key.clone(), val);
                }
                Ok(Arcvar::new(Variable::Object(collected)))
            }
        }
        Ast::Function { ref name, ref args, offset } => {
            let mut fn_args: Vec<Arcvar> = vec![];
            for arg_ast in args {
                fn_args.push(interpret(data, arg_ast, ctx)?);
            }
            ctx.offset = offset;
            match ctx.runtime.get_function(name) {
                Some(f) => f.evaluate(&fn_args, ctx),
                None => {
                    let reason = ErrorReason::Runtime(RuntimeError::UnknownFunction(name.to_owned()));
                    Err(JmespathError::from_ctx(ctx, reason))
                }
            }
        }
        Ast::Expref { ref ast, .. } => {
            Ok(Arcvar::new(Variable::Expref(ast.clone())))
        }
        Ast::Slice { start, stop, step, offset } => {
            if step == 0 {
                ctx.offset = offset;
                let reason = ErrorReason::Runtime(RuntimeError::InvalidSlice);
                Err(JmespathError::from_ctx(ctx, reason))
            } else {
                match data.slice(start, stop, step) {
                    Some(array) => Ok(Arcvar::new(Variable::Array(array))),
                    None => Ok(Arcvar::new(Variable::Null)),
                }
            }
        }
    };

    // 结果日志
    trace!("interpret: node={:?} => result={:?}", node, result);

    result
}