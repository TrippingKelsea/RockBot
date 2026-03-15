//! RockBot Core Framework
//!
//! This crate provides the core functionality for the RockBot AI agent framework,
//! including the gateway server, session management, and agent execution engine.
//!
//! Configuration, message, and error types are provided by `rockbot-config` and
//! re-exported here for backward compatibility.

pub mod config;
pub mod credential_bridge;
pub mod cron;
pub mod error;
pub mod gateway;
pub mod agent;
pub mod routing;
pub mod session;
pub mod skills;
pub mod message;
pub mod web_ui;
pub mod metrics;
pub mod hooks;
pub mod a2a;
pub mod acp;
pub mod guardrails;
pub mod trajectory;
pub mod indexer;
pub mod sandbox;
pub mod telemetry;
pub mod tokenizer;
pub mod orchestration;
pub mod slash_commands;
#[cfg(feature = "remote-exec")]
pub mod remote_exec;

pub use config::{
    Config, GatewayConfig, AgentConfig, ProvidersConfig, McpServerEntry,
    AnthropicProviderConfig, OpenAiProviderConfig, BedrockProviderConfig, OllamaProviderConfig,
    WorkflowDefinition, WorkflowNode, WorkflowEdge, EdgeCondition,
};
pub use credential_bridge::VaultCredentialAccessor;
pub use error::{RockBotError, Result};
pub use gateway::Gateway;
pub use agent::Agent;
pub use session::{Session, SessionManager};
pub use message::{Message, MessageContent, MessageMetadata, ContentPart};
pub use routing::{RoutingEngine, ResolvedAgentRoute, SessionScope, MatchedByType};
pub use skills::{SkillManager, Skill, SkillMetadata, SkillInvocationPolicy, SlashCommandInfo};
pub use cron::{CronJob, CronSchedule, CronPayload, CronScheduler, CronExecutor};
pub use hooks::{Hook, HookEvent, HookResult, HookRegistry};
pub use gateway::GatewayInvoker;
pub use guardrails::{Guardrail, GuardrailResult, GuardrailPipeline, PiiGuardrail, PromptInjectionGuardrail};
pub use trajectory::{Trajectory, TrajectoryEvent, TrajectoryEntry};
pub use telemetry::{TelemetryConfig, init_telemetry};
pub use orchestration::{SwarmBlackboard, WorkflowExecutor};
pub use agent::HandoffSignal;
