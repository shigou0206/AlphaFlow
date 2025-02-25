// // src/executor.rs

// use std::collections::{HashMap, VecDeque};
// use serde_json::{json, Value};
// use log::{info, warn, error};
// use crate::workflow::Workflow;
// use alphaflow_nodes::node::Node;
// use crate::jmes_runtime::{compile_and_search, JmesMappingError};
// use alphaflow_nodes::node_type::{NodeExecutionContext, build_context};
// use crate::waiting_queue::WaitingQueue;


// /// 每个待执行项：节点名称及其输入数据 (JSON)
// #[derive(Debug)]
// pub struct ExecutionItem {
//     pub node_name: String,
//     pub input_data: Value,
// }

// /// 执行上下文：包含执行栈、节点最终结果和一个等待队列（用于多父节点合并）
// #[derive(Debug)]
// pub struct ExecutionData {
//     pub execution_stack: VecDeque<ExecutionItem>,
//     pub results: HashMap<String, Value>,
//     pub waiting_queue: WaitingQueue,
//     pub canceled: bool,
// }

// /// 主执行函数：遍历执行栈，处理每个节点
// ///
// /// 参数说明：
// /// - workflow: 工作流，包含所有节点、连接和 pin_data；
// /// - globals, env: 全局变量和环境数据（这里作为 JSON 对象传入，可扩展）；
// /// - exec_data: 执行上下文，包含执行栈、结果和等待队列；
// /// - required_parent_count: 子节点至少需要来自多少个父节点的数据后才开始执行（用于多父节点合并）。
// pub fn run_workflow(
//     workflow: &Workflow,
//     globals: Value,
//     env: Value,
//     exec_data: &mut ExecutionData,
//     required_parent_count: usize,
// ) -> Result<(), String> {
//     while let Some(item) = exec_data.execution_stack.pop_front() {
//         if exec_data.canceled {
//             warn!("Execution canceled, stopping.");
//             return Ok(());
//         }

//         // 查找当前节点（内嵌的 Node 定义）
//         let node = match workflow.nodes.get(&item.node_name) {
//             Some(n) => n,
//             None => {
//                 error!("Node '{}' not found, skipping.", item.node_name);
//                 continue;
//             }
//         };
//         if node.disabled {
//             info!("Node '{}' is disabled, skipping.", node.name);
//             continue;
//         }

//         info!("Executing node '{}', input: {}", node.name, item.input_data);
//         // 模拟节点执行：如果输入为字符串，则在末尾添加 "-> processed_by_NODE"
//         let node_output = simulate_node_exec(node, &item.input_data);
//         exec_data.results.insert(node.name.clone(), node_output.clone());

//         // 遍历所有下游连接
//         let outgoings = workflow.get_outgoings(&node.name);
//         for conn in outgoings {
//             let child_name = &conn.child_name;
//             // 查找子节点配置
//             let child_node = match workflow.nodes.get(child_name) {
//                 Some(n) => n,
//                 None => {
//                     error!("Child node '{}' not found.", child_name);
//                     continue;
//                 }
//             };

//             // 如果子节点有 PinData，则直接使用 PinData（优先级最高）
//             if let Some(pin_val) = workflow.pin_data.get(child_name) {
//                 info!("Child node '{}' is pinned. Using pinData: {}", child_name, pin_val);
//                 exec_data.execution_stack.push_back(ExecutionItem {
//                     node_name: child_name.clone(),
//                     input_data: pin_val.clone(),
//                 });
//             } else {
//                 // 否则，将当前父节点输出加入等待队列（用于多父节点合并）
//                 exec_data.waiting_queue.add(child_name, &node.name, node_output.clone());
//                 // 如果等待队列中数据足够，则合并后生成统一输入
//                 if exec_data.waiting_queue.is_ready(child_name, required_parent_count) {
//                     let merged = exec_data.waiting_queue.merge(child_name)
//                         .ok_or_else(|| format!("Failed to merge waiting data for child '{}'", child_name))?;
//                     // 构造映射上下文，将合并结果放入 "$json" 字段中
//                     let mapping_context = json!({ "$json": merged });
//                     // 如果子节点配置了映射表达式，则调用映射引擎
//                     if let Some(mapping_expr) = &child_node.input_mapping {
//                         info!("Child node '{}' has mapping: {}", child_name, mapping_expr);
//                         match compile_and_search(mapping_expr, &mapping_context) {
//                             Ok(mapped) => {
//                                 info!("Mapped output for child '{}': {}", child_name, mapped);
//                                 exec_data.execution_stack.push_back(ExecutionItem {
//                                     node_name: child_name.clone(),
//                                     input_data: mapped,
//                                 });
//                             }
//                             Err(e) => {
//                                 error!("Mapping error for child '{}': {:?}", child_name, e);
//                                 return Err(format!("Mapping error for child '{}': {:?}", child_name, e));
//                             }
//                         }
//                     } else {
//                         // 若无映射表达式，则直接将合并后的结果推入执行栈
//                         info!("Child node '{}' has no mapping; using merged input.", child_name);
//                         exec_data.execution_stack.push_back(ExecutionItem {
//                             node_name: child_name.clone(),
//                             input_data: merged,
//                         });
//                     }
//                 }
//             }
//         }
//     }
//     Ok(())
// }

// /// 模拟节点执行：
// /// - 如果输入为字符串，则返回 "input -> processed_by_NODE"；
// /// - 如果输入为对象，则在对象中插入 "processed_by" 字段。
// fn simulate_node_exec(node: &Node, input: &Value) -> Value {
//     if input.is_string() {
//         let s = input.as_str().unwrap();
//         Value::String(format!("{} -> processed_by_{}", s, node.name))
//     } else if input.is_object() {
//         let mut obj = input.as_object().unwrap().clone();
//         obj.insert("processed_by".to_string(), Value::String(node.name.clone()));
//         Value::Object(obj)
//     } else if input.is_array() {
//         let mut arr = input.as_array().unwrap().clone();
//         arr.push(Value::String(format!("processed_by_{}", node.name)));
//         Value::Array(arr)
//     } else {
//         Value::String(format!("(unknown) -> processed_by_{}", node.name))
//     }
// }