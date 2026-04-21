use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::AppState;
use crate::ui::theme;

pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
    let text = format!("/{}", state.search_query);
    let para = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(theme::GREEN)),
        )
        .style(Style::default().fg(theme::TEXT).bg(theme::SURFACE));
    f.render_widget(para, area);
}
