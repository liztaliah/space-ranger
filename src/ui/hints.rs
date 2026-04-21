use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::ui::theme;

pub fn render(f: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" hjkl/arrows", Style::default().fg(theme::BORDER)),
        Span::styled(":navigate  ", Style::default().fg(theme::MUTED)),
        Span::styled("/", Style::default().fg(theme::BORDER)),
        Span::styled(":search  ", Style::default().fg(theme::MUTED)),
        Span::styled("d", Style::default().fg(theme::BORDER)),
        Span::styled(":delete  ", Style::default().fg(theme::MUTED)),
        Span::styled("q", Style::default().fg(theme::BORDER)),
        Span::styled(":quit ", Style::default().fg(theme::MUTED)),
    ]);
    let para = Paragraph::new(line).style(Style::default().bg(theme::BG));
    f.render_widget(para, area);
}
