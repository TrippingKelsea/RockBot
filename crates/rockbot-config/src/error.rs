//! Error types shared across the RockBot workspace.
//!
//! Domain-specific error enums live here so that leaf crates can reference
//! them without depending on heavy runtime crates.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Config file not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Invalid configuration: {message}")]
    Invalid { message: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Environment variable not found: {var}")]
    EnvVarNotFound { var: String },
}

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("Failed to bind to {host}:{port}")]
    BindFailed { host: String, port: u16 },

    #[error("WebSocket error: {message}")]
    WebSocket { message: String },

    #[error("Authentication failed")]
    AuthFailed,

    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {session_id}")]
    NotFound { session_id: String },

    #[error("Session already exists: {session_id}")]
    AlreadyExists { session_id: String },

    #[error("Session limit exceeded")]
    LimitExceeded,

    #[error("Invalid session state: {message}")]
    InvalidState { message: String },
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Agent not found: {agent_id}")]
    NotFound { agent_id: String },

    #[error("Agent execution failed: {message}")]
    ExecutionFailed { message: String },

    #[error("Context too large: {size} tokens")]
    ContextTooLarge { size: usize },

    #[error("Model error: {message}")]
    ModelError { message: String },
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool '{tool_name}' not found")]
    NotFound { tool_name: String },

    #[error("Invalid parameters for tool '{tool_name}': {details}")]
    InvalidParameters { tool_name: String, details: String },

    #[error("Execution failed: {message}")]
    ExecutionFailed { message: String },

    #[error("Capability required: {capability}")]
    CapabilityRequired { capability: String },

    #[error("Timeout after {seconds} seconds")]
    Timeout { seconds: u64 },
}

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Memory store not found: {store_id}")]
    StoreNotFound { store_id: String },

    #[error("Index error: {message}")]
    IndexError { message: String },

    #[error("Search failed: {message}")]
    SearchFailed { message: String },
}

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Access denied: {resource}")]
    AccessDenied { resource: String },

    #[error("Sandbox error: {message}")]
    SandboxError { message: String },

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Authorization failed")]
    AuthorizationFailed,
}
