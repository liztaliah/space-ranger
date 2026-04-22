//! Left panel: directory tree rendered as a ratatui List.
//!
//! ratatui has no built-in tree widget, so the tree is stored as a flat
//! `Vec<DirEntry>` in AppState with a `depth` field for indentation.
//! Expanding/collapsing a directory splices its children in or out of that
//! vec — see `app::expand_dir` and `app::collapse_dir`.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{AppState, Focus};
use crate::ui::theme;

const ICON_DIR_CLOSED: &str = " ";
const ICON_DIR_OPEN: &str = " ";
const ICON_FILE: &str = " ";

pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
    let focused = state.focus == Focus::Tree;
    let title = format!(" {} ", state.root.display());
    // Dim the border when the preview panel has focus, so it's immediately
    // clear which panel is receiving keyboard input.
    let border_color = if focused { theme::BORDER } else { theme::MUTED };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme::BG))
        .title(Span::styled(title, Style::default().fg(theme::TEXT)));

    // Show a loading indicator while the background search walk is in progress.
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
            let indent = "  ".repeat(entry.depth);
            let icon = if entry.is_dir {
                if entry.is_expanded { ICON_DIR_OPEN } else { ICON_DIR_CLOSED }
            } else {
                ICON_FILE
            };
            let prefix = format!("{}{}", indent, icon);

            let line = if !query.is_empty() {
                // Highlight the matching substring in green.
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
                    // Non-matching entry — dim it so matches stand out.
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
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    // `with_offset` sets the first visible row; `select` marks the cursor row.
    // Both are needed: offset controls scrolling, select controls the highlight.
    let mut list_state = ListState::default().with_offset(state.scroll_offset);
    list_state.select(if state.entries.is_empty() { None } else { Some(state.cursor) });

    f.render_stateful_widget(list, area, &mut list_state);
}
