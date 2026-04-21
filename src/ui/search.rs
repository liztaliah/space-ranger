//! Bottom search bar — shown only when AppMode::Search is active.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::AppState;
use crate::ui::theme;

pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
    let line = Line::from(vec![
        Span::styled("Search: ", Style::default().fg(theme::GREEN)),
        Span::styled(
            state.search_query.clone(),
            Style::default().fg(theme::GREEN),
        ),
    ]);
    let para = Paragraph::new(line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::GREEN)),
        )
        .style(Style::default().bg(theme::BG));
    f.render_widget(para, area);
}
