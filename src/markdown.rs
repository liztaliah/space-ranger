//! Renders markdown to ratatui Lines using termimad.
//!
//! termimad outputs ANSI-escaped strings; we pass them through to ratatui
//! as raw owned strings. The crossterm backend forwards ANSI codes correctly,
//! so colours and basic formatting (bold, italic) are preserved without needing
//! to parse the escape sequences ourselves.

use ratatui::text::Line;
use termimad::MadSkin;

/// Render `content` as markdown, wrapping to `width` columns.
/// Returns one `Line` per output line, ready to pass to a `Paragraph` widget.
pub fn render_markdown(content: &str, width: u16) -> Vec<Line<'static>> {
    let skin = MadSkin::default();
    let text = skin.text(content, Some(width as usize));
    let rendered = format!("{}", text);
    rendered
        .lines()
        .map(|l| Line::from(l.to_owned()))
        .collect()
}
