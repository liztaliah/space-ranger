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

pub fn render_rename(f: &mut Frame, state: &AppState, area: Rect) {
    let modal_rect = centered_rect(60, 7, area);
    f.render_widget(Clear, modal_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .style(Style::default().bg(theme::SURFACE))
        .title(Span::styled(" Rename ", Style::default().fg(theme::BORDER)));

    let cursor = "\u{2588}";
    let input_spans: Line = if state.rename_fresh {
        // Cursor before stem signals the whole name will be replaced on first keypress.
        Line::from(vec![
            Span::styled(cursor, Style::default().fg(theme::TEXT)),
            Span::styled(state.rename_stem.clone(), Style::default().fg(theme::MUTED).add_modifier(Modifier::BOLD)),
            Span::styled(state.rename_ext.clone(), Style::default().fg(theme::MUTED)),
        ])
    } else if state.rename_ext_focused {
        // Cursor in the extension. When fresh, cursor sits after the dot so the
        // extension name is visually selected; first keypress replaces it.
        let dot = state.rename_ext.get(..1).unwrap_or("");
        let ext_name = state.rename_ext.get(1..).unwrap_or(&state.rename_ext);
        if state.rename_ext_fresh {
            Line::from(vec![
                Span::styled(state.rename_stem.clone(), Style::default().fg(theme::MUTED).add_modifier(Modifier::BOLD)),
                Span::styled(dot, Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
                Span::styled(cursor, Style::default().fg(theme::TEXT)),
                Span::styled(ext_name, Style::default().fg(theme::MUTED).add_modifier(Modifier::BOLD)),
            ])
        } else {
            Line::from(vec![
                Span::styled(state.rename_stem.clone(), Style::default().fg(theme::MUTED).add_modifier(Modifier::BOLD)),
                Span::styled(state.rename_ext.clone(), Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
                Span::styled(cursor, Style::default().fg(theme::TEXT)),
            ])
        }
    } else {
        // Normal editing: cursor after stem, ext dimmed.
        Line::from(vec![
            Span::styled(state.rename_stem.clone(), Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
            Span::styled(cursor, Style::default().fg(theme::TEXT)),
            Span::styled(state.rename_ext.clone(), Style::default().fg(theme::MUTED)),
        ])
    };

    let rename_style = if !state.rename_cancel_focused {
        Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::MUTED)
    };
    let cancel_style = if state.rename_cancel_focused {
        Style::default().fg(theme::PINK).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::MUTED)
    };

    let lines = vec![
        Line::from(""),
        input_spans,
        Line::from(""),
        Line::from(vec![
            Span::styled("  [Enter] Rename  ", rename_style),
            Span::raw("    "),
            Span::styled("  [Esc] Cancel  ", cancel_style),
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
