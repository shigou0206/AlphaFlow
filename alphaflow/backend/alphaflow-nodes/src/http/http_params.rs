// src/http/http_params.rs

use serde::Deserialize;
use crate::node_type::NodeError;

/// HTTP 节点所需的配置参数
#[derive(Debug, Deserialize)]
pub struct HttpParams {
    /// 目标地址
    pub url: String,

    /// HTTP Method，默认为 GET
    #[serde(default = "default_method")]
    pub method: String,
}

/// 默认 method 为 GET
fn default_method() -> String {
    "GET".to_string()
}

impl HttpParams {
    /// 对输入的 url, method 做基本校验
    pub fn validate(&self) -> Result<(), NodeError> {
        if self.url.trim().is_empty() {
            return Err(NodeError::InvalidConfig("URL cannot be empty".to_owned()));
        }
        // 若要限制 method 在 [GET, POST, PUT, ...] 内，也可在这里检查
        Ok(())
    }
}