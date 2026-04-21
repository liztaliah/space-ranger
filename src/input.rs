use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::AppMode;

#[derive(Debug, PartialEq)]
pub enum AppAction {
    Quit,
    CursorUp,
    CursorDown,
    EnterOrExpand,
    ParentDir,
    OpenSearch,
    CloseSearch,
    SearchInput(char),
    SearchBackspace,
    DeleteSelected,
    ConfirmDelete,
    CancelDelete,
    NoOp,
}

pub fn map_key(key: KeyEvent, mode: &AppMode) -> AppAction {
    match mode {
        AppMode::Browse => map_browse(key),
        AppMode::Search => map_search(key),
        AppMode::DeleteConfirm => map_delete_confirm(key),
    }
}

fn map_browse(key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Char('q') => AppAction::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => AppAction::Quit,
        KeyCode::Char('k') | KeyCode::Up => AppAction::CursorUp,
        KeyCode::Char('j') | KeyCode::Down => AppAction::CursorDown,
        KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => AppAction::EnterOrExpand,
        KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => AppAction::ParentDir,
        KeyCode::Char('/') => AppAction::OpenSearch,
        KeyCode::Char('d') => AppAction::DeleteSelected,
        _ => AppAction::NoOp,
    }
}

fn map_search(key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc => AppAction::CloseSearch,
        KeyCode::Enter => AppAction::CloseSearch,
        KeyCode::Backspace => AppAction::SearchBackspace,
        KeyCode::Char(c) => AppAction::SearchInput(c),
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

    #[test]
    fn browse_quit() {
        assert_eq!(map_key(key(KeyCode::Char('q')), &AppMode::Browse), AppAction::Quit);
    }

    #[test]
    fn browse_vim_nav() {
        assert_eq!(map_key(key(KeyCode::Char('j')), &AppMode::Browse), AppAction::CursorDown);
        assert_eq!(map_key(key(KeyCode::Char('k')), &AppMode::Browse), AppAction::CursorUp);
        assert_eq!(map_key(key(KeyCode::Char('l')), &AppMode::Browse), AppAction::EnterOrExpand);
        assert_eq!(map_key(key(KeyCode::Char('h')), &AppMode::Browse), AppAction::ParentDir);
    }

    #[test]
    fn browse_open_search() {
        assert_eq!(map_key(key(KeyCode::Char('/')), &AppMode::Browse), AppAction::OpenSearch);
    }

    #[test]
    fn search_escape_closes() {
        assert_eq!(map_key(key(KeyCode::Esc), &AppMode::Search), AppAction::CloseSearch);
    }

    #[test]
    fn search_char_input() {
        assert_eq!(
            map_key(key(KeyCode::Char('a')), &AppMode::Search),
            AppAction::SearchInput('a')
        );
    }

    #[test]
    fn delete_confirm_y() {
        assert_eq!(
            map_key(key(KeyCode::Char('y')), &AppMode::DeleteConfirm),
            AppAction::ConfirmDelete
        );
    }

    #[test]
    fn delete_confirm_n() {
        assert_eq!(
            map_key(key(KeyCode::Char('n')), &AppMode::DeleteConfirm),
            AppAction::CancelDelete
        );
    }
}
