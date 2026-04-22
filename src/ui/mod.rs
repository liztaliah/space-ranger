//! Top-level render function and layout.
//!
//! ratatui is immediate-mode: this function redraws the entire terminal from
//! scratch every frame. There is no diffing or retained widget state — each
//! call to `render` produces a complete description of what to display.

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

    let bottom_height = if state.mode == AppMode::Search { 3 } else { 1 };
    let [main_area, bottom_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(bottom_height),
    ])
    .areas(area);

    // Split main area 30% tree / 70% preview.
    let [tree_area, preview_area] = Layout::horizontal([
        Constraint::Percentage(30),
        Constraint::Percentage(70),
    ])
    .areas(main_area);

    tree::render(f, state, tree_area);
    preview::render(f, state, preview_area);

    match state.mode {
        AppMode::Search => search::render(f, state, bottom_area),
        AppMode::Browse | AppMode::DeleteConfirm | AppMode::Rename => hints::render(f, state, bottom_area),
    }

    // Modals are drawn last so they paint over both panels.
    if state.mode == AppMode::DeleteConfirm {
        modal::render(f, state, area);
    }
    if state.mode == AppMode::Rename {
        modal::render_rename(f, state, area);
    }
}
