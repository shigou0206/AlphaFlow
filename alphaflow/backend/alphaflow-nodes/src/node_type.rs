// src/node_type.rs

use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

/// 节点执行时的上下文信息，用于传递各类运行时数据。
#[derive(Debug)]
pub struct NodeExecutionContext {
    /// 节点的参数，由前端配置传入，例如映射表达式、API 密钥等。
    pub parameters: Value,
    /// 上游节点传入的数据（可能为多个父节点合并后的 JSON）。
    pub input_data: Value,
    /// 全局变量，例如用户信息、流程级配置等。
    pub globals: Value,
    /// 环境或运行时变量，例如系统环境、时间戳等。
    pub env: Value,
    /// 如果节点被 Pin，则存放该节点固定使用的数据。
    pub pin_data: Option<Value>,
}

/// 节点执行返回值，统一以 JSON 格式返回，便于后续节点解析。
#[derive(Debug)]
pub struct NodeOutput {
    pub data: Value,
}

/// 节点执行过程中可能出现的错误类型。
#[derive(Error, Debug)]
pub enum NodeError {
    #[error("Invalid config: {0}")]
    InvalidConfig(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

/// 节点元数据，用于动态生成前端配置界面或进行节点描述。
#[derive(Debug)]
pub struct NodeDescription {
    /// 节点内在标识（例如 "openai"、"http"）。
    pub name: String,
    /// UI显示名称（例如 "OpenAI Node"）。
    pub display_name: String,
    /// 节点的可配置属性列表。
    pub properties: Vec<NodeProperty>,
}

/// 单个节点的属性信息，用于前端动态生成配置界面。
#[derive(Debug)]
pub struct NodeProperty {
    /// 属性名称（例如 "url"、"prompt"）。
    pub name: String,
    /// UI显示名称（例如 "URL"、"Prompt"）。
    pub display_name: String,
    /// 属性类型（例如 "string", "number", "boolean"）。
    pub type_: String,
    /// 是否为必填项。
    pub required: bool,
}

/// 所有节点必须实现的核心接口，定义节点如何执行自己的业务逻辑。
#[async_trait]
pub trait NodeType: Send + Sync {
    /// 返回节点内在标识（例如 "openai"、"http"），用于注册和检索。
    fn name(&self) -> &str;
    /// 返回节点在 UI 中显示的名称（例如 "OpenAI Node"）。
    fn display_name(&self) -> &str;
    /// 可选：提供节点描述信息，帮助前端构建配置界面。默认返回 None。
    fn description(&self) -> Option<NodeDescription> {
        None
    }
    /// 异步执行节点逻辑：
    /// - 首先解析/验证参数；
    /// - 根据上下文中的输入数据进行实际操作（例如 HTTP 请求、AI 调用等）；
    /// - 返回 NodeOutput 或 NodeError。
    async fn execute(&self, ctx: &NodeExecutionContext) -> Result<NodeOutput, NodeError>;
}

/// 辅助函数，用于构造 NodeExecutionContext。将各部分数据组合在一起。
pub fn build_context(
    parameters: Value,
    input_data: Value,
    globals: Value,
    env: Value,
    pin_data: Option<Value>,
) -> NodeExecutionContext {
    NodeExecutionContext {
        parameters,
        input_data,
        globals,
        env,
        pin_data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    /// 一个极简的 MockNode，用于测试 NodeType 接口
    pub struct MockNode;

    #[async_trait]
    impl NodeType for MockNode {
        fn name(&self) -> &str {
            "mock_node"
        }
        fn display_name(&self) -> &str {
            "Mock Node"
        }
        async fn execute(
            &self,
            ctx: &NodeExecutionContext,
        ) -> Result<NodeOutput, NodeError> {
            // 简单示例：合并参数和输入数据输出
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
        let ctx = build_context(
            json!({"key": "value"}),
            json!([1, 2, 3]),
            json!({}),
            json!({}),
            None,
        );
        let output = node.execute(&ctx).await.expect("Execution should succeed");
        assert_eq!(
            output.data,
            json!({
                "parameters": {"key": "value"},
                "input_data": [1,2,3]
            })
        );
    }

    #[tokio::test]
    async fn test_error_example() {
        let err = NodeError::InvalidConfig("Some invalid param".to_string());
        match err {
            NodeError::InvalidConfig(msg) => assert_eq!(msg, "Some invalid param"),
            _ => panic!("Expected InvalidConfig"),
        }
    }
}