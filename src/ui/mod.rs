mod hints;
pub mod modal;
pub mod preview;
pub mod search;
pub mod tree;
pub mod theme;

use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;

use crate::app::{AppMode, AppState};

pub fn render(f: &mut Frame, state: &AppState) {
    let area = f.area();

    let [main_area, bottom_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    let [tree_area, preview_area] = Layout::horizontal([
        Constraint::Percentage(30),
        Constraint::Percentage(70),
    ])
    .areas(main_area);

    tree::render(f, state, tree_area);
    preview::render(f, state, preview_area);

    match state.mode {
        AppMode::Search => search::render(f, state, bottom_area),
        AppMode::Browse | AppMode::DeleteConfirm => hints::render(f, bottom_area),
    }

    if state.mode == AppMode::DeleteConfirm {
        modal::render(f, state, area);
    }
}
