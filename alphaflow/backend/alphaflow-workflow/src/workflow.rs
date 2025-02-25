// src/workflow.rs

use std::collections::{HashMap, VecDeque};
use serde_json::{Value, json};
use alphaflow_nodes::node::Node;
use alphaflow_nodes::node_type::{NodeType, NodeExecutionContext, NodeError};
use alphaflow_nodes::NodeRegistry;
use crate::jmes_runtime::compile_and_search;
use log::{info, warn, error};

/// 工作流结构，包含节点、连接和全局设置
#[derive(Debug, Default)]
pub struct Workflow {
    /// 工作流 ID（可选）
    pub id: Option<String>,
    /// 节点 ID -> 节点实例（静态配置）
    pub nodes: HashMap<String, Node>,
    /// 源节点 ID -> 目标节点 ID 列表
    pub connections_by_source: HashMap<String, Vec<String>>,
    /// 目标节点 ID -> 源节点 ID 列表（反向关系）
    pub connections_by_destination: HashMap<String, Vec<String>>,
    /// 工作流是否激活
    pub active: bool,
    /// 工作流级别配置（例如时区等）
    pub settings: Value,
}

impl Workflow {
    /// 创建一个空的工作流
    pub fn new(id: Option<String>) -> Self {
        Self {
            id,
            nodes: HashMap::new(),
            connections_by_source: HashMap::new(),
            connections_by_destination: HashMap::new(),
            active: false,
            settings: json!({}),
        }
    }

    // -----------------------------
    // 节点管理
    // -----------------------------

    /// 添加节点
    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.name.clone(), node);
    }

    /// 移除节点并删除相关连接
    pub fn remove_node(&mut self, node_id: &str) {
        self.nodes.remove(node_id);
        self.connections_by_source.remove(node_id);
        for (_, targets) in self.connections_by_source.iter_mut() {
            targets.retain(|t| t != node_id);
        }
        self.connections_by_destination.remove(node_id);
        for (_, sources) in self.connections_by_destination.iter_mut() {
            sources.retain(|s| s != node_id);
        }
    }

    /// 连接两个节点 (source -> target)
    pub fn connect_nodes(&mut self, source_id: &str, target_id: &str) -> Result<(), String> {
        if !self.nodes.contains_key(source_id) {
            return Err(format!("Source node {} not found", source_id));
        }
        if !self.nodes.contains_key(target_id) {
            return Err(format!("Target node {} not found", target_id));
        }
        self.connections_by_source
            .entry(source_id.to_string())
            .or_insert_with(Vec::new)
            .push(target_id.to_string());

        self.connections_by_destination
            .entry(target_id.to_string())
            .or_insert_with(Vec::new)
            .push(source_id.to_string());
        Ok(())
    }

    /// 断开两个节点之间的连接
    pub fn disconnect_nodes(&mut self, source_id: &str, target_id: &str) {
        if let Some(targets) = self.connections_by_source.get_mut(source_id) {
            targets.retain(|t| t != target_id);
        }
        if let Some(sources) = self.connections_by_destination.get_mut(target_id) {
            sources.retain(|s| s != source_id);
        }
    }

    /// 构建反向连接表（仅用于初始化时）
    pub fn build_reverse_connections(&mut self) {
        let mut reverse_map = HashMap::new();
        for (src, targets) in &self.connections_by_source {
            for t in targets {
                reverse_map
                    .entry(t.clone())
                    .or_insert_with(Vec::new)
                    .push(src.clone());
            }
        }
        self.connections_by_destination = reverse_map;
    }

    // -----------------------------
    // 父子节点查询
    // -----------------------------

    /// 获取指定节点的直接子节点列表
    pub fn get_children(&self, node_id: &str) -> Vec<String> {
        self.connections_by_source
            .get(node_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 获取指定节点的直接父节点列表
    pub fn get_parents(&self, node_id: &str) -> Vec<String> {
        self.connections_by_destination
            .get(node_id)
            .cloned()
            .unwrap_or_default()
    }

    // -----------------------------
    // 工作流执行 (BFS 示例)
    // -----------------------------

    /// 运行工作流，采用 BFS 遍历所有节点进行执行
    ///
    /// 执行流程：
    /// 1. 找到所有没有父节点且未禁用的节点作为起始。
    /// 2. 对于每个节点：
    ///    a. 根据节点配置中的 node_type_name，从 NodeRegistry 中查找对应实现。
    ///    b. 合并所有父节点的输出：
    ///         - 如果只有一个父节点，则直接使用其输出。
    ///         - 如果多个，则合并为数组；如果没有，则使用空对象。
    ///    c. 如果节点配置了 input_mapping，则调用表达式引擎对合并结果进行映射，
    ///       注意映射表达式应明确引用上游数据中某个字段（例如 "uppercase(@.response)"）。
    ///    d. 构造 NodeExecutionContext，将节点的 custom_config 作为 parameters 传入（也可调整为 parameters 字段）。
    ///    e. 调用节点的 execute 方法，记录输出结果。
    ///    f. 将子节点加入队列继续执行。
    pub async fn run(&self, registry: &NodeRegistry) -> Result<HashMap<String, Value>, NodeError> {
        // 1) 找到起始节点（无父节点且未禁用）
        let mut start_nodes = Vec::new();
        for (node_id, node_cfg) in &self.nodes {
            if node_cfg.disabled {
                continue;
            }
            if self.get_parents(node_id).is_empty() {
                start_nodes.push(node_id.clone());
            }
        }
        if start_nodes.is_empty() {
            return Ok(HashMap::new());
        }

        let mut queue: VecDeque<String> = VecDeque::from(start_nodes);
        let mut results: HashMap<String, Value> = HashMap::new();

        while let Some(current_id) = queue.pop_front() {
            // 获取当前节点配置，跳过禁用或不存在的节点
            let node_cfg = match self.nodes.get(&current_id) {
                Some(n) if !n.disabled => n,
                _ => continue,
            };

            // 2) 使用 node_type_name 从注册表中查找节点实现
            let node_impl = match registry.get(&node_cfg.node_type_name) {
                Some(impl_) => impl_,
                None => {
                    let err_msg = format!(
                        "NodeType '{}' not registered for node '{}'",
                        node_cfg.node_type_name, current_id
                    );
                    error!("{}", err_msg);
                    return Err(NodeError::InvalidConfig(err_msg));
                }
            };

            // 3) 合并所有父节点的输出
            let parent_ids = self.get_parents(&current_id);
            let mut merged_inputs = Vec::new();
            for pid in parent_ids {
                if let Some(data) = results.get(&pid) {
                    merged_inputs.push(data.clone());
                }
            }
            let merged_input = if merged_inputs.len() == 1 {
                merged_inputs.remove(0)
            } else if merged_inputs.is_empty() {
                json!({})
            } else {
                json!(merged_inputs)
            };

            // 4) 执行映射：如果配置了 input_mapping，则对合并后的数据执行映射处理
            let final_input_data = if let Some(mapping) = &node_cfg.input_mapping {
                // 构造映射上下文：将合并结果放入 "$json" 字段
                let ctx_json = json!({ "$json": merged_input });
                match mapping {
                    alphaflow_nodes::input_mapping::InputMapping::Single(expr_str) => {
                        match compile_and_search(expr_str, &ctx_json) {
                            Ok(mapped) => mapped,
                            Err(e) => {
                                let err_msg = format!(
                                    "Mapping error at node '{}' (expr='{:?}'): {:?}",
                                    current_id, expr_str, e
                                );
                                error!("{}", err_msg);
                                return Err(NodeError::InvalidConfig(err_msg));
                            }
                        }
                    }
                    alphaflow_nodes::input_mapping::InputMapping::Multi { fields, defaultValue: _ } => {
                        let mut mapped_obj = serde_json::Map::new();
                        for (field, expr_str) in fields {
                            match compile_and_search(expr_str, &ctx_json) {
                                Ok(mapped_field) => {
                                    mapped_obj.insert(field.clone(), mapped_field);
                                }
                                Err(e) => {
                                    let err_msg = format!(
                                        "Mapping error at node '{}' for field '{}' (expr='{:?}'): {:?}",
                                        current_id, field, expr_str, e
                                    );
                                    error!("{}", err_msg);
                                    return Err(NodeError::InvalidConfig(err_msg));
                                }
                            }
                        }
                        Value::Object(mapped_obj)
                    }
                }
            } else {
                merged_input
            };

            // 5) 构造 NodeExecutionContext
            // 此处我们使用 custom_config 作为节点执行参数
            let parameters = node_cfg.custom_config.clone().unwrap_or(Value::Null);
            let exec_ctx = NodeExecutionContext {
                parameters,
                input_data: final_input_data,
                globals: json!(null),
                env: json!(null),
                pin_data: None,
            };

            // 6) 调用节点实现的 execute 方法
            let output = match node_impl.execute(&exec_ctx).await {
                Ok(o) => o,
                Err(err) => {
                    error!("Execution error at node '{}': {:?}", current_id, err);
                    return Err(err);
                }
            };

            // 保存结果
            results.insert(current_id.clone(), output.data);

            // 7) 将当前节点的子节点加入 BFS 队列
            let children = self.get_children(&current_id);
            for child in children {
                queue.push_back(child);
            }
        }

        Ok(results)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use alphaflow_nodes::NodeRegistry;
    use alphaflow_nodes::registry_helper::register_all_nodes;
    use serde_json::json;
    use tokio;

    #[tokio::test]
    async fn test_workflow_run_two_openai_nodes_with_mapping() {
        // 1) 构造 NodeRegistry 并注册所有节点（例如 openai）
        let mut registry = NodeRegistry::new();
        register_all_nodes(&mut registry);

        // 2) 构造 Workflow
        let mut wf = Workflow::new(Some("openai_test_wf".to_string()));

        // 3) 创建第一个 OpenAI 节点 ("chat_node_1")
        //    它的 custom_config 包含 API key、prompt 等参数，
        //    并直接执行后返回的结果会包含 "response" 字段（例如调用 API 后返回）。
        let openai_node_1 = Node {
            name: "chat_node_1".to_string(),
            node_type_name: "openai".to_string(),
            disabled: false,
            // 此节点不设置 input_mapping，直接执行
            input_mapping: None,
            parameters: json!(null),
            display_name: Some("OpenAI Node 1".to_string()),
            description: None,
            custom_config: Some(json!({
                "api_key": std::env::var("OPENAI_API_KEY").unwrap_or("".to_string()),
                "model": "gpt-3.5-turbo",
                "base_url": "https://api.openai.com/v1",
                "prompt": "Hello from Rust, node 1",
                "system_content": "You are a helpful assistant."
            })),
        };

        // 4) 创建第二个 OpenAI 节点 ("chat_node_2")
        //    该节点的 input_mapping 配置为 "uppercase(@.response)"，意思是取上游输出中的 "response" 字段并转换成大写
        let openai_node_2 = Node {
            name: "chat_node_2".to_string(),
            node_type_name: "openai".to_string(),
            disabled: false,
            input_mapping: Some(alphaflow_nodes::input_mapping::InputMapping::Single("uppercase(@.response)".into())),
            parameters: json!(null),
            display_name: Some("OpenAI Node 2".to_string()),
            description: None,
            custom_config: Some(json!({
                "api_key": std::env::var("OPENAI_API_KEY").unwrap_or("".to_string()),
                "model": "gpt-3.5-turbo",
                "base_url": "https://api.openai.com/v1",
                // 此节点的 prompt 可以为空或固定，主要测试映射效果
                "prompt": "",
                "system_content": "You are an assistant that echoes input in uppercase."
            })),
        };

        // 5) 将两个节点添加到工作流
        wf.add_node(openai_node_1);
        wf.add_node(openai_node_2);

        // 6) 建立连接：chat_node_1 -> chat_node_2
        //    这样 chat_node_2 的输入将会是 chat_node_1 的输出
        wf.connect_nodes("chat_node_1", "chat_node_2").expect("连接节点失败");

        // 7) 执行工作流 (BFS 方式)
        let result = wf.run(&registry).await;
        match result {
            Ok(res_map) => {
                // 输出两个节点的执行结果
                if let Some(output1) = res_map.get("chat_node_1") {
                    println!("Node chat_node_1 output: {}", output1);
                } else {
                    panic!("Expected output for 'chat_node_1'");
                }
                if let Some(output2) = res_map.get("chat_node_2") {
                    println!("Node chat_node_2 output: {}", output2);
                    // 断言：输出应包含转换后的 response 字段（大写）
                    // 假设 chat_node_1 的输出中有 "response" 字段, 则 chat_node_2 会输出其大写形式
                    let response_upper = output2.as_str().unwrap_or("");
                    assert!(response_upper.chars().all(|c| !c.is_lowercase()),
                        "chat_node_2 output should be in uppercase");
                } else {
                    panic!("Expected output for 'chat_node_2'");
                }
            },
            Err(e) => {
                panic!("Workflow error: {:?}", e);
            }
        }
    }
}