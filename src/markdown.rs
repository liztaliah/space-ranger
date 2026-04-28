//! Renders markdown to ratatui Lines using termimad.
//!
//! termimad outputs ANSI-escaped strings. We parse those sequences into
//! ratatui Spans with proper Style values so that Paragraph::scroll() can
//! compute viewport offsets correctly over styled text. Passing raw ANSI bytes
//! as plain strings causes escape sequences to appear as literal text when
//! scrolling because ratatui counts escape bytes as visible characters.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use termimad::MadSkin;

/// Render `content` as markdown, wrapping to `width` columns.
/// Returns one `Line` per output line, ready to pass to a `Paragraph` widget.
pub fn render_markdown(content: &str, width: u16) -> Vec<Line<'static>> {
    let skin = MadSkin::default();
    let text = skin.text(content, Some(width as usize));
    let rendered = format!("{}", text);
    ansi_to_lines(&rendered)
}

/// Convert an ANSI-escaped string into ratatui Lines with styled Spans.
fn ansi_to_lines(s: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut style = Style::default();
    let mut buf = String::new();

    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next(); // consume '['
            let mut seq = String::new();
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    if c == 'm' {
                        if !buf.is_empty() {
                            spans.push(Span::styled(std::mem::take(&mut buf), style));
                        }
                        style = apply_sgr(style, &seq);
                    }
                    // skip non-'m' CSI commands entirely
                    break;
                }
                seq.push(c);
            }
        } else if ch == '\n' {
            if !buf.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut buf), style));
            }
            lines.push(Line::from(std::mem::take(&mut spans)));
        } else {
            buf.push(ch);
        }
    }

    if !buf.is_empty() {
        spans.push(Span::styled(buf, style));
    }
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }

    lines
}

/// Apply a single SGR parameter string (e.g. "1;38;5;235") to a Style.
fn apply_sgr(mut style: Style, params: &str) -> Style {
    let codes: Vec<&str> = if params.is_empty() { vec!["0"] } else { params.split(';').collect() };
    let mut i = 0;
    while i < codes.len() {
        match codes[i] {
            "0" => style = Style::default(),
            "1" => style = style.add_modifier(Modifier::BOLD),
            "2" => style = style.add_modifier(Modifier::DIM),
            "3" => style = style.add_modifier(Modifier::ITALIC),
            "4" => style = style.add_modifier(Modifier::UNDERLINED),
            "9" => style = style.add_modifier(Modifier::CROSSED_OUT),
            "22" => style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            "23" => style = style.remove_modifier(Modifier::ITALIC),
            "24" => style = style.remove_modifier(Modifier::UNDERLINED),
            "39" => style = style.fg(Color::Reset),
            "49" => style = style.bg(Color::Reset),
            "38" if i + 2 < codes.len() && codes[i + 1] == "5" => {
                if let Ok(n) = codes[i + 2].parse::<u8>() {
                    style = style.fg(Color::Indexed(n));
                }
                i += 2;
            }
            "38" if i + 4 < codes.len() && codes[i + 1] == "2" => {
                let r = codes[i + 2].parse::<u8>().unwrap_or(0);
                let g = codes[i + 3].parse::<u8>().unwrap_or(0);
                let b = codes[i + 4].parse::<u8>().unwrap_or(0);
                style = style.fg(Color::Rgb(r, g, b));
                i += 4;
            }
            "48" if i + 2 < codes.len() && codes[i + 1] == "5" => {
                if let Ok(n) = codes[i + 2].parse::<u8>() {
                    style = style.bg(Color::Indexed(n));
                }
                i += 2;
            }
            "48" if i + 4 < codes.len() && codes[i + 1] == "2" => {
                let r = codes[i + 2].parse::<u8>().unwrap_or(0);
                let g = codes[i + 3].parse::<u8>().unwrap_or(0);
                let b = codes[i + 4].parse::<u8>().unwrap_or(0);
                style = style.bg(Color::Rgb(r, g, b));
                i += 4;
            }
            s => {
                if let Ok(n) = s.parse::<u8>() {
                    style = match n {
                        30..=37 => style.fg(Color::Indexed(n - 30)),
                        40..=47 => style.bg(Color::Indexed(n - 40)),
                        90..=97 => style.fg(Color::Indexed(n - 90 + 8)),
                        100..=107 => style.bg(Color::Indexed(n - 100 + 8)),
                        _ => style,
                    };
                }
            }
        }
        i += 1;
    }
    style
}
