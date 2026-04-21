//! Delete confirmation modal overlay.
//!
//! ratatui has no built-in modal system. The technique used here:
//!   1. Compute a centered Rect over the full terminal area.
//!   2. Render `Clear` to erase whatever was drawn underneath.
//!   3. Draw the dialog box on top.
//!
//! Because modals are rendered last in ui/mod.rs, they always appear above
//! the tree and preview panels.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::AppState;
use crate::ui::theme;

pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
    let filename = state
        .delete_target
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let modal_rect = centered_rect(60, 7, area);
    // Erase the background before drawing the dialog, otherwise panel content
    // would show through.
    f.render_widget(Clear, modal_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::PINK))
        .style(Style::default().bg(theme::SURFACE))
        .title(Span::styled(" Delete? ", Style::default().fg(theme::PINK)));

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("\"{}\"", filename),
            Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("This cannot be undone.", Style::default().fg(theme::MUTED))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [y] Delete  ", Style::default().fg(theme::RED).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled("  [n] Cancel  ", Style::default().fg(theme::TEXT)),
        ]),
    ];

    let para = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center);
    f.render_widget(para, modal_rect);
}

/// Returns a `Rect` of the given `width` × `height` centered within `area`.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let h_pad = area.width.saturating_sub(width) / 2;
    let v_pad = area.height.saturating_sub(height) / 2;

    let [_, vert_center, _] = Layout::vertical([
        Constraint::Length(v_pad),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .areas(area);

    let [_, horiz_center, _] = Layout::horizontal([
        Constraint::Length(h_pad),
        Constraint::Length(width),
        Constraint::Min(0),
    ])
    .areas(vert_center);

    horiz_center
}
