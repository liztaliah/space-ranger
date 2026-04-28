//! Directory columns: current directory and parent directory panels.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{AppState, Focus};
use crate::ui::theme;

const ICON_DIR: &str = " ";
const ICON_FILE: &str = " ";

/// Render the current directory listing (the active/middle column).
pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
    let focused = state.focus == Focus::Tree;
    let title = format!(" {} ", state.root.display());
    // Col 2 is blue when it has column focus, or when the preview panel has focus.
    let border_color = if focused && state.focused_col == 2 || state.focus == Focus::Preview {
        theme::BORDER
    } else {
        theme::MUTED
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme::BG))
        .title(Span::styled(title, Style::default().fg(theme::TEXT)));

    if state.search_loading {
        let loading = Paragraph::new("Scanning…")
            .block(block)
            .style(Style::default().fg(theme::MUTED))
            .alignment(Alignment::Center);
        f.render_widget(loading, area);
        return;
    }

    let query = state.search_query.to_lowercase();

    let items: Vec<ListItem> = state
        .entries
        .iter()
        .map(|entry| {
            let icon = if entry.is_dir { ICON_DIR } else { ICON_FILE };
            let prefix = format!("{}{}", "  ".repeat(entry.depth), icon);

            let line = if !query.is_empty() {
                let name_lower = entry.name.to_lowercase();
                if let Some(pos) = name_lower.find(&query) {
                    let end = pos + query.len();
                    let before  = &entry.name[..pos];
                    let matched = &entry.name[pos..end];
                    let after   = &entry.name[end..];
                    Line::from(vec![
                        Span::styled(prefix, Style::default().fg(theme::MUTED)),
                        Span::styled(before.to_owned(), Style::default().fg(theme::TEXT)),
                        Span::styled(matched.to_owned(), Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
                        Span::styled(after.to_owned(), Style::default().fg(theme::TEXT)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(prefix, Style::default().fg(theme::MUTED)),
                        Span::styled(entry.name.clone(), Style::default().fg(theme::MUTED)),
                    ])
                }
            } else {
                let name_style = if entry.is_dir {
                    Style::default().fg(theme::BORDER).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT)
                };
                Line::from(vec![
                    Span::styled(prefix, Style::default().fg(theme::MUTED)),
                    Span::styled(entry.name.clone(), name_style),
                ])
            };

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme::PINK)
                .fg(ratatui::style::Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut list_state = ListState::default().with_offset(state.scroll_offset);
    list_state.select(if state.entries.is_empty() { None } else { Some(state.cursor) });

    f.render_stateful_widget(list, area, &mut list_state);
}

/// Render the parent directory listing (middle-left column).
pub fn render_parent(f: &mut Frame, state: &AppState, area: Rect) {
    let title = state.root
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| format!(" {} ", n.to_string_lossy()))
        .unwrap_or_else(|| " / ".to_owned());
    let active = state.focus == Focus::Tree && state.focused_col == 1;
    render_ancestor_column(f, &state.parent_entries, state.parent_cursor, &title, active, area);
}

/// Render the grandparent directory listing (far-left column).
pub fn render_grandparent(f: &mut Frame, state: &AppState, area: Rect) {
    let title = state.root
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .map(|n| format!(" {} ", n.to_string_lossy()))
        .unwrap_or_else(|| " / ".to_owned());
    let active = state.focus == Focus::Tree && state.focused_col == 0;
    render_ancestor_column(f, &state.grandparent_entries, state.grandparent_cursor, &title, active, area);
}

/// Shared renderer for ancestor columns (parent, grandparent).
/// `active` controls whether the border is blue (focused) or muted.
fn render_ancestor_column(f: &mut Frame, entries: &[crate::app::DirEntry], cursor: usize, title: &str, active: bool, area: Rect) {
    let border_color = if active { theme::BORDER } else { theme::MUTED };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme::BG))
        .title(Span::styled(title.to_owned(), Style::default().fg(theme::MUTED)));

    if entries.is_empty() {
        f.render_widget(block, area);
        return;
    }

    let items: Vec<ListItem> = entries
        .iter()
        .map(|entry| {
            let icon = if entry.is_dir { ICON_DIR } else { ICON_FILE };
            let name_style = if entry.is_dir {
                Style::default().fg(theme::MUTED).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::MUTED)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {}", icon), Style::default().fg(theme::MUTED)),
                Span::styled(entry.name.clone(), name_style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme::BORDER)
                .fg(ratatui::style::Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let visible = area.height.saturating_sub(2) as usize;
    let offset = cursor.saturating_sub(visible / 2);
    let mut list_state = ListState::default().with_offset(offset);
    list_state.select(Some(cursor));

    f.render_stateful_widget(list, area, &mut list_state);
}
