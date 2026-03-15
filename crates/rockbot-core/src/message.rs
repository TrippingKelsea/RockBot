//! Re-export of message types from `rockbot-config`, plus LLM conversion helpers.

pub use rockbot_config::message::*;

use crate::error::AgentError;

/// Create a Message from an LLM message.
///
/// This lives here (not in `rockbot-config`) because it depends on `rockbot_llm` types.
pub fn from_llm_message(
    llm_message: rockbot_llm::Message,
    session_id: &str,
    agent_id: &str,
) -> Result<Message, AgentError> {
    let role = match llm_message.role {
        rockbot_llm::MessageRole::User => MessageRole::User,
        rockbot_llm::MessageRole::Assistant => MessageRole::Assistant,
        rockbot_llm::MessageRole::System => MessageRole::System,
        rockbot_llm::MessageRole::Tool => MessageRole::Tool,
    };

    let content = MessageContent::Text { text: llm_message.content };

    let mut msg = Message::new(content)
        .with_session_id(session_id)
        .with_agent_id(agent_id)
        .with_role(role);

    if let Some(ref tool_calls) = llm_message.tool_calls {
        if !tool_calls.is_empty() {
            if let Ok(tc_json) = serde_json::to_value(tool_calls) {
                msg.metadata.extra.insert("tool_calls".to_string(), tc_json);
            }
        }
    }

    if let Some(ref tool_call_id) = llm_message.tool_call_id {
        msg.metadata.extra.insert("tool_call_id".to_string(), serde_json::Value::String(tool_call_id.clone()));
    }

    Ok(msg)
}
