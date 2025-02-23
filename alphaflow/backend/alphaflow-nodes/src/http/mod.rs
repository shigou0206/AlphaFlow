// src/http/mod.rs
//! mod.rs for the `http` node
//! 
//! Provides `register_node` which registers HttpHandler into NodeRegistry.

pub mod http_params;
pub mod http_handler;

use crate::registry::NodeRegistry;
use std::sync::Arc;

/// 供外部调用以注册 HTTP 节点到 NodeRegistry
pub fn register_node(registry: &mut NodeRegistry) {
    registry.register(Arc::new(http_handler::HttpHandler::new()));
}