//! registry.rs
//! 
//! 提供一个 `NodeRegistry` 结构，用于维护 (name -> Arc<dyn NodeType>)。
//! 通常在启动时一次性注册所有节点，后续按节点名检索即可。

use std::collections::HashMap;
use std::sync::Arc;
use crate::node_type::NodeType;

/// 一个简单的节点注册表，内含 (name -> Arc<dyn NodeType>)。
/// 若不需要在运行时动态增删，可维持一个无锁版本。
/// 如果要多线程写(动态注册/移除)，应当把 `handlers` 包在 RwLock/Mutex 中。
pub struct NodeRegistry {
    handlers: HashMap<String, Arc<dyn NodeType>>,
}

impl NodeRegistry {
    /// 创建一个空的注册表
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new()
        }
    }

    /// 注册一个节点，以 `node.name()` 作为映射的 key
    pub fn register(&mut self, node: Arc<dyn NodeType>) {
        self.handlers.insert(node.name().to_owned(), node);
    }

    /// 按名称获取节点实现
    pub fn get(&self, name: &str) -> Option<Arc<dyn NodeType>> {
        self.handlers.get(name).cloned()
    }

    /// 返回目前已注册的节点名称列表（调试/枚举用）
    pub fn list_nodes(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_type::{
        NodeType, NodeExecutionContext, NodeOutput, NodeError
    };
    use async_trait::async_trait;
    use serde_json::Value;

    /// mock node for testing registry
    struct MockNode;
    #[async_trait]
    impl NodeType for MockNode {
        fn name(&self) -> &str { "mock_node" }
        fn display_name(&self) -> &str { "Mock Node" }
        async fn execute(
            &self, 
            _: &NodeExecutionContext
        ) -> Result<NodeOutput, NodeError> {
            Ok(NodeOutput {
                data: Value::String("mock output".into())
            })
        }
    }

    #[tokio::test]
    async fn test_register_and_get_node() {
        let mut registry = NodeRegistry::new();
        registry.register(Arc::new(MockNode));

        // 检索
        let maybe_node = registry.get("mock_node");
        assert!(maybe_node.is_some(), "should find the newly registered node");

        // 调用execute
        let node_impl = maybe_node.unwrap();
        let result = node_impl.execute(&NodeExecutionContext {
            parameters: Value::Null,
            input_data: Value::Null,
            globals: Value::Null,
            env: Value::Null,
            pin_data: None,
        }).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().data,
            Value::String("mock output".into())
        );
    }

    #[test]
    fn test_list_nodes() {
        let mut registry = NodeRegistry::new();
        registry.register(Arc::new(MockNode));

        let nodes = registry.list_nodes();
        assert_eq!(nodes, vec!["mock_node"]);
    }

    #[test]
    fn test_get_nonexistent() {
        let registry = NodeRegistry::new();
        let node = registry.get("not_exist");
        assert!(node.is_none());
    }
}