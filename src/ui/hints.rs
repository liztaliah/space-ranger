//! Key binding hints bar — the single row at the bottom of the screen.
//! Content switches depending on which panel currently has focus.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{AppState, Focus};
use crate::ui::theme;

pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
    let line = if state.focus == Focus::Preview {
        Line::from(vec![
            Span::styled(" jk", Style::default().fg(theme::PINK)),
            Span::styled(":scroll  ", Style::default().fg(theme::MUTED)),
            Span::styled("^d/^u", Style::default().fg(theme::PINK)),
            Span::styled(":page  ", Style::default().fg(theme::MUTED)),
            Span::styled("g/G", Style::default().fg(theme::PINK)),
            Span::styled(":top/bot  ", Style::default().fg(theme::MUTED)),
            Span::styled("Tab/h/Esc", Style::default().fg(theme::PINK)),
            Span::styled(":back  ", Style::default().fg(theme::MUTED)),
            Span::styled("q", Style::default().fg(theme::PINK)),
            Span::styled(":quit ", Style::default().fg(theme::MUTED)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" hjkl/arrows", Style::default().fg(theme::BORDER)),
            Span::styled(":navigate  ", Style::default().fg(theme::MUTED)),
            Span::styled("Tab", Style::default().fg(theme::BORDER)),
            Span::styled(":read  ", Style::default().fg(theme::MUTED)),
            Span::styled("/", Style::default().fg(theme::BORDER)),
            Span::styled(":search  ", Style::default().fg(theme::MUTED)),
            Span::styled("d", Style::default().fg(theme::BORDER)),
            Span::styled(":delete  ", Style::default().fg(theme::MUTED)),
            Span::styled("q", Style::default().fg(theme::BORDER)),
            Span::styled(":quit ", Style::default().fg(theme::MUTED)),
        ])
    };
    let para = Paragraph::new(line).style(Style::default().bg(theme::BG));
    f.render_widget(para, area);
}
