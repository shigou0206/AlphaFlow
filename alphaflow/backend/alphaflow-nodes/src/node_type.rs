//! node_type.rs
//! 
//! 定义所有节点共同遵循的核心接口(`NodeType`)，以及执行过程中使用的上下文、输出、错误类型。

use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

/// 节点执行时的上下文信息：
/// - `parameters`: JSON 格式的用户配置 (从前端或脚本注入)；
/// - `input_data`: 上游节点产生的数据，可为Null或任意JSON。
#[derive(Debug)]
pub struct NodeExecutionContext {
    pub parameters: Value,
    pub input_data: Value,
}

/// 节点执行返回值：`data`字段中承载核心输出数据，
/// 下游节点可对其进行二次解析或使用。
#[derive(Debug)]
pub struct NodeOutput {
    pub data: Value,
}

/// 节点执行过程中可能发生的错误类型：
/// - `InvalidConfig`: 参数不合法或缺失；
/// - `ExecutionFailed`: 运行时故障 (网络、IO等)。
#[derive(Error, Debug)]
pub enum NodeError {
    #[error("Invalid config: {0}")]
    InvalidConfig(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

/// 描述一个节点在UI上的元数据，如属性、显示名称等。
/// 如果你需要动态生成配置界面，可以在此定义更多字段(默认值、枚举选项等)。
#[derive(Debug)]
pub struct NodeDescription {
    pub name: String,
    pub display_name: String,
    pub properties: Vec<NodeProperty>,
}

/// 表示单个可配置属性的信息，如 "url", "prompt" 等。
#[derive(Debug)]
pub struct NodeProperty {
    pub name: String,
    pub display_name: String,
    pub type_: String,
    pub required: bool,
}

/// 所有节点必须实现的核心 Trait：`NodeType`。
#[async_trait]
pub trait NodeType: Send + Sync {
    /// 节点的内在标识 (比如 "openai"、"http")，用于注册与检索。
    fn name(&self) -> &str;

    /// UI要显示的节点名称 (比如 "OpenAI Node")。
    fn display_name(&self) -> &str;

    /// 可选：提供节点属性描述，用于动态生成前端配置界面。默认返回 None。
    fn description(&self) -> Option<NodeDescription> {
        None
    }

    /// 异步执行节点逻辑:
    ///  - 解析/验证parameters
    ///  - 实际执行操作 (HTTP, AI等)
    ///  - 返回 NodeOutput 或 NodeError
    async fn execute(&self, ctx: &NodeExecutionContext) -> Result<NodeOutput, NodeError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    /// 一个极简的 MockNode，用来测试 NodeType 的正确性
    pub struct MockNode;

    #[async_trait]
    impl NodeType for MockNode {
        fn name(&self) -> &str { "mock_node" }
        fn display_name(&self) -> &str { "Mock Node" }

        async fn execute(
            &self, 
            ctx: &NodeExecutionContext
        ) -> Result<NodeOutput, NodeError> {
            // 简单示例：把 parameters + input_data 合并输出
            let out = json!({
                "parameters": ctx.parameters,
                "input_data": ctx.input_data
            });
            Ok(NodeOutput { data: out })
        }
    }

    #[tokio::test]
    async fn test_mock_node_execute() {
        let node = MockNode;
        let ctx = NodeExecutionContext {
            parameters: json!({"key": "value"}),
            input_data: json!([1,2,3]),
        };

        let output = node.execute(&ctx).await.expect("execution should succeed");
        // 检查输出
        assert_eq!(
            output.data,
            json!({
                "parameters": {"key":"value"},
                "input_data": [1,2,3]
            })
        );
    }

    #[tokio::test]
    async fn test_error_example() {
        // 假设要测试 NodeError::InvalidConfig
        // 这里只是演示如何直接构造
        let err = NodeError::InvalidConfig("Some invalid param".to_string());
        match err {
            NodeError::InvalidConfig(msg) => {
                assert_eq!(msg, "Some invalid param");
            }
            _ => panic!("Expected InvalidConfig"),
        }
    }
}