use crate::node_type::{NodeType, NodeExecutionContext, NodeOutput, NodeError};
use crate::http::http_params::HttpParams;
use async_trait::async_trait;
use reqwest::Client;
use reqwest::Method;

/// HTTP节点：
/// - 解析 HttpParams (url, method)
/// - 发起真实 HTTP 请求 (reqwest)
/// - 返回响应的status与body
pub struct HttpHandler;

impl HttpHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeType for HttpHandler {
    fn name(&self) -> &str {
        "http"
    }

    fn display_name(&self) -> &str {
        "HTTP Node"
    }

    async fn execute(
        &self,
        ctx: &NodeExecutionContext
    ) -> Result<NodeOutput, NodeError> {
        // 1) 解析参数
        let params: HttpParams = serde_json::from_value(ctx.parameters.clone())
            .map_err(|e| NodeError::InvalidConfig(format!("Param parse error: {e}")))?;

        // 2) 校验
        params.validate()?;

        // 3) 发起 HTTP 请求 (reqwest)
        let client = Client::new();
        let method = params.method.to_uppercase().parse::<Method>()
            .map_err(|_| NodeError::InvalidConfig("Invalid HTTP method".to_string()))?;

        let resp = client
            .request(method, &params.url)
            .send()
            .await
            .map_err(|e| NodeError::ExecutionFailed(format!("HTTP request error: {e}")))?;

        let status = resp.status();
        let text_body = resp
            .text()
            .await
            .map_err(|e| NodeError::ExecutionFailed(format!("Read body error: {e}")))?;

        // 4) 封装响应到 JSON
        let result_json = serde_json::json!({
            "status": status.as_u16(),
            "body": text_body
        });

        // 5) 返回 NodeOutput
        Ok(NodeOutput {
            data: result_json
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_type::NodeExecutionContext;
    use serde_json::json;

    /// 测试正常场景: 使用 httpbin.org
    /// 若网络环境正常, 会返回 200 + 含 "args" / "test=123" 的body.
    #[tokio::test]
    async fn test_http_handler_ok() {
        let handler = HttpHandler::new();
        let ctx = NodeExecutionContext {
            // 指向 httpbin.org/get 并加个 test 参数
            parameters: json!({
                "url": "https://httpbin.org/get?test=123",
                "method": "GET"
            }),
            input_data: json!(null),
        };

        let output = handler.execute(&ctx).await.expect("execution should succeed");
        // output.data 形如 { "status": 200, "body": "...json string..." }

        let status = output.data["status"].as_u64().unwrap();
        assert_eq!(status, 200, "Expect 200 from httpbin for GET");

        let body_str = output.data["body"].as_str().unwrap();
        // 检查 body 是否包含 test=123
        assert!(body_str.contains("\"test\": \"123\""), "Should contain test=123 in response JSON");
    }

    // 测试缺少url的场景: 应该报 InvalidConfig
    #[tokio::test]
async fn test_http_handler_missing_url() {
    let handler = HttpHandler::new();
    let ctx = NodeExecutionContext {
        // 这里将 url 字段设为 "" 而不是删除这个字段
        // 确保 serde 能成功 parse HttpParams { url: "", method: "GET" }
        // 并在 validate() 中报 "URL cannot be empty"
        parameters: json!({
            "url": "",
            "method":"GET"
        }),
        input_data: json!(null),
    };

    let result = handler.execute(&ctx).await;
    assert!(result.is_err(), "Should fail because url is empty");
    let err = result.err().unwrap();
    match err {
        NodeError::InvalidConfig(msg) => {
            // 校验错误信息中含 "URL cannot be empty"
            assert!(
                msg.contains("URL cannot be empty"),
                "Expect URL empty error"
            );
            },
            _ => panic!("Expected NodeError::InvalidConfig"),
        }
    }
}