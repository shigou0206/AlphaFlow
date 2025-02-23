// src/openai/mod.rs

pub mod openai_params;
pub mod openai_handler;

use crate::registry::NodeRegistry;
use std::sync::Arc;

/// 供 orchestrator 或 lib.rs 调用，以注册OpenAi节点
pub fn register_node(registry: &mut NodeRegistry) {
    registry.register(Arc::new(openai_handler::OpenAiChatHandler::new()));
}