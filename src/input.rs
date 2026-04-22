//! Pure key-event → AppAction mapping.
//!
//! `map_key` takes the current mode and focus so the same physical key can
//! mean different things (e.g. `j` moves the tree cursor in Tree focus but
//! scrolls the preview in Preview focus). Keeping this logic in its own module
//! makes it trivially unit-testable without a running terminal.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{AppMode, Focus};

#[derive(Debug, PartialEq)]
pub enum AppAction {
    Quit,
    // Tree navigation
    CursorUp,
    CursorDown,
    EnterOrExpand,
    ParentDir,
    // Search
    OpenSearch,
    CloseSearch,
    SearchConfirm,
    SearchInput(char),
    SearchBackspace,
    // File deletion
    DeleteSelected,
    ConfirmDelete,
    CancelDelete,
    // File rename
    RenameSelected,
    RenameInput(char),
    RenameBackspace,
    RenameRight,
    RenameLeft,
    RenameTab,
    ConfirmRename,
    CancelRename,
    // Preview
    ToggleFocus,
    PreviewScrollDown,
    PreviewScrollUp,
    PreviewPageDown,
    PreviewPageUp,
    PreviewTop,
    PreviewBottom,
    NoOp,
}

pub fn map_key(key: KeyEvent, mode: &AppMode, focus: &Focus) -> AppAction {
    match mode {
        AppMode::Browse => match focus {
            Focus::Tree => map_tree(key),
            Focus::Preview => map_preview(key),
        },
        AppMode::Search => map_search(key),
        AppMode::DeleteConfirm => map_delete_confirm(key),
        AppMode::Rename => map_rename(key),
    }
}

fn map_tree(key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Char('q') => AppAction::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => AppAction::Quit,
        KeyCode::Char('k') | KeyCode::Up => AppAction::CursorUp,
        KeyCode::Char('j') | KeyCode::Down => AppAction::CursorDown,
        KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => AppAction::EnterOrExpand,
        KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => AppAction::ParentDir,
        KeyCode::Char('/') => AppAction::OpenSearch,
        KeyCode::Char('d') => AppAction::DeleteSelected,
        KeyCode::Char('r') => AppAction::RenameSelected,
        KeyCode::Tab => AppAction::ToggleFocus,
        _ => AppAction::NoOp,
    }
}

fn map_preview(key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Char('q') => AppAction::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => AppAction::Quit,
        KeyCode::Char('j') | KeyCode::Down => AppAction::PreviewScrollDown,
        KeyCode::Char('k') | KeyCode::Up => AppAction::PreviewScrollUp,
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => AppAction::PreviewPageDown,
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => AppAction::PreviewPageUp,
        KeyCode::Char('g') => AppAction::PreviewTop,
        KeyCode::Char('G') => AppAction::PreviewBottom,
        // h, Esc, and Tab all return focus to the tree for easy one-handed use.
        KeyCode::Char('h') | KeyCode::Left | KeyCode::Esc | KeyCode::Tab => AppAction::ToggleFocus,
        _ => AppAction::NoOp,
    }
}

fn map_search(key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc => AppAction::CloseSearch,
        KeyCode::Enter => AppAction::SearchConfirm,
        KeyCode::Up | KeyCode::Char('k') => AppAction::CursorUp,
        KeyCode::Down | KeyCode::Char('j') => AppAction::CursorDown,
        KeyCode::Backspace => AppAction::SearchBackspace,
        KeyCode::Char(c) => AppAction::SearchInput(c),
        _ => AppAction::NoOp,
    }
}

fn map_rename(key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Enter => AppAction::ConfirmRename,
        KeyCode::Esc => AppAction::CancelRename,
        KeyCode::Tab => AppAction::RenameTab,
        KeyCode::Right => AppAction::RenameRight,
        KeyCode::Left => AppAction::RenameLeft,
        KeyCode::Backspace => AppAction::RenameBackspace,
        KeyCode::Char(c) => AppAction::RenameInput(c),
        _ => AppAction::NoOp,
    }
}

fn map_delete_confirm(key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => AppAction::ConfirmDelete,
        KeyCode::Char('n') | KeyCode::Esc => AppAction::CancelDelete,
        _ => AppAction::NoOp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn browse_quit() {
        assert_eq!(map_key(key(KeyCode::Char('q')), &AppMode::Browse, &Focus::Tree), AppAction::Quit);
    }

    #[test]
    fn browse_vim_nav() {
        assert_eq!(map_key(key(KeyCode::Char('j')), &AppMode::Browse, &Focus::Tree), AppAction::CursorDown);
        assert_eq!(map_key(key(KeyCode::Char('k')), &AppMode::Browse, &Focus::Tree), AppAction::CursorUp);
        assert_eq!(map_key(key(KeyCode::Char('l')), &AppMode::Browse, &Focus::Tree), AppAction::EnterOrExpand);
        assert_eq!(map_key(key(KeyCode::Char('h')), &AppMode::Browse, &Focus::Tree), AppAction::ParentDir);
    }

    #[test]
    fn browse_open_search() {
        assert_eq!(map_key(key(KeyCode::Char('/')), &AppMode::Browse, &Focus::Tree), AppAction::OpenSearch);
    }

    #[test]
    fn search_escape_closes() {
        assert_eq!(map_key(key(KeyCode::Esc), &AppMode::Search, &Focus::Tree), AppAction::CloseSearch);
    }

    #[test]
    fn search_char_input() {
        assert_eq!(
            map_key(key(KeyCode::Char('a')), &AppMode::Search, &Focus::Tree),
            AppAction::SearchInput('a')
        );
    }

    #[test]
    fn delete_confirm_y() {
        assert_eq!(
            map_key(key(KeyCode::Char('y')), &AppMode::DeleteConfirm, &Focus::Tree),
            AppAction::ConfirmDelete
        );
    }

    #[test]
    fn delete_confirm_n() {
        assert_eq!(
            map_key(key(KeyCode::Char('n')), &AppMode::DeleteConfirm, &Focus::Tree),
            AppAction::CancelDelete
        );
    }

    #[test]
    fn preview_scroll() {
        assert_eq!(map_key(key(KeyCode::Char('j')), &AppMode::Browse, &Focus::Preview), AppAction::PreviewScrollDown);
        assert_eq!(map_key(key(KeyCode::Char('k')), &AppMode::Browse, &Focus::Preview), AppAction::PreviewScrollUp);
        assert_eq!(map_key(ctrl(KeyCode::Char('d')), &AppMode::Browse, &Focus::Preview), AppAction::PreviewPageDown);
        assert_eq!(map_key(ctrl(KeyCode::Char('u')), &AppMode::Browse, &Focus::Preview), AppAction::PreviewPageUp);
    }

    #[test]
    fn preview_top_bottom() {
        assert_eq!(map_key(key(KeyCode::Char('g')), &AppMode::Browse, &Focus::Preview), AppAction::PreviewTop);
        assert_eq!(map_key(key(KeyCode::Char('G')), &AppMode::Browse, &Focus::Preview), AppAction::PreviewBottom);
    }

    #[test]
    fn tab_toggles_focus() {
        assert_eq!(map_key(key(KeyCode::Tab), &AppMode::Browse, &Focus::Tree), AppAction::ToggleFocus);
        assert_eq!(map_key(key(KeyCode::Tab), &AppMode::Browse, &Focus::Preview), AppAction::ToggleFocus);
    }
}
