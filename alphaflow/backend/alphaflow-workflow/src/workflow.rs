use std::collections::{HashMap, VecDeque};
use serde_json::{Value, json};
use alphaflow_nodes::node_type::{NodeType, NodeExecutionContext, NodeOutput, NodeError};
use alphaflow_nodes::NodeRegistry;

/// 表示在工作流中定义的“节点实例”
/// 这里的 `id`/`name` 是唯一标识；
/// `node_type_name` 则是在注册表里可查到对应的实现。
#[derive(Debug, Clone)]
pub struct WorkflowNode {
    pub id: String,
    /// 对应在 NodeRegistry 里的 name，例如 "mock_node", "http_handler" 等
    pub node_type_name: String,
    /// 节点配置，通常来自用户；会在执行时作为 `parameters`
    pub parameters: Value,
    /// 是否处于禁用状态
    pub disabled: bool,
}

impl WorkflowNode {
    pub fn new(id: &str, node_type_name: &str, parameters: Value) -> Self {
        Self {
            id: id.to_string(),
            node_type_name: node_type_name.to_string(),
            parameters,
            disabled: false,
        }
    }
}

/// 工作流结构，包含节点、节点间连接、以及一些可选的工作流级属性
#[derive(Debug, Default)]
pub struct Workflow {
    /// 工作流ID，可选
    pub id: Option<String>,

    /// 节点ID -> 节点实例
    pub nodes: HashMap<String, WorkflowNode>,

    /// 源节点ID -> 目标节点ID列表
    ///
    /// 对于多输出多输入的场景，可能需要更复杂的结构，
    /// 这里先用最简单的一对多表征。
    pub connections_by_source: HashMap<String, Vec<String>>,

    /// 可选：目标节点ID -> 源节点ID列表（反向关系）
    /// 在 add_node, connect_nodes 等操作后，可通过 build_reverse_connections 来同步维护
    pub connections_by_destination: HashMap<String, Vec<String>>,

    /// 工作流是否激活
    pub active: bool,

    /// 可选：工作流级别配置，如时区等
    pub settings: Value,
}

impl Workflow {
    /// 创建一个空的 Workflow
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
    //      节点增删改查
    // -----------------------------

    /// 添加一个节点到工作流，如果已存在同名节点可选择覆盖或报错
    pub fn add_node(&mut self, node: WorkflowNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// 从工作流移除某个节点，同时移除相关的连线
    pub fn remove_node(&mut self, node_id: &str) {
        // 1. 从 self.nodes 删除
        self.nodes.remove(node_id);

        // 2. 从 connections_by_source 移除所有源节点是 node_id 的记录
        self.connections_by_source.remove(node_id);

        // 3. 同时从所有其他源节点的目标列表里删掉 node_id
        for (_, targets) in self.connections_by_source.iter_mut() {
            targets.retain(|t| t != node_id);
        }

        // 4. 若有 connections_by_destination，也处理相同逻辑
        self.connections_by_destination.remove(node_id);
        for (_, sources) in self.connections_by_destination.iter_mut() {
            sources.retain(|s| s != node_id);
        }
    }

    /// 更新一个节点的参数 (或其他属性)
    pub fn update_node_parameters(&mut self, node_id: &str, new_params: Value) -> Result<(), String> {
        match self.nodes.get_mut(node_id) {
            Some(n) => {
                n.parameters = new_params;
                Ok(())
            },
            None => Err(format!("Node {} not found", node_id))
        }
    }

    // -----------------------------
    //      连接管理
    // -----------------------------

    /// 连接两个节点 (source -> target)
    pub fn connect_nodes(&mut self, source_id: &str, target_id: &str) -> Result<(), String> {
        // 校验节点是否存在
        if !self.nodes.contains_key(source_id) {
            return Err(format!("Source node {} not found", source_id));
        }
        if !self.nodes.contains_key(target_id) {
            return Err(format!("Target node {} not found", target_id));
        }
        // 插入记录
        self.connections_by_source
            .entry(source_id.to_string())
            .or_insert_with(Vec::new)
            .push(target_id.to_string());

        // 如需同步构建反向表，也做下处理
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

    /// 若仅在初始化时生成反向连接，可以用此方法
    /// 遍历 connections_by_source 并反向写入 connections_by_destination
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
    //      父子节点查询
    // -----------------------------

    /// 获取直接子节点列表
    pub fn get_children(&self, node_id: &str) -> Vec<String> {
        self.connections_by_source
            .get(node_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 获取直接父节点列表
    pub fn get_parents(&self, node_id: &str) -> Vec<String> {
        self.connections_by_destination
            .get(node_id)
            .cloned()
            .unwrap_or_default()
    }

    // -----------------------------
    //    workflow 执行主逻辑
    // -----------------------------

    /// 一个简单的运行示例：
    /// 1. 找所有"没有父节点" 或 "触发节点" 做为起始
    /// 2. BFS顺序执行，节点执行后将输出放到 results 映射中
    /// 3. 下游节点执行前，收集所有上游 outputData 并合并
    /// 4. 若节点报错，则停止
    ///
    /// 如果需要并行 / 异步，可以在 BFS 里并行调度
    pub async fn run(&self, registry: &NodeRegistry) -> Result<HashMap<String, Value>, NodeError> {
        // 1. 找到所有“父节点为空” 或 "可视为触发节点" 的节点 作为起始
        //    这里简单处理：get_parents(node) == 0 && disabled==false
        let mut start_nodes = Vec::new();
        for (node_id, node) in &self.nodes {
            if node.disabled {
                continue;
            }
            let parents = self.get_parents(node_id);
            if parents.is_empty() {
                // 认为它是起始
                start_nodes.push(node_id.clone());
            }
        }

        // 如果找不到任何起始节点，就不执行
        if start_nodes.is_empty() {
            return Ok(HashMap::new());
        }

        // BFS所需
        let mut queue: VecDeque<String> = VecDeque::from(start_nodes);

        // 保存每个节点执行结果: node_id -> 执行后输出(serde_json::Value)
        let mut results: HashMap<String, Value> = HashMap::new();

        // 2. BFS 执行
        while let Some(current_id) = queue.pop_front() {
            // 如果节点被禁用或不存在，则跳过
            let node = match self.nodes.get(&current_id) {
                Some(n) if !n.disabled => n,
                _ => continue,
            };

            // 找到节点对应的 NodeType 实现
            let node_impl = match registry.get(&node.node_type_name) {
                Some(n) => n,
                None => {
                    // 如果在注册表里找不到对应的实现，报错并停止
                    return Err(NodeError::InvalidConfig(format!(
                        "NodeType '{}' not registered for node '{}'",
                        node.node_type_name, current_id
                    )));
                }
            };

            // 3. 收集所有上游节点的输出，并合并为 input_data
            //    这里的策略：把多个上游输出合并成数组
            let mut merged_input = Vec::new();
            let parent_ids = self.get_parents(&current_id);
            for pid in parent_ids {
                if let Some(data) = results.get(&pid) {
                    merged_input.push(data.clone());
                }
            }
            // 如果只有一个上游，就直接拿它的输出，否则做成数组
            let input_data = if merged_input.len() == 1 {
                merged_input.remove(0)
            } else if merged_input.is_empty() {
                // 没有上游时，给个默认的空object
                json!({})
            } else {
                json!(merged_input)
            };

            // 构造 NodeExecutionContext
            let ctx = NodeExecutionContext {
                parameters: node.parameters.clone(),
                input_data,
            };

            // 调用 execute
            let output = match node_impl.execute(&ctx).await {
                Ok(o) => o,
                Err(err) => {
                    // n8n 默认遇到执行错误就停止(也可改为仅跳过)
                    return Err(err);
                }
            };

            // 4. 记录结果
            results.insert(current_id.clone(), output.data);

            // 5. 把子节点加入队列执行
            let children = self.get_children(&current_id);
            for child in children {
                // 这里如果 child 未在队列中，可插入；或者简单方式直接 push
                queue.push_back(child);
            }
        }

        // 返回各节点的执行结果
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;  // 引用 workflow.rs 中的内容
    use alphaflow_nodes::NodeRegistry;
    use alphaflow_nodes::node_type::{NodeType, NodeError};
    use alphaflow_nodes::registry_helper::register_all_nodes;
    use std::sync::Arc;
    use serde_json::json;
    use tokio; // 确保你启用 async 测试

    #[tokio::test]
    async fn test_workflow_with_openai_chat_node() {
        // 1) 准备: 注册表 + 注册 OpenAiChatHandler
        let mut registry = NodeRegistry::new();
        register_all_nodes(&mut registry);
        // 2) 创建一个 Workflow
        let mut wf = Workflow::new(Some("openai_test_wf".to_string()));

        // 3) 构造节点：指定 node_type_name = "openai"
        //    parameters 里包含 prompt, model, api_key, base_url 等
        let chat_node = WorkflowNode::new(
            "openai_chat",       // 节点 ID
            "openai",            // node_type_name，与 handler.name() 匹配
            json!({
                "api_key": "sk-proj-WR22G6DP-9EA",  // 真实测试请从env加载
                "prompt": "Hello from alphaflow test in workflow!",
                "model": "gpt-3.5-turbo",        // 或你的自定义模型
                "base_url": "https://api.openai.com/v1",
                "system_content": "You are a helpful assistant."
                // 可选: temperature, max_tokens 等
            })
        );

        // 4) 将节点添加进 Workflow
        wf.add_node(chat_node);

        // 由于只有一个节点，无需连接（connect_nodes）也可正常执行
        // 如果要加其它节点，需要先 wf.add_node(...) 再 wf.connect_nodes(source, target).

        // 5) 运行 workflow
        //   若你的 OpenAiChatHandler 真的调用了 OpenAI API，需要网络和有效 API_KEY 才能成功
        let result = wf.run(&registry).await;

        match result {
            Ok(outputs) => {
                // 期望有一个 key: "openai_chat"
                if let Some(output_val) = outputs.get("openai_chat") {
                    // output_val 里应该有 "model" 和 "response"
                    let model_str = output_val["model"].as_str().unwrap_or("");
                    let response_str = output_val["response"].as_str().unwrap_or("");

                    println!("OpenAI Node Output:\n  model={model_str}\n  response={response_str}");
                    // 做一些断言
                    assert_eq!(model_str, "gpt-3.5-turbo"); 
                    assert!(!response_str.is_empty(), "OpenAI response should not be empty!");
                } else {
                    panic!("Expected 'openai_chat' in workflow output");
                }
            }
            Err(e) => {
                panic!("Workflow with openai node failed: {e:?}");
            }
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use alphaflow_nodes::node_type::{NodeExecutionContext, NodeOutput, NodeError};
//     use async_trait::async_trait;
//     use serde_json::json;
//     use alphaflow_nodes::NodeRegistry;

//     /// 一个最简单的 MockNodeType，用于测试
//     struct MockNodeType;
//     #[async_trait]
//     impl NodeType for MockNodeType {
//         fn name(&self) -> &str { "mock_node" }
//         fn display_name(&self) -> &str { "Mock Node" }
//         async fn execute(
//             &self, 
//             ctx: &NodeExecutionContext
//         ) -> Result<NodeOutput, NodeError> {
//             // 模拟：把 parameters + input_data 合并输出
//             let out = json!({
//                 "parameters": ctx.parameters,
//                 "input": ctx.input_data
//             });
//             Ok(NodeOutput { data: out })
//         }
//     }

//     #[tokio::test]
//     async fn test_workflow_basic_bfs() {
//         let mut registry = NodeRegistry::new();
//         // 注册一个 mock_node
//         registry.register(std::sync::Arc::new(MockNodeType));

//         // 构造 workflow
//         let mut wf = Workflow::new(Some("test_wf".to_string()));

//         // 添加节点
//         let node_a: WorkflowNode = WorkflowNode::new("A", "mock_node", json!({"paramA": 123}));
//         let node_b = WorkflowNode::new("B", "mock_node", json!({"paramB": 456}));
//         let node_c = WorkflowNode::new("C", "mock_node", json!({"paramC": 789}));

//         wf.add_node(node_a);
//         wf.add_node(node_b);
//         wf.add_node(node_c);

//         // 连接关系: A -> B, B -> C
//         wf.connect_nodes("A", "B").unwrap();
//         wf.connect_nodes("B", "C").unwrap();
//         // 没有明确起始节点，但 A 没有父节点，故 BFS 会以 A 为起点

//         // 运行
//         let results = wf.run(&registry).await.unwrap();

//         // 检查执行结果
//         // A 的 input_data 应为空object
//         let a_result = results.get("A").unwrap();
//         assert_eq!(
//             a_result["parameters"]["paramA"],
//             json!(123)
//         );
//         assert_eq!(a_result["input"], json!({}));

//         // B 的 input_data 应该是 A 的输出
//         let b_result = results.get("B").unwrap();
//         assert_eq!(
//             b_result["parameters"]["paramB"],
//             json!(456)
//         );
//         // B 的 input 应为 A 的 output
//         assert_eq!(b_result["input"]["parameters"]["paramA"], json!(123));

//         // C 的 input_data 应该是 B 的输出
//         let c_result = results.get("C").unwrap();
//         assert_eq!(
//             c_result["parameters"]["paramC"],
//             json!(789)
//         );
//         assert_eq!(c_result["input"]["parameters"]["paramB"], json!(456));
//     }

//     #[tokio::test]
//     async fn test_workflow_remove_node() {
//         let mut wf = Workflow::new(None);
//         wf.add_node(WorkflowNode::new("X", "mock_node", json!({})));
//         wf.add_node(WorkflowNode::new("Y", "mock_node", json!({})));
//         wf.connect_nodes("X", "Y").unwrap();

//         // remove Y
//         wf.remove_node("Y");
//         assert!(!wf.nodes.contains_key("Y"));
//         // connections_by_source 里也应被移除
//         assert!(!wf.connections_by_source["X"].contains(&"Y".to_string()));
//     }
// }