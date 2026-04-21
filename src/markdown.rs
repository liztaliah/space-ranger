use ratatui::text::Line;
use termimad::MadSkin;

pub fn render_markdown(content: &str, width: u16) -> Vec<Line<'static>> {
    let skin = MadSkin::default();
    let text = skin.text(content, Some(width as usize));
    let rendered = format!("{}", text);
    rendered
        .lines()
        .map(|l| Line::from(l.to_owned()))
        .collect()
}
