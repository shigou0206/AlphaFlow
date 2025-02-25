// src/openai/openai_handler.rs

use crate::node_type::{NodeType, NodeExecutionContext, NodeOutput, NodeError};
use crate::openai::openai_params::OpenAiChatParams;
use async_trait::async_trait;
use serde_json::json;
use reqwest::Client;

/// OpenAiChatHandler 实现了基于 OpenAI Chat Completions 接口的节点功能。
/// 它解析 OpenAiChatParams，从 context.parameters 中获取配置，
/// 并构造一个包含 system 和 user 消息的请求体，然后调用 OpenAI API。
pub struct OpenAiChatHandler;

impl OpenAiChatHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeType for OpenAiChatHandler {
    fn name(&self) -> &str {
        "openai"
    }

    fn display_name(&self) -> &str {
        "OpenAI Node"
    }

    async fn execute(
        &self, 
        ctx: &NodeExecutionContext
    ) -> Result<NodeOutput, NodeError> {
        // 1) 解析和校验参数
        let params: OpenAiChatParams = serde_json::from_value(ctx.parameters.clone())
            .map_err(|e| NodeError::InvalidConfig(format!("Parameter parsing error: {e}")))?;
        params.validate()?;

        // 2) 构造请求体：如果 system_content 非空，则加入 system 消息
        let mut messages = Vec::new();
        if !params.system_content.trim().is_empty() {
            messages.push(json!({
                "role": "system",
                "content": params.system_content
            }));
        }
        messages.push(json!({
            "role": "user",
            "content": params.prompt
        }));

        let body = json!({
            "model": params.model,
            "messages": messages,
            "max_tokens": params.max_tokens.unwrap_or(100),
            "temperature": params.temperature.unwrap_or(0.7),
        });

        // 3) 拼接请求 URL
        let url = format!("{}/chat/completions", params.base_url);

        // 4) 发起 HTTP 请求
        let client = Client::new();
        let resp = client
            .post(&url)
            .bearer_auth(params.api_key.trim())
            .json(&body)
            .send()
            .await
            .map_err(|e| NodeError::ExecutionFailed(format!("OpenAI request error: {e}")))?;

        if !resp.status().is_success() {
            let code = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(NodeError::ExecutionFailed(
                format!("OpenAI responded with status={} body={}", code, text)
            ));
        }

        // 5) 解析返回的 JSON并提取 assistant 消息
        let resp_json: serde_json::Value = resp.json().await
            .map_err(|e| NodeError::ExecutionFailed(format!("JSON parse error: {e}")))?;

        let assistant_message = resp_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        // 6) 返回 NodeOutput，包含模型信息和响应文本
        let result_json = json!({
            "model": params.model,
            "response": assistant_message
        });

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
    use std::env;
    use dotenv::dotenv;

    #[tokio::test]
    async fn test_openai_chat_ok() {
        // 需要在测试环境中设置 OPENAI_API_KEY 环境变量
        dotenv().ok(); // 加载 .env 文件

        let api_key = match env::var("OPENAI_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                eprintln!("Skipping test: OPENAI_API_KEY is not set");
                return;
            }
        };
        
        let handler = OpenAiChatHandler::new();
        let ctx = NodeExecutionContext {
            parameters: json!({
                "api_key": api_key,
                "prompt": "Hello from alphaflow-nodes rust test!",
                "model": "gpt-4o-mini",  
                "temperature": 0.7,
                "max_tokens": 30,
                "base_url": "https://api.openai.com/v1",
                "system_content": "You are a helpful assistant."
            }),
            input_data: json!(null),
            globals: json!(null),
            env: json!(null),
            pin_data: None,
        };

        let result = handler.execute(&ctx).await;
        match result {
            Ok(output) => {
                let model = output.data["model"].as_str().unwrap_or("");
                let content = output.data["response"].as_str().unwrap_or("");
                println!("model={} response={}", model, content);
                assert!(!content.is_empty(), "The AI response text should not be empty");
            }
            Err(e) => {
                panic!("OpenAI request should succeed, but got error: {e}");
            }
        }
    }

    #[tokio::test]
    async fn test_openai_chat_empty_prompt() {
        let handler = OpenAiChatHandler::new();
        let ctx = NodeExecutionContext {
            parameters: json!({
                "api_key": "TEST_KEY",
                "prompt": "",
                "model": "gpt-4",
                "base_url": "https://api.openai.com/v1"
            }),
            input_data: json!(null),
            globals: json!(null),
            env: json!(null),
            pin_data: None,
        };

        let result = handler.execute(&ctx).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        match err {
            NodeError::InvalidConfig(msg) => {
                assert!(msg.contains("Prompt cannot be empty"));
            },
            _ => panic!("Expected NodeError::InvalidConfig"),
        }
    }
}