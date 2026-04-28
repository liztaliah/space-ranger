//! Top-level render function and layout.

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

    if state.preview_open {
        // Two-column layout when a file is open: current dir | file preview.
        let [tree_area, preview_area] = Layout::horizontal([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(2, 3),
        ])
        .areas(main_area);
        tree::render(f, state, tree_area);
        preview::render(f, state, preview_area);
    } else {
        // Three-column layout browsing: grandparent | parent | current dir.
        let [grandparent_area, parent_area, tree_area] = Layout::horizontal([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .areas(main_area);
        tree::render_grandparent(f, state, grandparent_area);
        tree::render_parent(f, state, parent_area);
        tree::render(f, state, tree_area);
    }

    match state.mode {
        AppMode::Search => search::render(f, state, bottom_area),
        AppMode::Browse | AppMode::DeleteConfirm | AppMode::Rename => hints::render(f, state, bottom_area),
    }

    if state.mode == AppMode::DeleteConfirm {
        modal::render(f, state, area);
    }
    if state.mode == AppMode::Rename {
        modal::render_rename(f, state, area);
    }
}
