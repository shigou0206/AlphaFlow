// alphaflow-nodes/src/registry_helper.rs

use crate::registry::NodeRegistry;
use crate::http::http_handler::HttpHandler;
use crate::openai::openai_handler::OpenAiChatHandler;
use std::sync::Arc;

/// 一次性注册所有节点 (HTTP, OpenAI 等)，以简化用户调用。
pub fn register_all_nodes(registry: &mut NodeRegistry) {
    // 如果你还有更多节点，也在此依次 register
    registry.register(Arc::new(HttpHandler::new()));
    registry.register(Arc::new(OpenAiChatHandler::new()));
}