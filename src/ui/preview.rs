use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{AppState, Focus, PreviewContent};
use crate::ui::theme;

pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
    let focused = state.focus == Focus::Preview;
    let title = state
        .selected_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| format!(" {} ", n.to_string_lossy()))
        .unwrap_or_else(|| " Preview ".to_owned());

    let border_color = if focused { theme::PINK } else { theme::BORDER };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme::BG))
        .title(Span::styled(title, Style::default().fg(theme::TEXT)));

    match &state.preview_content {
        PreviewContent::Empty => {
            let hint = Paragraph::new("Select a file to preview.")
                .block(block)
                .style(Style::default().fg(theme::MUTED))
                .alignment(Alignment::Center);
            f.render_widget(hint, area);
        }
        PreviewContent::Error(msg) => {
            let para = Paragraph::new(msg.as_str())
                .block(block)
                .style(Style::default().fg(theme::PINK))
                .wrap(Wrap { trim: false });
            f.render_widget(para, area);
        }
        PreviewContent::Highlighted(lines) => {
            let text = Text::from(lines.clone());
            let para = Paragraph::new(text)
                .block(block)
                .scroll((state.preview_scroll as u16, 0));
            f.render_widget(para, area);
        }
        PreviewContent::Markdown(lines) => {
            let text = Text::from(lines.clone());
            let para = Paragraph::new(text)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((state.preview_scroll as u16, 0));
            f.render_widget(para, area);
        }
    }
}
