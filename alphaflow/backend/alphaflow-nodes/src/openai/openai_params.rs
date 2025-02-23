use serde::Deserialize;
use crate::node_type::NodeError;

/// OpenAI Chat API parameters
#[derive(Debug, Deserialize)]
pub struct OpenAiChatParams {
    pub api_key: String,
    
    #[serde(default)]
    pub base_url: String,
    
    pub prompt: String,
    
    #[serde(default = "default_model")]
    pub model: String,
    
    #[serde(default)]
    pub system_content: String,
    
    #[serde(default)]
    pub temperature: Option<f32>,
    
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// Default model selection
fn default_model() -> String {
    "gpt-4o mini".to_string()
}

impl OpenAiChatParams {
    /// Validate required parameters
    pub fn validate(&self) -> Result<(), NodeError> {
        if self.api_key.trim().is_empty() {
            return Err(NodeError::InvalidConfig("OpenAI api_key cannot be empty".into()));
        }
        if self.prompt.trim().is_empty() {
            return Err(NodeError::InvalidConfig("Prompt cannot be empty".into()));
        }
        if self.base_url.trim().is_empty() {
            return Err(NodeError::InvalidConfig("Base URL cannot be empty".into()));
        }
        Ok(())
    }
}
