use derivative::Derivative;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::input_mapping::InputMapping;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Derivative)]
#[derivative(Hash)]
pub struct Node {
    /// 工作流中此节点的唯一标识，例如 "chat_node"
    pub name: String,
    /// 节点类型名称，用于在 NodeRegistry 中查找对应的 NodeType 实现，
    /// 例如 "openai"、"http"
    pub node_type_name: String,
    /// 执行时是否禁用此节点
    #[serde(default)]
    pub disabled: bool,
    /// 节点输入映射规则，决定如何从上游数据中提取或转换出子节点所需输入
    #[serde(default)]
    pub input_mapping: Option<InputMapping>,
    /// 节点专有参数，例如 API key、prompt、URL 等，供节点实现使用
    #[serde(default)]
    #[derivative(Hash = "ignore")]
    pub parameters: Value,
    /// 在 UI 上显示的名称（可选）
    #[serde(default)]
    pub display_name: Option<String>,
    /// 节点描述信息（可选）
    #[serde(default)]
    pub description: Option<String>,
    /// 其他自定义配置项（可选）
    #[serde(default)]
    #[derivative(Hash = "ignore")]
    pub custom_config: Option<Value>,
}

impl Node {
    /// 创建一个新的节点，必须提供节点的工作流 ID 和节点类型名称
    pub fn new(name: &str, node_type_name: &str) -> Self {
        Self {
            name: name.to_string(),
            node_type_name: node_type_name.to_string(),
            disabled: false,
            input_mapping: None,
            parameters: Value::Null,
            display_name: None,
            description: None,
            custom_config: None,
        }
    }

    /// 设置映射规则
    pub fn with_input_mapping(mut self, mapping: InputMapping) -> Self {
        self.input_mapping = Some(mapping);
        self
    }

    /// 设置节点参数
    pub fn with_parameters(mut self, params: Value) -> Self {
        self.parameters = params;
        self
    }

    /// 标记节点为禁用状态
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// 设置 UI 显示名称
    pub fn with_display_name(mut self, display_name: &str) -> Self {
        self.display_name = Some(display_name.to_string());
        self
    }

    /// 设置节点描述信息
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// 设置其他自定义配置
    pub fn with_custom_config(mut self, config: Value) -> Self {
        self.custom_config = Some(config);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_node_creation_and_serialization() {
        let node = Node::new("test_node", "openai")
            .with_input_mapping(InputMapping::Single("uppercase(@.name)".into()))
            .with_display_name("Test Node")
            .with_description("用于测试的节点")
            .with_custom_config(json!({ "retry": 3 }));

        assert_eq!(node.name, "test_node");
        assert!(!node.disabled);
        assert_eq!(node.input_mapping.as_ref().unwrap(), &InputMapping::Single("uppercase(@.name)".into()));
        assert_eq!(node.display_name.as_ref().unwrap(), "Test Node");
        assert_eq!(node.description.as_ref().unwrap(), "用于测试的节点");
        assert_eq!(node.custom_config.as_ref().unwrap(), &json!({ "retry": 3 }));

        // 测试序列化与反序列化
        let serialized = serde_json::to_string(&node).unwrap();
        let deserialized: Node = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, "test_node");
        assert_eq!(deserialized.node_type_name, "openai");
    }
}