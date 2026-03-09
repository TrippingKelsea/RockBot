//! Krabbykrus Core Framework
//!
//! This crate provides the core functionality for the Krabbykrus AI agent framework,
//! including the gateway server, session management, and agent execution engine.
//!
//! # Modules
//!
//! - [`config`] - Configuration loading and validation
//! - [`gateway`] - HTTP/WebSocket server
//! - [`agent`] - Agent execution engine
//! - [`session`] - Session persistence
//! - [`message`] - Message types
//! - [`credential_bridge`] - Credential injection for tools
//! - [`web_ui`] - Embedded web dashboard

pub mod config;
pub mod credential_bridge;
pub mod error;
pub mod gateway;
pub mod agent;
pub mod session;
pub mod message;
pub mod web_ui;

pub use config::{
    Config, GatewayConfig, AgentConfig, ProvidersConfig, 
    AnthropicProviderConfig, OpenAiProviderConfig, BedrockProviderConfig, OllamaProviderConfig
};
pub use credential_bridge::VaultCredentialAccessor;
pub use error::{KrabbykrusError, Result};
pub use gateway::Gateway;
pub use agent::Agent;
pub use session::{Session, SessionManager};
pub use message::{Message, MessageContent, MessageMetadata};