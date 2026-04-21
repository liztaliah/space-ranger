//! Converts syntect syntax-highlighted text into ratatui Spans.
//!
//! Highlighting is expensive to initialise (~300ms for the default syntax set),
//! so `Highlighter` is constructed lazily on first use via AppState's
//! `Option<Highlighter>` field rather than at app startup.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Highlight `text` using the syntax for `extension` and return it as a
    /// `Vec<Line<'static>>`. Lines are owned (`.to_owned()` on each span's text)
    /// so they can be stored directly in `AppState` and rendered without
    /// re-running the highlighter each frame.
    pub fn highlight_file(&self, text: &str, extension: &str) -> Vec<Line<'static>> {
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(extension)
            // Fall back to plain text so unknown extensions still display.
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        text.lines()
            .map(|line| {
                let ranges = h
                    .highlight_line(line, &self.syntax_set)
                    .unwrap_or_default();
                let spans: Vec<Span<'static>> = ranges
                    .iter()
                    .map(|(style, text)| syntect_to_span(style, text))
                    .collect();
                Line::from(spans)
            })
            .collect()
    }
}

/// Convert a syntect (style, text) pair into a ratatui Span with an owned
/// string. The `'static` lifetime comes from the `.to_owned()` call.
fn syntect_to_span(style: &SyntectStyle, text: &str) -> Span<'static> {
    let fg = style.foreground;
    let mut ratatui_style = Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b));
    if style.font_style.contains(FontStyle::BOLD) {
        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
    }
    Span::styled(text.to_owned(), ratatui_style)
}
