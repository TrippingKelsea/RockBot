//! Credentials/Vault management component
//!
//! Provider categories are populated dynamically from the gateway's credential
//! schema registries (LLM, Channel, Tool). When the gateway is not running,
//! the provider list shows an empty state message.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
    Frame,
};

use crate::tui::effects::{self, palette, EffectState};
use crate::tui::state::AppState;

/// Card dimensions
const CARD_WIDTH: u16 = 18;
const CARD_HEIGHT: u16 = 5;

/// Credential categories for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CredentialCategory {
    #[default]
    All,
    ModelProviders,
    CommunicationProviders,
    ToolProviders,
}

impl CredentialCategory {
    pub fn all() -> Vec<Self> {
        vec![
            Self::All,
            Self::ModelProviders,
            Self::CommunicationProviders,
            Self::ToolProviders,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::ModelProviders => "Model Providers",
            Self::CommunicationProviders => "Communication",
            Self::ToolProviders => "Tools",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::ModelProviders => "LLM",
            Self::CommunicationProviders => "Msg",
            Self::ToolProviders => "Tool",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::All => "All configured credentials",
            Self::ModelProviders => "LLM API keys (Anthropic, OpenAI, Google, AWS)",
            Self::CommunicationProviders => "Messaging services (Discord, Telegram, Signal)",
            Self::ToolProviders => "Tool integrations (MCP servers, etc.)",
        }
    }
}

/// Credential tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialsTab {
    Endpoints,
    Providers,
    Permissions,
    Audit,
}

impl CredentialsTab {
    pub fn all() -> Vec<Self> {
        vec![Self::Endpoints, Self::Providers, Self::Permissions, Self::Audit]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::Endpoints => "Endpoints",
            Self::Providers => "Providers",
            Self::Permissions => "Permissions",
            Self::Audit => "Audit",
        }
    }
}

/// Render the credentials page
pub fn render_credentials(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    selected_tab: usize,
    effect_state: &EffectState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Tabs
            Constraint::Min(0),     // Content
        ])
        .split(area);

    // Render tabs
    let titles: Vec<Line> = CredentialsTab::all()
        .iter()
        .map(|t| Line::from(t.title()))
        .collect();

    let tab_border_style = if !state.sidebar_focus {
        effects::active_border_style(effect_state.elapsed_secs())
    } else {
        effects::inactive_border_style()
    };

    let tabs = Tabs::new(titles)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(tab_border_style)
            .title("Credentials"))
        .select(selected_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(palette::ACTIVE_PRIMARY)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, chunks[0]);

    if !state.vault.initialized {
        render_vault_init(frame, chunks[1], state);
    } else if state.vault.locked {
        render_vault_locked(frame, chunks[1]);
    } else {
        match selected_tab {
            0 => render_endpoints(frame, chunks[1], state, effect_state),
            1 => render_providers(frame, chunks[1], state, effect_state),
            2 => render_permissions(frame, chunks[1]),
            3 => render_audit(frame, chunks[1]),
            _ => {}
        }
    }
}

fn render_vault_init(frame: &mut Frame, area: Rect, state: &AppState) {
    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Vault not initialized",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Vault Path: ", Style::default().fg(Color::Cyan)),
            Span::raw(state.vault_path.display().to_string()),
        ]),
        Line::from(""),
        Line::from("The credential vault needs to be initialized before"),
        Line::from("you can store API keys and secrets."),
        Line::from(""),
        Line::from(Span::styled(
            "Press 'i' to initialize with password",
            Style::default().fg(Color::Green),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Initialize Vault");

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_vault_locked(frame: &mut Frame, area: Rect) {
    let content = vec![
        Line::from(""),
        Line::from(Span::styled("Vault Locked", Style::default().fg(Color::Yellow))),
        Line::from(""),
        Line::from("Enter your password to unlock the vault."),
        Line::from(""),
        Line::from(Span::styled("Press 'u' to unlock", Style::default().fg(Color::Green))),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Unlock Vault");

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render endpoints with card strip on top, details below
fn render_endpoints(frame: &mut Frame, area: Rect, state: &AppState, effect_state: &EffectState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(CARD_HEIGHT), Constraint::Min(0)])
        .split(area);

    render_endpoint_cards(frame, chunks[0], state, effect_state);
    render_endpoint_details(frame, chunks[1], state);
}

fn render_endpoint_cards(frame: &mut Frame, area: Rect, state: &AppState, effect_state: &EffectState) {
    if state.endpoints.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(effects::inactive_border_style())
            .title("Endpoints");
        let hint = Paragraph::new(Line::from(Span::styled(
            " No endpoints. Press 'a' to add. ",
            Style::default().fg(Color::DarkGray),
        )))
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(hint, area);
        return;
    }

    let total = state.endpoints.len();
    let max_visible = (area.width / CARD_WIDTH) as usize;
    let max_visible = max_visible.max(1);

    let half = max_visible / 2;
    let start = if state.selected_endpoint <= half {
        0
    } else if state.selected_endpoint + half >= total {
        total.saturating_sub(max_visible)
    } else {
        state.selected_endpoint - half
    };
    let end = (start + max_visible).min(total);
    let visible_count = end - start;

    let mut constraints: Vec<Constraint> = (0..visible_count)
        .map(|_| Constraint::Length(CARD_WIDTH))
        .collect();
    constraints.push(Constraint::Min(0));

    let card_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    let elapsed = effect_state.elapsed_secs();

    for (vi, idx) in (start..end).enumerate() {
        let endpoint = &state.endpoints[idx];
        let is_selected = idx == state.selected_endpoint;

        let border_style = if is_selected {
            effects::active_border_style(elapsed)
        } else {
            Style::default().fg(palette::INACTIVE_BORDER)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(card_chunks[vi]);
        frame.render_widget(block, card_chunks[vi]);

        if inner.height < 3 || inner.width < 3 {
            continue;
        }

        let max_w = inner.width as usize;

        // Line 1: credential indicator + name
        let (cred_icon, cred_color) = if endpoint.has_credential {
            ("●", Color::Green)
        } else {
            ("○", Color::Yellow)
        };
        let name: String = if endpoint.name.len() > max_w.saturating_sub(2) {
            endpoint.name[..max_w.saturating_sub(2)].to_string()
        } else {
            endpoint.name.clone()
        };

        // Line 2: endpoint type (truncated)
        let etype: String = if endpoint.endpoint_type.len() > max_w {
            endpoint.endpoint_type[..max_w].to_string()
        } else {
            endpoint.endpoint_type.clone()
        };

        // Line 3: short URL or ID hint
        let url_short: String = if endpoint.base_url.is_empty() {
            endpoint.id[..endpoint.id.len().min(max_w)].to_string()
        } else {
            let u = endpoint.base_url.replace("https://", "").replace("http://", "");
            if u.len() > max_w { u[..max_w].to_string() } else { u }
        };

        let name_style = if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let lines = vec![
            Line::from(vec![
                Span::styled(cred_icon, Style::default().fg(cred_color)),
                Span::styled(format!(" {name}"), name_style),
            ]),
            Line::from(Span::styled(etype, Style::default().fg(Color::Cyan))),
            Line::from(Span::styled(url_short, Style::default().fg(Color::DarkGray))),
        ];

        let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
        let render_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height.min(3),
        };
        frame.render_widget(paragraph, render_area);
    }

    if visible_count < card_chunks.len() {
        let filler = Block::default().borders(Borders::NONE);
        frame.render_widget(filler, card_chunks[visible_count]);
    }
}

fn render_endpoint_details(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette::INACTIVE_BORDER))
        .title("Details");

    if let Some(endpoint) = state.endpoints.get(state.selected_endpoint) {
        let cred_status = if endpoint.has_credential { "Stored" } else { "Missing" };
        let cred_color = if endpoint.has_credential { Color::Green } else { Color::Red };

        let details = vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::Cyan)),
                Span::raw(&endpoint.id),
            ]),
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                Span::raw(&endpoint.name),
            ]),
            Line::from(vec![
                Span::styled("Type: ", Style::default().fg(Color::Cyan)),
                Span::raw(&endpoint.endpoint_type),
            ]),
            Line::from(vec![
                Span::styled("URL: ", Style::default().fg(Color::Cyan)),
                Span::raw(&endpoint.base_url),
            ]),
            Line::from(vec![
                Span::styled("Credential: ", Style::default().fg(Color::Cyan)),
                Span::styled(cred_status, Style::default().fg(cred_color)),
            ]),
            Line::from(vec![
                Span::styled("Expires: ", Style::default().fg(Color::Cyan)),
                Span::raw(endpoint.expiration.as_deref().unwrap_or("Never")),
            ]),
            Line::from(""),
            Line::from(Span::styled("[d]elete  [e]dit  [r]efresh", Style::default().fg(Color::DarkGray))),
        ];
        let paragraph = Paragraph::new(details).block(block);
        frame.render_widget(paragraph, area);
    } else {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("Select an endpoint", Style::default().fg(Color::DarkGray))),
        ])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(empty, area);
    }
}

/// Render the Providers tab - categorized credential templates
fn render_providers(frame: &mut Frame, area: Rect, state: &AppState, effect_state: &EffectState) {
    // Split: category card strip on top, provider list below
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(CARD_HEIGHT), Constraint::Min(0)])
        .split(area);

    render_category_cards(frame, chunks[0], state, effect_state);

    let categories = CredentialCategory::all();
    let selected_category = categories.get(state.selected_category).copied().unwrap_or(CredentialCategory::All);
    render_category_providers(frame, chunks[1], state, selected_category, effect_state);
}

fn render_category_cards(frame: &mut Frame, area: Rect, state: &AppState, effect_state: &EffectState) {
    let categories = CredentialCategory::all();
    let total = categories.len();
    let card_w: u16 = 14;

    let mut constraints: Vec<Constraint> = (0..total)
        .map(|_| Constraint::Length(card_w))
        .collect();
    constraints.push(Constraint::Min(0));

    let card_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    let elapsed = effect_state.elapsed_secs();
    let category_active = !state.sidebar_focus && !state.provider_list_focus;

    for (idx, cat) in categories.iter().enumerate() {
        let is_selected = idx == state.selected_category;

        let border_style = if is_selected && category_active {
            effects::active_border_style(elapsed)
        } else if is_selected {
            Style::default().fg(palette::ACTIVE_PRIMARY)
        } else {
            Style::default().fg(palette::INACTIVE_BORDER)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(card_chunks[idx]);
        frame.render_widget(block, card_chunks[idx]);

        if inner.height < 3 || inner.width < 1 {
            continue;
        }

        // Count providers in this category
        let count = match cat {
            CredentialCategory::All => state.credential_schemas.len(),
            CredentialCategory::ModelProviders => state.credential_schemas.iter().filter(|s| s.category == "model").count(),
            CredentialCategory::CommunicationProviders => state.credential_schemas.iter().filter(|s| s.category == "communication").count(),
            CredentialCategory::ToolProviders => state.credential_schemas.iter().filter(|s| s.category == "tool").count(),
        };

        let title_style = if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let lines = vec![
            Line::from(Span::styled(cat.icon(), Style::default().fg(Color::Cyan))),
            Line::from(Span::styled(cat.title(), title_style)),
            Line::from(Span::styled(format!("{count}"), Style::default().fg(Color::DarkGray))),
        ];

        let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
        let render_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height.min(3),
        };
        frame.render_widget(paragraph, render_area);
    }
}

/// Render provider list for a category (vertical list below the category cards)
fn render_category_providers(frame: &mut Frame, area: Rect, state: &AppState, category: CredentialCategory, effect_state: &EffectState) {
    let provider_active = !state.sidebar_focus && state.provider_list_focus;
    let border_style = if provider_active {
        effects::active_border_style(effect_state.elapsed_secs())
    } else {
        effects::inactive_border_style()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!("{} (Enter to select, 'a' to add)", category.title()));

    let schemas = &state.credential_schemas;

    if schemas.is_empty() {
        let content = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Start the gateway to see providers",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Run: rockbot gateway start",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let paragraph = Paragraph::new(content).block(block).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    let filtered: Vec<&crate::tui::state::CredentialSchemaInfo> = match category {
        CredentialCategory::All => schemas.iter().collect(),
        CredentialCategory::ModelProviders => schemas.iter().filter(|s| s.category == "model").collect(),
        CredentialCategory::CommunicationProviders => schemas.iter().filter(|s| s.category == "communication").collect(),
        CredentialCategory::ToolProviders => schemas.iter().filter(|s| s.category == "tool").collect(),
    };

    if filtered.is_empty() {
        let content = vec![
            Line::from(Span::styled(category.description(), Style::default().fg(Color::DarkGray))),
            Line::from(""),
            Line::from(Span::styled(
                "No providers registered for this category",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = filtered.iter().map(|schema| {
        let configured = check_provider_configured(state, &schema.provider_id);
        let indicator = if configured { "●" } else { "○" };
        let ind_color = if configured { Color::Green } else { Color::Yellow };

        let cat_icon = match schema.category.as_str() {
            "model" => "LLM ",
            "communication" => "MSG ",
            "tool" => "TL  ",
            _ => "",
        };

        let prefix = if category == CredentialCategory::All { cat_icon } else { "" };

        ListItem::new(Line::from(vec![
            Span::raw(prefix),
            Span::styled(format!("{indicator} "), Style::default().fg(ind_color)),
            Span::styled(schema.provider_name.as_str(), Style::default().fg(Color::White)),
            Span::styled(format!(" ({})", schema.provider_id), Style::default().fg(Color::DarkGray)),
        ]))
    }).collect();

    let highlight_style = if provider_active {
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
    if provider_active || state.provider_list_focus {
        list_state.select(Some(state.selected_provider_index.min(filtered.len().saturating_sub(1))));
    }

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Check if a provider is configured in state
fn check_provider_configured(state: &AppState, provider_id: &str) -> bool {
    state.endpoints.iter().any(|e| {
        e.id.to_lowercase().contains(provider_id) ||
        e.name.to_lowercase().contains(provider_id)
    })
}

fn render_permissions(frame: &mut Frame, area: Rect) {
    let content = vec![
        Line::from(""),
        Line::from("Permission rules control agent access to credentials."),
        Line::from(""),
        Line::from(Span::styled(
            "Press 'a' to add a rule",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Permission Rules");

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_audit(frame: &mut Frame, area: Rect) {
    let content = vec![
        Line::from(""),
        Line::from("Audit log tracks all credential access."),
        Line::from(""),
        Line::from(Span::styled(
            "Press 'v' to verify integrity",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Audit Log");

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}
