//! Sidebar navigation component

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::tui::effects::{self, palette, EffectState};
use crate::tui::state::{AppState, MenuItem};

/// Render the sidebar navigation
pub fn render_sidebar(frame: &mut Frame, area: Rect, state: &AppState, effect_state: &EffectState) {
    // Split into title row + menu list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    // Title bar
    let title_style = if state.sidebar_focus {
        Style::default().fg(palette::ACTIVE_PRIMARY).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    };
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" 🦀 ", title_style),
        Span::styled("RockBot", title_style),
    ]));
    frame.render_widget(title, chunks[0]);

    // Menu items
    let items: Vec<ListItem> = MenuItem::all()
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let num = i + 1;
            let content = format!(" {num} {} {}", item.icon(), item.title());
            ListItem::new(content)
        })
        .collect();

    // Use animated purple border when sidebar is focused
    let border_style = if state.sidebar_focus {
        effects::active_border_style(effect_state.elapsed_secs())
    } else {
        effects::inactive_border_style()
    };

    let highlight_style = if state.sidebar_focus {
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
        .block(
            Block::default()
                .borders(Borders::RIGHT)
                .border_style(border_style),
        )
        .highlight_style(highlight_style)
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.menu_index));

    frame.render_stateful_widget(list, chunks[1], &mut list_state);
}
