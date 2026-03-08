//! Models/Providers component
//!
//! Shows LLM provider configuration status with clear guidance on
//! how to configure credentials via vault or environment variables.
//!
//! ## Anthropic Authentication
//!
//! Anthropic supports two authentication methods:
//! 1. **API Key** - Traditional key from console.anthropic.com
//! 2. **Session Key** - OAuth tokens from Claude Code CLI
//!
//! Session key auth uses the same tokens as Claude Code, allowing use of
//! your Claude subscription without a separate API key.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::tui::effects::{self, palette, EffectState};
use crate::tui::state::AppState;

/// Authentication type for a provider
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthType {
    /// Traditional API key
    ApiKey,
    /// OAuth session key (Claude Code style)
    SessionKey,
    /// No authentication needed (e.g., local Ollama)
    None,
    /// AWS credentials (access key + secret + region)
    AwsCredentials,
}

impl AuthType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ApiKey => "API Key",
            Self::SessionKey => "Session Key",
            Self::None => "None",
            Self::AwsCredentials => "AWS Credentials",
        }
    }
}

/// Known LLM providers with their configuration details
const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo {
        name: "Anthropic",
        env_var: "ANTHROPIC_API_KEY",
        vault_name: "anthropic",
        description: "Claude models (Opus 4, Sonnet 4, Haiku 3.5)",
        api_url: "https://api.anthropic.com",
        docs_url: "https://console.anthropic.com/",
        auth_types: &[AuthType::ApiKey, AuthType::SessionKey],
        session_key_hint: Some("Uses Claude Code credentials (~/.claude/.credentials.json)"),
    },
    ProviderInfo {
        name: "OpenAI",
        env_var: "OPENAI_API_KEY",
        vault_name: "openai",
        description: "GPT-4, GPT-4o, o1 models",
        api_url: "https://api.openai.com",
        docs_url: "https://platform.openai.com/api-keys",
        auth_types: &[AuthType::ApiKey],
        session_key_hint: None,
    },
    ProviderInfo {
        name: "Ollama",
        env_var: "",
        vault_name: "ollama",
        description: "Local models (no API key needed)",
        api_url: "http://localhost:11434",
        docs_url: "https://ollama.ai",
        auth_types: &[AuthType::None],
        session_key_hint: None,
    },
    ProviderInfo {
        name: "AWS Bedrock",
        env_var: "AWS_ACCESS_KEY_ID",
        vault_name: "bedrock",
        description: "Claude, Llama, Titan via AWS",
        api_url: "bedrock-runtime.{region}.amazonaws.com",
        docs_url: "https://aws.amazon.com/bedrock/",
        auth_types: &[AuthType::AwsCredentials],
        session_key_hint: None,
    },
    ProviderInfo {
        name: "Google AI",
        env_var: "GOOGLE_API_KEY",
        vault_name: "google",
        description: "Gemini models",
        api_url: "https://generativelanguage.googleapis.com",
        docs_url: "https://aistudio.google.com/apikey",
        auth_types: &[AuthType::ApiKey],
        session_key_hint: None,
    },
];

struct ProviderInfo {
    name: &'static str,
    env_var: &'static str,
    vault_name: &'static str,
    description: &'static str,
    api_url: &'static str,
    docs_url: &'static str,
    /// Supported authentication types (first is default)
    auth_types: &'static [AuthType],
    /// Hint text for session key auth (if supported)
    session_key_hint: Option<&'static str>,
}

/// Check if Claude Code credentials exist
fn has_claude_code_credentials() -> bool {
    if let Some(home) = dirs::home_dir() {
        let credentials_path = home.join(".claude").join(".credentials.json");
        credentials_path.exists()
    } else {
        false
    }
}

/// Render the models page
pub fn render_models(frame: &mut Frame, area: Rect, state: &AppState, effect_state: &EffectState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    render_provider_list(frame, chunks[0], state, effect_state);
    render_provider_details(frame, chunks[1], state, effect_state);
}

fn render_provider_list(frame: &mut Frame, area: Rect, state: &AppState, effect_state: &EffectState) {
    // Use animated border when content pane is focused (sidebar_focus = false)
    let border_style = if !state.sidebar_focus {
        effects::active_border_style(effect_state.elapsed_secs())
    } else {
        effects::inactive_border_style()
    };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title("LLM Providers");
    
    // Build list from known providers, checking configuration status
    let items: Vec<ListItem> = PROVIDERS.iter().enumerate().map(|(idx, info)| {
        // Check if configured via state or environment
        let is_configured = if let Some(provider) = state.providers.get(idx) {
            provider.configured
        } else {
            !info.env_var.is_empty() && std::env::var(info.env_var).is_ok()
        };
        
        let (indicator, indicator_style) = if is_configured {
            ("● ", Style::default().fg(palette::CONFIGURED))
        } else if info.env_var.is_empty() {
            // Ollama doesn't need API key
            ("◌ ", Style::default().fg(Color::DarkGray))
        } else {
            ("○ ", Style::default().fg(palette::UNCONFIGURED))
        };
        
        ListItem::new(Line::from(vec![
            Span::styled(indicator, indicator_style),
            Span::raw(info.name),
        ]))
    }).collect();

    // Use active highlight only when content is focused
    let highlight_style = if !state.sidebar_focus {
        Style::default()
            .bg(palette::ACTIVE_PRIMARY)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::DIM)
    };

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_provider.min(PROVIDERS.len() - 1)));
    
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_provider_details(frame: &mut Frame, area: Rect, state: &AppState, _effect_state: &EffectState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Provider Configuration");

    // Get provider info from our known list
    let provider_idx = state.selected_provider.min(PROVIDERS.len() - 1);
    let info = &PROVIDERS[provider_idx];
    
    // Check configuration status based on auth type
    let (is_configured, detected_auth_type) = detect_provider_config(info);
    
    let status_color = if is_configured { palette::CONFIGURED } else { palette::UNCONFIGURED };
    let status_text = if is_configured { "✓ Configured" } else { "○ Not Configured" };
    
    let mut content = vec![
        Line::from(vec![
            Span::styled(info.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(info.description, Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Cyan)),
            Span::styled(status_text, Style::default().fg(status_color)),
        ]),
    ];
    
    // Show detected auth type if configured
    if is_configured {
        if let Some(auth_type) = detected_auth_type {
            content.push(Line::from(vec![
                Span::styled("Auth Type: ", Style::default().fg(Color::Cyan)),
                Span::styled(auth_type.label(), Style::default().fg(Color::Green)),
            ]));
        }
    }
    
    content.push(Line::from(vec![
        Span::styled("API URL: ", Style::default().fg(Color::Cyan)),
        Span::raw(info.api_url),
    ]));
    
    // Add models if loaded
    if let Some(provider) = state.providers.get(state.selected_provider) {
        if !provider.models.is_empty() {
            content.push(Line::from(""));
            content.push(Line::from(Span::styled("Available Models:", Style::default().fg(Color::Cyan))));
            for model in provider.models.iter().take(5) {
                content.push(Line::from(format!("  • {}", model)));
            }
            if provider.models.len() > 5 {
                content.push(Line::from(Span::styled(
                    format!("  ... and {} more", provider.models.len() - 5),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }
    
    // Configuration guidance
    content.push(Line::from(""));
    content.push(Line::from(Span::styled("─── Configuration ───", Style::default().fg(Color::DarkGray))));
    content.push(Line::from(""));
    
    // Render auth options based on provider capabilities
    render_auth_options(info, &mut content);
    
    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "[e]dit  [t]est connection  [s]witch auth type",
        Style::default().fg(Color::DarkGray),
    )));
    
    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

/// Detect if a provider is configured and what auth type is in use
fn detect_provider_config(info: &ProviderInfo) -> (bool, Option<AuthType>) {
    // Check for no-auth providers first
    if info.auth_types.contains(&AuthType::None) {
        return (true, Some(AuthType::None));
    }
    
    // For Anthropic, check session key first (preferred), then API key
    if info.name == "Anthropic" {
        if has_claude_code_credentials() {
            return (true, Some(AuthType::SessionKey));
        }
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            return (true, Some(AuthType::ApiKey));
        }
        return (false, None);
    }
    
    // For AWS Bedrock, check all required env vars
    if info.name == "AWS Bedrock" {
        let has_access = std::env::var("AWS_ACCESS_KEY_ID").is_ok();
        let has_secret = std::env::var("AWS_SECRET_ACCESS_KEY").is_ok();
        let has_region = std::env::var("AWS_REGION").is_ok() || std::env::var("AWS_DEFAULT_REGION").is_ok();
        if has_access && has_secret && has_region {
            return (true, Some(AuthType::AwsCredentials));
        }
        return (false, None);
    }
    
    // Standard API key check
    if !info.env_var.is_empty() && std::env::var(info.env_var).is_ok() {
        return (true, Some(AuthType::ApiKey));
    }
    
    (false, None)
}

/// Render authentication options for a provider
fn render_auth_options(info: &ProviderInfo, content: &mut Vec<Line>) {
    // No auth needed (Ollama)
    if info.auth_types.contains(&AuthType::None) {
        content.push(Line::from(Span::styled("No API key required", Style::default().fg(Color::Green))));
        content.push(Line::from(""));
        content.push(Line::from("Just start the Ollama server locally."));
        return;
    }
    
    let mut option_num = 1;
    
    // Session Key option (for Anthropic)
    if info.auth_types.contains(&AuthType::SessionKey) {
        let has_creds = has_claude_code_credentials();
        let status = if has_creds { 
            Span::styled(" ✓", Style::default().fg(Color::Green)) 
        } else { 
            Span::raw("") 
        };
        
        content.push(Line::from(vec![
            Span::styled(format!("Option {}: ", option_num), Style::default().fg(palette::INFO)),
            Span::raw("Session Key (Claude Code)"),
            status,
        ]));
        
        if let Some(hint) = info.session_key_hint {
            content.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(hint, Style::default().fg(Color::Gray)),
            ]));
        }
        
        if !has_creds {
            content.push(Line::from(vec![
                Span::styled("  Install Claude Code: ", Style::default().fg(Color::Gray)),
                Span::styled("npm i -g @anthropic-ai/claude-code", Style::default().fg(Color::Yellow)),
            ]));
            content.push(Line::from(vec![
                Span::styled("  Then run: ", Style::default().fg(Color::Gray)),
                Span::styled("claude", Style::default().fg(Color::Yellow)),
                Span::styled(" (to authenticate)", Style::default().fg(Color::Gray)),
            ]));
        } else {
            content.push(Line::from(Span::styled(
                "  ✓ Credentials detected - ready to use!",
                Style::default().fg(Color::Green),
            )));
        }
        
        content.push(Line::from(""));
        option_num += 1;
    }
    
    // API Key option
    if info.auth_types.contains(&AuthType::ApiKey) {
        let has_env = !info.env_var.is_empty() && std::env::var(info.env_var).is_ok();
        let status = if has_env { 
            Span::styled(" ✓", Style::default().fg(Color::Green)) 
        } else { 
            Span::raw("") 
        };
        
        content.push(Line::from(vec![
            Span::styled(format!("Option {}: ", option_num), Style::default().fg(palette::INFO)),
            Span::raw("API Key"),
            status,
        ]));
        
        content.push(Line::from(vec![
            Span::styled("  export ", Style::default().fg(Color::DarkGray)),
            Span::styled(info.env_var, Style::default().fg(Color::Yellow)),
            Span::styled("=\"your-api-key\"", Style::default().fg(Color::DarkGray)),
        ]));
        content.push(Line::from(""));
        
        // Vault option for API keys
        content.push(Line::from(vec![
            Span::styled(format!("Option {}: ", option_num + 1), Style::default().fg(palette::VAULT_HINT)),
            Span::raw("Credential vault (recommended)"),
        ]));
        content.push(Line::from(vec![
            Span::styled("  1. Go to ", Style::default().fg(Color::Gray)),
            Span::styled("Credentials", Style::default().fg(palette::ACTIVE_PRIMARY)),
            Span::styled(" (press ", Style::default().fg(Color::Gray)),
            Span::styled("2", Style::default().fg(Color::Yellow)),
            Span::styled(")", Style::default().fg(Color::Gray)),
        ]));
        content.push(Line::from(vec![
            Span::styled("  2. Switch to ", Style::default().fg(Color::Gray)),
            Span::styled("Providers", Style::default().fg(palette::ACTIVE_PRIMARY)),
            Span::styled(" tab (", Style::default().fg(Color::Gray)),
            Span::styled("{", Style::default().fg(Color::Yellow)),
            Span::styled("/", Style::default().fg(Color::Gray)),
            Span::styled("}", Style::default().fg(Color::Yellow)),
            Span::styled(" to navigate tabs)", Style::default().fg(Color::Gray)),
        ]));
        content.push(Line::from(vec![
            Span::styled("  3. Select ", Style::default().fg(Color::Gray)),
            Span::styled(info.vault_name, Style::default().fg(Color::Yellow)),
            Span::styled(" and press ", Style::default().fg(Color::Gray)),
            Span::styled("a", Style::default().fg(Color::Yellow)),
            Span::styled(" to add your API key", Style::default().fg(Color::Gray)),
        ]));
        content.push(Line::from(""));
        
        // Documentation link
        content.push(Line::from(vec![
            Span::styled("Get API key: ", Style::default().fg(Color::Cyan)),
            Span::styled(info.docs_url, Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)),
        ]));
    }
    
    // AWS Credentials option
    if info.auth_types.contains(&AuthType::AwsCredentials) {
        content.push(Line::from(vec![
            Span::styled(format!("Option {}: ", option_num), Style::default().fg(palette::INFO)),
            Span::raw("AWS Credentials"),
        ]));
        content.push(Line::from(vec![
            Span::styled("  export ", Style::default().fg(Color::DarkGray)),
            Span::styled("AWS_ACCESS_KEY_ID", Style::default().fg(Color::Yellow)),
            Span::styled("=\"...\"", Style::default().fg(Color::DarkGray)),
        ]));
        content.push(Line::from(vec![
            Span::styled("  export ", Style::default().fg(Color::DarkGray)),
            Span::styled("AWS_SECRET_ACCESS_KEY", Style::default().fg(Color::Yellow)),
            Span::styled("=\"...\"", Style::default().fg(Color::DarkGray)),
        ]));
        content.push(Line::from(vec![
            Span::styled("  export ", Style::default().fg(Color::DarkGray)),
            Span::styled("AWS_REGION", Style::default().fg(Color::Yellow)),
            Span::styled("=\"us-east-1\"", Style::default().fg(Color::DarkGray)),
        ]));
        content.push(Line::from(""));
        content.push(Line::from(vec![
            Span::styled("Docs: ", Style::default().fg(Color::Cyan)),
            Span::styled(info.docs_url, Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)),
        ]));
    }
}
