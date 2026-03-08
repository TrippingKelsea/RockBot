//! Anthropic Claude API provider
//!
//! Supports two authentication methods:
//! 1. **API Key** - Traditional `ANTHROPIC_API_KEY` from console.anthropic.com
//! 2. **Session Key** - OAuth tokens from Claude Code CLI (~/.claude/.credentials.json)
//!
//! Session key auth uses the same tokens as Claude Code, allowing you to use your
//! Claude subscription without a separate API key.

use crate::{
    ChatCompletionRequest, ChatCompletionResponse, Choice, CompletionStream, LlmError,
    LlmProvider, Message, MessageRole, ModelInfo, ProviderCapabilities, Result, ToolDefinition,
    Usage, StreamingChunk, StreamingChoice, StreamingDelta,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use futures_util::{Stream, StreamExt};
use std::pin::Pin;

/// Authentication method for Anthropic API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicAuth {
    /// Traditional API key authentication
    ApiKey {
        key: String,
    },
    /// OAuth session key (Claude Code style)
    SessionKey {
        access_token: String,
        refresh_token: String,
        expires_at: u64,
        #[serde(default)]
        scopes: Vec<String>,
    },
}

impl AnthropicAuth {
    /// Load from Claude Code credentials file (~/.claude/.credentials.json)
    pub fn from_claude_credentials() -> Result<Self> {
        let credentials_path = Self::claude_credentials_path()?;
        let content = std::fs::read_to_string(&credentials_path).map_err(|_| {
            LlmError::AuthenticationFailed
        })?;
        
        #[derive(Deserialize)]
        struct ClaudeCredentials {
            #[serde(rename = "claudeAiOauth")]
            claude_ai_oauth: Option<OAuthCredentials>,
        }
        
        #[derive(Deserialize)]
        struct OAuthCredentials {
            #[serde(rename = "accessToken")]
            access_token: String,
            #[serde(rename = "refreshToken")]
            refresh_token: String,
            #[serde(rename = "expiresAt")]
            expires_at: u64,
            #[serde(default)]
            scopes: Vec<String>,
        }
        
        let creds: ClaudeCredentials = serde_json::from_str(&content)?;
        let oauth = creds.claude_ai_oauth.ok_or(LlmError::AuthenticationFailed)?;
        
        Ok(Self::SessionKey {
            access_token: oauth.access_token,
            refresh_token: oauth.refresh_token,
            expires_at: oauth.expires_at,
            scopes: oauth.scopes,
        })
    }
    
    /// Get the Claude Code credentials file path
    fn claude_credentials_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or(LlmError::AuthenticationFailed)?;
        Ok(home.join(".claude").join(".credentials.json"))
    }
    
    /// Check if session key is expired (with 5 minute buffer)
    pub fn is_expired(&self) -> bool {
        match self {
            Self::ApiKey { .. } => false,
            Self::SessionKey { expires_at, .. } => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                // Add 5 minute buffer
                *expires_at < now + 300_000
            }
        }
    }
    
    /// Get the authorization header value
    pub fn auth_header(&self) -> (&'static str, String) {
        match self {
            Self::ApiKey { key } => ("x-api-key", key.clone()),
            Self::SessionKey { access_token, .. } => ("Authorization", format!("Bearer {}", access_token)),
        }
    }
}

/// Anthropic API provider with support for both API key and session key auth
pub struct AnthropicProvider {
    client: reqwest::Client,
    auth: Arc<RwLock<AnthropicAuth>>,
    base_url: String,
}

/// Anthropic API request format
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

/// Anthropic API response format
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicContent>,
    model: String,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct AnthropicError {
    #[serde(rename = "type")]
    error_type: String,
    error: AnthropicErrorDetail,
}

#[derive(Debug, Deserialize)]
struct AnthropicErrorDetail {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

/// OAuth token refresh response
#[derive(Debug, Deserialize)]
struct TokenRefreshResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

/// Anthropic streaming event types
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart {
        message: AnthropicStreamMessage,
    },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: AnthropicStreamContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: u32,
        delta: AnthropicStreamDelta,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop {
        index: u32,
    },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: AnthropicStreamMessageDelta,
        usage: Option<AnthropicUsage>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error {
        error: AnthropicErrorDetail,
    },
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamMessage {
    id: String,
    #[serde(rename = "type")]
    message_type: String,
    role: String,
    model: String,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicStreamContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicStreamDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamMessageDelta {
    stop_reason: Option<String>,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider from environment variable (API key)
    pub fn new() -> Result<Self> {
        let api_key = env::var("ANTHROPIC_API_KEY").map_err(|_| LlmError::AuthenticationFailed)?;

        Ok(Self {
            client: reqwest::Client::new(),
            auth: Arc::new(RwLock::new(AnthropicAuth::ApiKey { key: api_key })),
            base_url: "https://api.anthropic.com".to_string(),
        })
    }

    /// Create with explicit API key
    pub fn with_api_key(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            auth: Arc::new(RwLock::new(AnthropicAuth::ApiKey { key: api_key })),
            base_url: "https://api.anthropic.com".to_string(),
        }
    }
    
    /// Create with session key auth (Claude Code style)
    pub fn with_session_key(auth: AnthropicAuth) -> Self {
        Self {
            client: reqwest::Client::new(),
            auth: Arc::new(RwLock::new(auth)),
            base_url: "https://api.anthropic.com".to_string(),
        }
    }
    
    /// Create from Claude Code credentials file
    pub fn from_claude_credentials() -> Result<Self> {
        let auth = AnthropicAuth::from_claude_credentials()?;
        Ok(Self::with_session_key(auth))
    }
    
    /// Try to create provider, preferring session key over API key
    /// Priority: 1. Claude Code credentials, 2. ANTHROPIC_API_KEY env var
    pub fn auto() -> Result<Self> {
        // Try Claude Code credentials first
        if let Ok(provider) = Self::from_claude_credentials() {
            return Ok(provider);
        }
        
        // Fall back to API key
        Self::new()
    }

    /// Extract model name from full ID (e.g., "anthropic/claude-3-opus" -> "claude-3-opus")
    fn normalize_model(&self, model_id: &str) -> String {
        model_id
            .strip_prefix("anthropic/")
            .unwrap_or(model_id)
            .to_string()
    }
    
    /// Refresh the OAuth token if expired
    async fn refresh_token_if_needed(&self) -> Result<()> {
        let auth = self.auth.read().await;
        
        if let AnthropicAuth::SessionKey { refresh_token, expires_at, .. } = &*auth {
            if !auth.is_expired() {
                return Ok(());
            }
            
            let refresh_token = refresh_token.clone();
            let _expires_at = *expires_at;
            drop(auth); // Release read lock
            
            // Perform token refresh
            let response = self
                .client
                .post("https://console.anthropic.com/v1/oauth/token")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .form(&[
                    ("grant_type", "refresh_token"),
                    ("refresh_token", &refresh_token),
                ])
                .send()
                .await?;
            
            if !response.status().is_success() {
                return Err(LlmError::AuthenticationFailed);
            }
            
            let token_response: TokenRefreshResponse = response.json().await?;
            
            // Update stored auth
            let mut auth = self.auth.write().await;
            let new_expires_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64 + (token_response.expires_in * 1000);
            
            if let AnthropicAuth::SessionKey { 
                access_token, 
                refresh_token: stored_refresh, 
                expires_at,
                ..
            } = &mut *auth {
                *access_token = token_response.access_token;
                if let Some(new_refresh) = token_response.refresh_token {
                    *stored_refresh = new_refresh;
                }
                *expires_at = new_expires_at;
            }
            
            // Persist updated credentials back to Claude credentials file
            self.persist_credentials(&auth).await?;
        }
        
        Ok(())
    }
    
    /// Persist updated credentials to Claude Code's credentials file
    async fn persist_credentials(&self, auth: &AnthropicAuth) -> Result<()> {
        if let AnthropicAuth::SessionKey { access_token, refresh_token, expires_at, scopes } = auth {
            let credentials_path = AnthropicAuth::claude_credentials_path()?;
            
            // Read existing file to preserve other fields
            let content = tokio::fs::read_to_string(&credentials_path).await.unwrap_or_default();
            let mut creds: serde_json::Value = serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
            
            // Update OAuth section
            creds["claudeAiOauth"] = serde_json::json!({
                "accessToken": access_token,
                "refreshToken": refresh_token,
                "expiresAt": expires_at,
                "scopes": scopes,
            });
            
            // Write back
            let updated = serde_json::to_string_pretty(&creds)?;
            tokio::fs::write(&credentials_path, updated).await.map_err(|_| {
                LlmError::ApiError {
                    message: "Failed to persist updated credentials".to_string(),
                }
            })?;
        }
        
        Ok(())
    }
    
    /// Get current auth type for display/debugging
    pub async fn auth_type(&self) -> &'static str {
        let auth = self.auth.read().await;
        match &*auth {
            AnthropicAuth::ApiKey { .. } => "api_key",
            AnthropicAuth::SessionKey { .. } => "session_key",
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn id(&self) -> &str {
        "anthropic"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
            supports_embeddings: false,
            max_tokens: Some(8192),
            context_window: 200000,
        }
    }

    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        // Refresh token if needed
        self.refresh_token_if_needed().await?;
        
        let model = self.normalize_model(&request.model);

        // Extract system message and convert others
        let mut system_message: Option<String> = None;
        let messages: Vec<AnthropicMessage> = request
            .messages
            .iter()
            .filter_map(|m| {
                match m.role {
                    MessageRole::System => {
                        system_message = Some(m.content.clone());
                        None
                    }
                    MessageRole::User => Some(AnthropicMessage {
                        role: "user".to_string(),
                        content: m.content.clone(),
                    }),
                    MessageRole::Assistant => Some(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: m.content.clone(),
                    }),
                    MessageRole::Tool => Some(AnthropicMessage {
                        role: "user".to_string(), // Tool results come as user messages in Anthropic
                        content: m.content.clone(),
                    }),
                }
            })
            .collect();

        // Convert tools
        let tools: Option<Vec<AnthropicTool>> = request.tools.map(|t| {
            t.into_iter()
                .map(|tool| AnthropicTool {
                    name: tool.name,
                    description: tool.description,
                    input_schema: tool.parameters,
                })
                .collect()
        });

        let api_request = AnthropicRequest {
            model: model.clone(),
            max_tokens: request.max_tokens.unwrap_or(4096),
            messages,
            system: system_message,
            tools,
            temperature: request.temperature,
        };

        // Get auth header
        let auth = self.auth.read().await;
        let (header_name, header_value) = auth.auth_header();
        
        let mut req_builder = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header(header_name, header_value)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json");
        
        // Add beta header for session key auth (some features require it)
        if matches!(&*auth, AnthropicAuth::SessionKey { .. }) {
            req_builder = req_builder.header("anthropic-beta", "max-tokens-3-5-sonnet-2024-07-15");
        }
        
        drop(auth); // Release lock before await

        let response = req_builder
            .json(&api_request)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            // Try to parse error response
            if let Ok(error) = serde_json::from_str::<AnthropicError>(&body) {
                return Err(LlmError::ApiError {
                    message: format!("{}: {}", error.error.error_type, error.error.message),
                });
            }
            return Err(LlmError::ApiError {
                message: format!("HTTP {}: {}", status, body),
            });
        }

        let api_response: AnthropicResponse = serde_json::from_str(&body)?;

        // Convert response to our format
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for block in api_response.content {
            match block {
                AnthropicContent::Text { text } => {
                    content.push_str(&text);
                }
                AnthropicContent::ToolUse { id, name, input } => {
                    tool_calls.push(crate::ToolCall {
                        id,
                        r#type: "function".to_string(),
                        function: crate::FunctionCall {
                            name,
                            arguments: serde_json::to_string(&input).unwrap_or_default(),
                        },
                    });
                }
            }
        }

        Ok(ChatCompletionResponse {
            id: api_response.id,
            object: "chat.completion".to_string(),
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: MessageRole::Assistant,
                    content,
                    tool_calls: if tool_calls.is_empty() {
                        None
                    } else {
                        Some(tool_calls)
                    },
                },
                finish_reason: api_response.stop_reason.unwrap_or_else(|| "stop".to_string()),
            }],
            usage: Usage {
                prompt_tokens: api_response.usage.input_tokens,
                completion_tokens: api_response.usage.output_tokens,
                total_tokens: api_response.usage.input_tokens + api_response.usage.output_tokens,
            },
        })
    }

    async fn stream_completion(&self, request: ChatCompletionRequest) -> Result<CompletionStream> {
        // Refresh token if needed
        self.refresh_token_if_needed().await?;
        
        let model = self.normalize_model(&request.model);

        // Extract system message and convert others
        let mut system_message: Option<String> = None;
        let messages: Vec<AnthropicMessage> = request
            .messages
            .iter()
            .filter_map(|m| {
                match m.role {
                    MessageRole::System => {
                        system_message = Some(m.content.clone());
                        None
                    }
                    MessageRole::User => Some(AnthropicMessage {
                        role: "user".to_string(),
                        content: m.content.clone(),
                    }),
                    MessageRole::Assistant => Some(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: m.content.clone(),
                    }),
                    MessageRole::Tool => Some(AnthropicMessage {
                        role: "user".to_string(), // Tool results come as user messages in Anthropic
                        content: m.content.clone(),
                    }),
                }
            })
            .collect();

        // Convert tools
        let tools: Option<Vec<AnthropicTool>> = request.tools.map(|t| {
            t.into_iter()
                .map(|tool| AnthropicTool {
                    name: tool.name,
                    description: tool.description,
                    input_schema: tool.parameters,
                })
                .collect()
        });

        let mut api_request = AnthropicRequest {
            model: model.clone(),
            max_tokens: request.max_tokens.unwrap_or(4096),
            messages,
            system: system_message,
            tools,
            temperature: request.temperature,
        };

        // For streaming, we need to add stream: true to the request
        let mut request_json = serde_json::to_value(&api_request)?;
        request_json["stream"] = serde_json::Value::Bool(true);

        // Get auth header
        let auth = self.auth.read().await;
        let (header_name, header_value) = auth.auth_header();
        
        let mut req_builder = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header(header_name, header_value)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json");
        
        // Add beta header for session key auth (some features require it)
        if matches!(&*auth, AnthropicAuth::SessionKey { .. }) {
            req_builder = req_builder.header("anthropic-beta", "max-tokens-3-5-sonnet-2024-07-15");
        }
        
        drop(auth); // Release lock before await

        let response = req_builder
            .json(&request_json)
            .send()
            .await?;

        let status = response.status();
        
        if !status.is_success() {
            let body = response.text().await?;
            // Try to parse error response
            if let Ok(error) = serde_json::from_str::<AnthropicError>(&body) {
                return Err(LlmError::ApiError {
                    message: format!("{}: {}", error.error.error_type, error.error.message),
                });
            }
            return Err(LlmError::ApiError {
                message: format!("HTTP {}: {}", status, body),
            });
        }

        // Implement proper SSE streaming
        let stream = async_stream::stream! {
            use futures_util::StreamExt;
            
            // Get response stream
            let mut byte_stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut message_id: Option<String> = None;
            let mut current_content = String::new();
            
            while let Some(chunk) = byte_stream.next().await {
                let chunk = match chunk {
                    Ok(chunk) => chunk,
                    Err(e) => {
                        yield Err(LlmError::Request(e));
                        return;
                    }
                };
                
                let chunk_str = String::from_utf8_lossy(&chunk);
                buffer.push_str(&chunk_str);
                
                // Process complete SSE events
                while let Some(event_end) = buffer.find("\n\n") {
                    let event_data = buffer[..event_end].to_string();
                    buffer = buffer[event_end + 2..].to_string();
                    
                    // Parse SSE event
                    if let Some(data) = parse_sse_event(&event_data) {
                        match handle_anthropic_stream_event(&data, &mut message_id, &mut current_content, &model) {
                            Ok(Some(chunk)) => yield Ok(chunk),
                            Ok(None) => continue, // Event handled, but no chunk to yield
                            Err(e) => {
                                yield Err(e);
                                return;
                            }
                        }
                    }
                }
            }
            
            // Send final chunk if we have content
            if !current_content.is_empty() {
                let final_chunk = StreamingChunk {
                    id: message_id.unwrap_or_else(|| format!("stream-{}", uuid::Uuid::new_v4())),
                    object: "chat.completion.chunk".to_string(),
                    created: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    model: model.clone(),
                    choices: vec![StreamingChoice {
                        index: 0,
                        delta: StreamingDelta {
                            role: None,
                            content: None,
                            tool_calls: None,
                        },
                        finish_reason: Some("stop".to_string()),
                    }],
                };
                yield Ok(final_chunk);
            }
        };
        
        Ok(Box::pin(stream))
    }

    async fn generate_embedding(&self, _text: &str) -> Result<Vec<f32>> {
        Err(LlmError::ApiError {
            message: "Anthropic does not support embeddings".to_string(),
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        // Anthropic doesn't have a model listing API, return known models
        Ok(vec![
            ModelInfo {
                id: "claude-opus-4-20250514".to_string(),
                name: "Claude Opus 4".to_string(),
                description: "Most capable Claude model".to_string(),
                context_window: 200000,
                max_output_tokens: Some(8192),
                supports_tools: true,
                supports_vision: true,
            },
            ModelInfo {
                id: "claude-sonnet-4-20250514".to_string(),
                name: "Claude Sonnet 4".to_string(),
                description: "Balanced performance and speed".to_string(),
                context_window: 200000,
                max_output_tokens: Some(8192),
                supports_tools: true,
                supports_vision: true,
            },
            ModelInfo {
                id: "claude-3-5-haiku-latest".to_string(),
                name: "Claude 3.5 Haiku".to_string(),
                description: "Fast and efficient".to_string(),
                context_window: 200000,
                max_output_tokens: Some(8192),
                supports_tools: true,
                supports_vision: true,
            },
        ])
    }

    async fn get_model_info(&self, model_id: &str) -> Result<ModelInfo> {
        let models = self.list_models().await?;
        let normalized = self.normalize_model(model_id);

        models
            .into_iter()
            .find(|m| m.id == normalized || m.id == model_id)
            .ok_or_else(|| LlmError::ModelNotFound {
                model: model_id.to_string(),
            })
    }
}

/// Parse Server-Sent Events (SSE) format
fn parse_sse_event(event_data: &str) -> Option<String> {
    for line in event_data.lines() {
        if line.starts_with("data: ") {
            let data = &line[6..]; // Skip "data: "
            if data == "[DONE]" {
                return None; // End of stream
            }
            return Some(data.to_string());
        }
    }
    None
}

/// Handle Anthropic-specific streaming events
fn handle_anthropic_stream_event(
    data: &str,
    message_id: &mut Option<String>,
    current_content: &mut String,
    model: &str,
) -> std::result::Result<Option<StreamingChunk>, LlmError> {
    let event: AnthropicStreamEvent = serde_json::from_str(data)
        .map_err(|e| LlmError::ApiError {
            message: format!("Failed to parse streaming event: {}", e),
        })?;
    
    match event {
        AnthropicStreamEvent::MessageStart { message } => {
            *message_id = Some(message.id);
            Ok(None) // No content chunk yet
        }
        AnthropicStreamEvent::ContentBlockDelta { delta, .. } => {
            match delta {
                AnthropicStreamDelta::TextDelta { text } => {
                    current_content.push_str(&text);
                    
                    let chunk = StreamingChunk {
                        id: message_id.as_ref()
                            .unwrap_or(&format!("stream-{}", uuid::Uuid::new_v4()))
                            .clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        model: model.to_string(),
                        choices: vec![StreamingChoice {
                            index: 0,
                            delta: StreamingDelta {
                                role: None,
                                content: Some(text),
                                tool_calls: None,
                            },
                            finish_reason: None,
                        }],
                    };
                    Ok(Some(chunk))
                }
                AnthropicStreamDelta::InputJsonDelta { partial_json } => {
                    // Handle tool call streaming
                    // For now, accumulate the JSON
                    // TODO: Properly handle partial JSON for tool calls
                    tracing::debug!("Received partial JSON: {}", partial_json);
                    Ok(None)
                }
            }
        }
        AnthropicStreamEvent::MessageDelta { .. } => {
            // Message metadata changes (like stop reason)
            Ok(None)
        }
        AnthropicStreamEvent::MessageStop => {
            // End of message
            Ok(None)
        }
        _ => {
            // Other event types (ContentBlockStart, ContentBlockStop, etc.)
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_model() {
        let provider = AnthropicProvider {
            client: reqwest::Client::new(),
            auth: Arc::new(RwLock::new(AnthropicAuth::ApiKey { key: "test".to_string() })),
            base_url: "https://api.anthropic.com".to_string(),
        };

        // Need to use a runtime for async tests
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            assert_eq!(
                provider.normalize_model("anthropic/claude-3-opus"),
                "claude-3-opus"
            );
            assert_eq!(provider.normalize_model("claude-3-opus"), "claude-3-opus");
        });
    }
    
    #[test]
    fn test_auth_expiration() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // Expired token
        let expired = AnthropicAuth::SessionKey {
            access_token: "test".to_string(),
            refresh_token: "test".to_string(),
            expires_at: now - 1000,
            scopes: vec![],
        };
        assert!(expired.is_expired());
        
        // Valid token
        let valid = AnthropicAuth::SessionKey {
            access_token: "test".to_string(),
            refresh_token: "test".to_string(),
            expires_at: now + 600_000, // 10 minutes from now
            scopes: vec![],
        };
        assert!(!valid.is_expired());
        
        // API key never expires
        let api_key = AnthropicAuth::ApiKey { key: "test".to_string() };
        assert!(!api_key.is_expired());
    }
}
