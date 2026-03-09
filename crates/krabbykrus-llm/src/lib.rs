//! LLM provider abstraction for Krabbykrus
//!
//! This crate provides a unified interface for multiple LLM providers:
//!
//! - **Anthropic** - Claude models via Claude Code SDK (OAuth only)
//! - **OpenAI** - GPT-4, o1, and other OpenAI models
//! - **Mock** - For development and testing
//!
//! # Authentication
//!
//! ## Anthropic (OAuth via Claude Code)
//! - Requires Claude Code CLI: `npm install -g @anthropic-ai/claude-code`
//! - Run `claude` to authenticate (OAuth flow)
//! - Credentials stored at `~/.claude/.credentials.json`
//!
//! ## OpenAI
//! - Set `OPENAI_API_KEY` environment variable
//!
//! # Example
//!
//! ```no_run
//! use krabbykrus_llm::{LlmProviderRegistry, ChatCompletionRequest, Message, MessageRole};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let registry = LlmProviderRegistry::new().await?;
//!     
//!     let provider = registry.get_provider_for_model("anthropic/claude-sonnet-4-20250514").await?;
//!     
//!     let request = ChatCompletionRequest {
//!         model: "claude-sonnet-4-20250514".to_string(),
//!         messages: vec![Message {
//!             role: MessageRole::User,
//!             content: "Hello!".to_string(),
//!             tool_calls: None,
//!         }],
//!         tools: None,
//!         temperature: Some(0.7),
//!         max_tokens: Some(1000),
//!         stream: false,
//!     };
//!     
//!     let response = provider.chat_completion(request).await?;
//!     println!("{}", response.choices[0].message.content);
//!     
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use futures_util::Stream;
use std::pin::Pin;

pub mod anthropic;
pub mod openai;

pub use anthropic::AnthropicProvider;
pub use openai::OpenAiProvider;

/// LLM provider errors
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("Model not found: {model}")]
    ModelNotFound { model: String },
    
    #[error("API error: {message}")]
    ApiError { message: String },
    
    #[error("Authentication failed - run 'claude' to authenticate")]
    AuthenticationFailed,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type for LLM operations
pub type Result<T> = std::result::Result<T, LlmError>;

/// LLM provider trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider identifier
    fn id(&self) -> &str;
    
    /// Provider capabilities
    fn capabilities(&self) -> ProviderCapabilities;
    
    /// Chat completion
    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse>;
    
    /// Streaming chat completion
    async fn stream_completion(&self, request: ChatCompletionRequest) -> Result<CompletionStream>;
    
    /// Generate text embeddings
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>>;
    
    /// List available models
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    
    /// Get model information
    async fn get_model_info(&self, model_id: &str) -> Result<ModelInfo>;
}

/// Provider capabilities
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_embeddings: bool,
    pub max_tokens: Option<u32>,
    pub context_window: u32,
}

/// Chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

/// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

/// Message in a chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool call in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Choice in a chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: String,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// Streaming completion chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamingChoice>,
}

/// Streaming choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChoice {
    pub index: u32,
    pub delta: StreamingDelta,
    pub finish_reason: Option<String>,
}

/// Streaming delta (incremental content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingDelta {
    pub role: Option<MessageRole>,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Streaming completion response
pub type CompletionStream = Pin<Box<dyn Stream<Item = Result<StreamingChunk>> + Send>>;

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub context_window: u32,
    pub max_output_tokens: Option<u32>,
    pub supports_tools: bool,
    pub supports_vision: bool,
}

/// LLM provider registry
pub struct LlmProviderRegistry {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
    model_mapping: HashMap<String, String>,
}

impl LlmProviderRegistry {
    /// Create a new provider registry
    pub async fn new() -> Result<Self> {
        let mut registry = Self {
            providers: HashMap::new(),
            model_mapping: HashMap::new(),
        };
        
        registry.register_builtin_providers().await?;
        Ok(registry)
    }
    
    /// Register built-in providers
    async fn register_builtin_providers(&mut self) -> Result<()> {
        // Register mock provider for development
        let mock_provider = Arc::new(MockLlmProvider::new());
        self.register_provider(mock_provider).await;
        
        // Register Anthropic provider if Claude Code credentials exist
        if AnthropicProvider::has_credentials() {
            if let Ok(anthropic) = AnthropicProvider::new() {
                self.register_provider(Arc::new(anthropic)).await;
            }
        }
        
        // Register OpenAI provider if API key is available
        if let Ok(openai) = OpenAiProvider::new() {
            self.register_provider(Arc::new(openai)).await;
        }
        
        Ok(())
    }
    
    /// Register a provider
    pub async fn register_provider(&mut self, provider: Arc<dyn LlmProvider>) {
        let provider_id = provider.id().to_string();
        self.providers.insert(provider_id.clone(), provider.clone());
        
        // Register model mappings
        if let Ok(models) = provider.list_models().await {
            for model in models {
                self.model_mapping.insert(model.id, provider_id.clone());
            }
        }
    }
    
    /// Get provider for a model
    pub async fn get_provider_for_model(&self, model_id: &str) -> Result<Arc<dyn LlmProvider>> {
        // Extract provider from model ID (e.g., "anthropic/claude-3-opus")
        let provider_id = if let Some(slash_pos) = model_id.find('/') {
            &model_id[..slash_pos]
        } else if let Some(provider_id) = self.model_mapping.get(model_id) {
            provider_id
        } else {
            "mock"
        };
        
        self.providers.get(provider_id)
            .cloned()
            .ok_or_else(|| {
                let hint = match provider_id {
                    "anthropic" => " (run 'claude' to authenticate)",
                    "openai" => " (set OPENAI_API_KEY environment variable)",
                    _ => "",
                };
                LlmError::ApiError {
                    message: format!("Provider '{}' not available for model '{}'{}", provider_id, model_id, hint),
                }
            })
    }

    /// List available providers
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
    
    /// Check if Anthropic (Claude) is available
    pub fn has_anthropic(&self) -> bool {
        self.providers.contains_key("anthropic")
    }
}

/// Mock LLM provider for development and testing
pub struct MockLlmProvider;

impl MockLlmProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    fn id(&self) -> &str {
        "mock"
    }
    
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: false,
            supports_tools: true,
            supports_vision: false,
            supports_embeddings: false,
            max_tokens: Some(4000),
            context_window: 128000,
        }
    }
    
    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let response_content = format!(
            "Mock response to: {}",
            request.messages.last()
                .map(|m| m.content.chars().take(50).collect::<String>())
                .unwrap_or_default()
        );
        
        Ok(ChatCompletionResponse {
            id: format!("mock-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            model: request.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: MessageRole::Assistant,
                    content: response_content,
                    tool_calls: None,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Usage {
                prompt_tokens: 50,
                completion_tokens: 25,
                total_tokens: 75,
            },
        })
    }
    
    async fn stream_completion(&self, request: ChatCompletionRequest) -> Result<CompletionStream> {
        let response_content = format!(
            "Mock streamed response to: {}",
            request.messages.last()
                .map(|m| m.content.chars().take(50).collect::<String>())
                .unwrap_or_default()
        );
        
        let stream = async_stream::stream! {
            let words: Vec<&str> = response_content.split_whitespace().collect();
            
            for (i, word) in words.iter().enumerate() {
                let chunk = StreamingChunk {
                    id: format!("mock-stream-{}", uuid::Uuid::new_v4()),
                    object: "chat.completion.chunk".to_string(),
                    created: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    model: request.model.clone(),
                    choices: vec![StreamingChoice {
                        index: 0,
                        delta: StreamingDelta {
                            role: if i == 0 { Some(MessageRole::Assistant) } else { None },
                            content: Some(format!("{} ", word)),
                            tool_calls: None,
                        },
                        finish_reason: if i == words.len() - 1 { Some("stop".to_string()) } else { None },
                    }],
                };
                
                yield Ok(chunk);
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        };
        
        Ok(Box::pin(stream))
    }
    
    async fn generate_embedding(&self, _text: &str) -> Result<Vec<f32>> {
        Ok(vec![0.1, 0.2, 0.3, 0.4, 0.5])
    }
    
    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "mock-model".to_string(),
                name: "Mock Model".to_string(),
                description: "A mock model for development".to_string(),
                context_window: 128000,
                max_output_tokens: Some(4000),
                supports_tools: true,
                supports_vision: false,
            },
        ])
    }
    
    async fn get_model_info(&self, model_id: &str) -> Result<ModelInfo> {
        if model_id == "mock-model" {
            Ok(ModelInfo {
                id: model_id.to_string(),
                name: "Mock Model".to_string(),
                description: "A mock model for development".to_string(),
                context_window: 128000,
                max_output_tokens: Some(4000),
                supports_tools: true,
                supports_vision: false,
            })
        } else {
            Err(LlmError::ModelNotFound {
                model: model_id.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockLlmProvider::new();
        
        let request = ChatCompletionRequest {
            model: "mock-model".to_string(),
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello, world!".to_string(),
                tool_calls: None,
            }],
            tools: None,
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: false,
        };
        
        let response = provider.chat_completion(request).await.unwrap();
        assert_eq!(response.choices.len(), 1);
        assert!(!response.choices[0].message.content.is_empty());
    }
    
    #[tokio::test]
    async fn test_provider_registry() {
        let registry = LlmProviderRegistry::new().await.unwrap();
        let provider = registry.get_provider_for_model("mock-model").await.unwrap();
        assert_eq!(provider.id(), "mock");
    }
}
