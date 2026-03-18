//! Overlay renderers for vault, settings, models, and cron.
//!
//! Each overlay is a centered modal that delegates to existing component renderers.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Tabs, Wrap},
    Frame,
};

use super::centered_rect;
use crate::app::CredentialsTab;
use crate::effects::EffectState;
use crate::state::AppState;

/// Render the vault/credentials overlay (Alt+V) — 90%x90% centered.
pub fn render_vault_overlay(
    frame: &mut Frame,
    full: Rect,
    state: &AppState,
    _effect_state: &EffectState,
) {
    let area = centered_rect(90, 90, full);
    frame.render_widget(Clear, area);

    let tab = CredentialsTab::from_index(state.credentials_tab);
    let titles: Vec<Line<'_>> = CredentialsTab::all()
        .iter()
        .map(|t| {
            if *t == tab {
                Line::from(Span::styled(
                    t.label(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(
                    t.label(),
                    Style::default().fg(Color::DarkGray),
                ))
            }
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Vault ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Tab bar + body
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Fill(1)])
        .split(inner);

    let tabs = Tabs::new(titles)
        .select(tab.index())
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider("│");
    frame.render_widget(tabs, chunks[0]);

    // Vault not ready guard
    if !state.vault.initialized {
        super::credentials::render_vault_hint(
            frame,
            chunks[1],
            "Vault not initialized",
            "i: initialize",
        );
        return;
    }
    if state.vault.locked {
        super::credentials::render_vault_hint(frame, chunks[1], "Vault locked", "u: unlock");
        return;
    }

    match state.credentials_tab {
        0 => super::credentials::render_endpoints_list(frame, chunks[1], state),
        1 => super::credentials::render_providers_list(frame, chunks[1], state),
        2 => super::credentials::render_permissions_list(frame, chunks[1], state),
        3 => super::credentials::render_audit_list(frame, chunks[1]),
        _ => {}
    }
}

/// Render the settings overlay (Alt+S) — 80%x85% centered.
pub fn render_settings_overlay(
    frame: &mut Frame,
    full: Rect,
    state: &AppState,
    _effect_state: &EffectState,
) {
    use crate::components::settings::SETTINGS_SECTION_LABELS;
    use crate::effects::palette;

    let area = centered_rect(80, 85, full);
    frame.render_widget(Clear, area);

    let primary = palette::border(&state.tui_config);
    let secondary = palette::text_secondary(&state.tui_config);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(primary))
        .style(Style::default().bg(palette::bg_primary(&state.tui_config)))
        .title(Span::styled(
            " Settings ",
            Style::default().fg(primary).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Tab bar + body
    let titles: Vec<Line<'_>> = SETTINGS_SECTION_LABELS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if i == state.selected_settings_card {
                Line::from(Span::styled(
                    *label,
                    Style::default().fg(primary).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(*label, Style::default().fg(secondary)))
            }
        })
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Fill(1)])
        .split(inner);

    let tabs = Tabs::new(titles)
        .select(state.selected_settings_card)
        .style(Style::default().fg(secondary))
        .highlight_style(Style::default().fg(primary).add_modifier(Modifier::BOLD))
        .divider("│");
    frame.render_widget(tabs, chunks[0]);

    super::settings::render_settings_detail(frame, chunks[1], state);
}

/// Render the models overlay (Alt+M) — 80%x85% centered.
pub fn render_models_overlay(
    frame: &mut Frame,
    full: Rect,
    state: &AppState,
    provider_index: usize,
    query: &str,
    selected_model: usize,
    _effect_state: &EffectState,
) {
    let area = centered_rect(80, 85, full);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Models ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.providers.is_empty() {
        super::models::render_no_providers(frame, inner);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(26), Constraint::Fill(1)])
        .split(inner);

    let provider_rows: Vec<Line<'_>> = state
        .providers
        .iter()
        .enumerate()
        .map(|(i, provider)| {
            let model_count = provider.models.len();
            let profiles = provider
                .models
                .iter()
                .filter(|model| {
                    model
                        .kind
                        .as_deref()
                        .is_some_and(|kind| kind.starts_with("inference_profile"))
                })
                .count();
            let label = format!(
                "{}  {}  ({model_count} items, {profiles} profiles)",
                if i == provider_index { "▶" } else { " " },
                provider.name
            );
            let style = if i == provider_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(Span::styled(label, style))
        })
        .collect();

    let provider_block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " Providers ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(Paragraph::new(provider_rows).block(provider_block), chunks[0]);

    let rhs = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(8), Constraint::Fill(1)])
        .split(chunks[1]);

    let search = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("Search: {}", if query.is_empty() { "(type to filter)" } else { query }),
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "Left/Right provider  Up/Down result  Enter create agent  Ctrl+E configure  Ctrl+U clear",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(search, rhs[0]);

    let idx = provider_index.min(state.providers.len().saturating_sub(1));
    render_provider_detail_at(frame, rhs[1], state, idx);

    let provider = &state.providers[idx];
    let filtered: Vec<usize> = crate::search::fuzzy_indices(
        query,
        provider.models.iter().enumerate().map(|(index, model)| {
            (
                index,
                format!(
                    "{} {} {} {}",
                    model.name,
                    model.id,
                    model.description,
                    model.kind.clone().unwrap_or_default()
                ),
            )
        }),
    );
    let selected = selected_model.min(filtered.len().saturating_sub(1));
    let rows: Vec<Line<'_>> = if filtered.is_empty() {
        vec![Line::from(Span::styled(
            "No models or inference profiles match this search",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        filtered
            .iter()
            .enumerate()
            .map(|(row, model_index)| {
                let model = &provider.models[*model_index];
                let prefix = if row == selected { "▶" } else { " " };
                let kind = match model.kind.as_deref() {
                    Some("foundation_model") => "model",
                    Some(kind) if kind.starts_with("inference_profile") => "profile",
                    Some(other) => other,
                    None => "model",
                };
                let tokens = model
                    .max_output_tokens
                    .map_or_else(|| format!("{}k ctx", model.context_window / 1000), |tokens| {
                        format!("{}k ctx / {}k out", model.context_window / 1000, tokens / 1000)
                    });
                let style = if row == selected {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(vec![
                    Span::styled(format!("{prefix} "), style),
                    Span::styled(format!("[{kind}] "), Style::default().fg(Color::DarkGray)),
                    Span::styled(&model.name, style),
                    Span::styled(format!("  {tokens}"), Style::default().fg(Color::DarkGray)),
                ])
            })
            .collect()
    };

    let models_block = Block::default().title(Span::styled(
        format!(" Results ({}) ", filtered.len()),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(
        Paragraph::new(rows)
            .block(models_block)
            .wrap(Wrap { trim: false }),
        rhs[2],
    );
}

pub fn render_agent_launcher_overlay(
    frame: &mut Frame,
    full: Rect,
    state: &AppState,
    launcher: &crate::state::AgentLauncherState,
    _effect_state: &EffectState,
) {
    let area = centered_rect(60, 60, full);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Agents ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Fill(1)])
        .split(inner);

    let query = Paragraph::new(vec![
        Line::from(Span::styled(
            format!(
                "Search: {}",
                if launcher.query.is_empty() {
                    "(type an agent id or model)"
                } else {
                    launcher.query.as_str()
                }
            ),
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "Enter selects  Ctrl+U clears  Esc closes",
            Style::default().fg(Color::DarkGray),
        )),
    ]);
    frame.render_widget(query, chunks[0]);

    let mut items: Vec<Line<'_>> = crate::search::fuzzy_indices(
        &launcher.query,
        state
            .agents
            .iter()
            .enumerate()
            .filter(|(_, agent)| agent.enabled)
            .map(|(idx, agent)| {
                (
                    idx,
                    format!(
                        "{} {} {}",
                        agent.id,
                        agent.model.as_deref().unwrap_or("unconfigured"),
                        agent.workspace.as_deref().unwrap_or("")
                    ),
                )
            }),
    )
    .into_iter()
    .enumerate()
    .filter_map(|(row, index)| {
        let agent = state.agents.get(index)?;
        let style = if row == launcher.selected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        Some(Line::from(vec![
            Span::styled(
                if row == launcher.selected { "▶ " } else { "  " },
                style,
            ),
            Span::styled(agent.id.clone(), style),
            Span::styled(
                format!(
                    "  [{}]",
                    agent.model.as_deref().unwrap_or("unconfigured")
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]))
    })
    .collect();

    let create_selected = launcher.selected == items.len();
    items.push(Line::from(vec![
        Span::styled(
            if create_selected { "▶ " } else { "  " },
            if create_selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
        Span::styled(
            "+ Create new agent",
            if create_selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            },
        ),
    ]));

    frame.render_widget(Paragraph::new(items), chunks[1]);
}

/// Render provider details for a specific index (used by models overlay).
fn render_provider_detail_at(frame: &mut Frame, area: Rect, state: &AppState, idx: usize) {
    use crate::effects::palette;
    use ratatui::widgets::Wrap;

    let Some(provider) = state.providers.get(idx) else {
        let paragraph = Paragraph::new("No provider selected");
        frame.render_widget(paragraph, area);
        return;
    };

    let status_color = if provider.available {
        palette::CONFIGURED
    } else {
        palette::UNCONFIGURED
    };
    let status_text = if provider.available {
        "Available"
    } else {
        "Not Available"
    };

    let mut content = vec![
        Line::from(Span::styled(
            &provider.name,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("Provider ID: {}", provider.id),
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Cyan)),
            Span::styled(status_text, Style::default().fg(status_color)),
        ]),
    ];

    // Capabilities
    content.push(Line::from(""));
    let cap_items = [
        ("Streaming", provider.supports_streaming),
        ("Tool Use", provider.supports_tools),
        ("Vision", provider.supports_vision),
    ];
    for (name, supported) in cap_items {
        let (icon, color) = if supported {
            ("\u{2713}", Color::Green)
        } else {
            ("\u{2717}", Color::DarkGray)
        };
        content.push(Line::from(vec![
            Span::styled(format!("  {icon} "), Style::default().fg(color)),
            Span::raw(name),
        ]));
    }

    let foundation_models = provider
        .models
        .iter()
        .filter(|model| model.kind.as_deref() == Some("foundation_model"))
        .count();
    let inference_profiles = provider
        .models
        .iter()
        .filter(|model| {
            model
                .kind
                .as_deref()
                .is_some_and(|kind| kind.starts_with("inference_profile"))
        })
        .count();

    content.push(Line::from(""));
    content.push(Line::from(vec![
        Span::styled("Inventory: ", Style::default().fg(Color::Cyan)),
        Span::styled(
            format!(
                "{} total / {} models / {} profiles",
                provider.models.len(),
                foundation_models,
                inference_profiles
            ),
            Style::default().fg(Color::White),
        ),
    ]));

    // Models summary
    if !provider.models.is_empty() {
        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            format!("{} targets available — type to search, Enter to create an agent", provider.models.len()),
            Style::default().fg(Color::DarkGray),
        )));
    }

    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "[Enter] create agent  [Ctrl+E] configure provider",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(content).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

/// Render the cron jobs overlay (Alt+C) — 85%x85% centered.
pub fn render_cron_overlay(
    frame: &mut Frame,
    full: Rect,
    state: &AppState,
    _scroll: usize,
    _effect_state: &EffectState,
) {
    let area = centered_rect(85, 85, full);
    frame.render_widget(Clear, area);

    let filter_labels = ["All", "Active", "Disabled"];
    let titles: Vec<Line<'_>> = filter_labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if i == state.selected_cron_card {
                Line::from(Span::styled(
                    *label,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(*label, Style::default().fg(Color::DarkGray)))
            }
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Cron Jobs ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Fill(1)])
        .split(inner);

    let tabs = Tabs::new(titles)
        .select(state.selected_cron_card)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider("│");
    frame.render_widget(tabs, chunks[0]);

    super::cron::render_cron_detail(frame, chunks[1], state);
}
