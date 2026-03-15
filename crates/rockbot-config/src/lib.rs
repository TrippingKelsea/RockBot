//! RockBot Configuration, Message, and Error types
//!
//! This is a leaf crate providing shared types used across the RockBot workspace.

pub mod config;
pub mod error;
pub mod message;

// Re-export primary types at crate root for convenience
pub use config::{
    Config, GatewayConfig, AgentConfig, AgentDefaults, AgentInstance, AgentToolConfig,
    McpServerEntry, ToolConfig, SecurityConfig, SandboxConfig, CapabilityConfig,
    FilesystemCapabilities, NetworkCapabilities, ProcessCapabilities,
    CredentialsConfig, ProvidersConfig,
    AnthropicProviderConfig, OpenAiProviderConfig, BedrockProviderConfig, OllamaProviderConfig,
    ConfigWatcher,
    WorkflowDefinition, WorkflowNode, WorkflowEdge, EdgeCondition,
};

pub use error::{
    ConfigError, GatewayError, SessionError, AgentError, ToolError, MemoryError, SecurityError,
};

pub use message::{
    Message, MessageId, MessageContent, MessageMetadata, MessageRole,
    ContentPart, ContentBlock, RichContent, TextFormatting,
    ToolResult, SystemLevel, Attachment, MessageBuilder,
};
