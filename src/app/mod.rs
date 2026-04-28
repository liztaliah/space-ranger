//! Application state and all state transitions.
//!
//! `AppState` is the single source of truth for the entire UI. The render
//! functions in `ui/` read it; `apply()` is the only way to mutate it.
//! This makes the data flow easy to follow: key event → AppAction → apply().

mod nav;
mod preview;
mod search;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use ratatui::text::Line;

use crate::fs as fsx;
use crate::highlight::Highlighter;
use crate::input::AppAction;

struct CacheEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

type SearchResult = Vec<(String, PathBuf, bool)>;

/// Top-level mode — determines which key map is active.
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Browse,
    Search,
    DeleteConfirm,
    Rename,
}

/// Which panel currently receives keyboard input.
#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Tree,
    Preview,
}

/// A single visible row in the directory listing.
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub depth: usize,
}

/// What the preview panel is currently showing.
#[derive(Clone)]
pub enum PreviewContent {
    Empty,
    Loading,
    Error(String),
    Highlighted(Vec<Line<'static>>),
    Markdown(Vec<Line<'static>>),
}

pub struct AppState {
    pub root: PathBuf,
    pub entries: Vec<DirEntry>,
    pub cursor: usize,
    pub scroll_offset: usize,
    /// Entries from the parent directory — shown in the middle-left column.
    pub parent_entries: Vec<DirEntry>,
    /// Index into parent_entries that corresponds to the current root.
    pub parent_cursor: usize,
    /// Entries from the grandparent directory — shown in the far-left column.
    pub grandparent_entries: Vec<DirEntry>,
    /// Index into grandparent_entries that corresponds to the parent of root.
    pub grandparent_cursor: usize,
    pub selected_path: Option<PathBuf>,
    pub preview_content: PreviewContent,
    pub preview_scroll: usize,
    /// Whether the preview pane is currently visible.
    pub preview_open: bool,
    /// Which directory column currently has keyboard focus (0=grandparent, 1=parent, 2=current).
    pub focused_col: usize,
    pub mode: AppMode,
    pub search_query: String,
    pub delete_target: Option<PathBuf>,
    pub rename_target: Option<PathBuf>,
    pub rename_stem: String,
    pub rename_ext: String,
    pub rename_fresh: bool,
    pub rename_cancel_focused: bool,
    pub rename_ext_focused: bool,
    pub rename_ext_fresh: bool,
    pub terminal_size: (u16, u16),
    pub highlighter: Option<Arc<Mutex<Highlighter>>>,
    pub should_quit: bool,
    pub focus: Focus,
    pub search_loading: bool,
    search_root: PathBuf,
    search_cache: Vec<CacheEntry>,
    search_rx: Option<Receiver<SearchResult>>,
    preview_result_rx: Option<Receiver<(PathBuf, PreviewContent)>>,
    preview_cache: Vec<(PathBuf, PreviewContent)>,
}

impl AppState {
    pub fn new(root: PathBuf) -> Result<Self> {
        let mut state = Self {
            root: root.clone(),
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            parent_entries: Vec::new(),
            parent_cursor: 0,
            grandparent_entries: Vec::new(),
            grandparent_cursor: 0,
            selected_path: None,
            preview_content: PreviewContent::Empty,
            preview_scroll: 0,
            preview_open: false,
            focused_col: 2,
            mode: AppMode::Browse,
            search_query: String::new(),
            delete_target: None,
            rename_target: None,
            rename_stem: String::new(),
            rename_ext: String::new(),
            rename_fresh: false,
            rename_cancel_focused: false,
            rename_ext_focused: false,
            rename_ext_fresh: false,
            terminal_size: (80, 24),
            highlighter: None,
            should_quit: false,
            focus: Focus::Tree,
            search_loading: false,
            search_root: root.clone(),
            search_cache: Vec::new(),
            search_rx: None,
            preview_result_rx: None,
            preview_cache: Vec::new(),
        };
        state.load_entries();
        Ok(state)
    }

    pub fn apply(&mut self, action: AppAction) -> Result<()> {
        match action {
            AppAction::Quit => self.should_quit = true,
            AppAction::CursorUp => self.move_cursor(-1),
            AppAction::CursorDown => self.move_cursor(1),
            AppAction::FocusRight => {
                if self.focused_col < 2 {
                    self.focused_col += 1;
                } else {
                    self.enter_or_expand()?;
                }
            }
            AppAction::EnterOrExpand => self.enter_or_expand()?,
            AppAction::ParentDir => {
                if self.focused_col > 0 {
                    self.focused_col -= 1;
                } else {
                    // Block sliding further left once / is the leftmost column
                    // (i.e. no great-grandparent exists).
                    let has_great_grandparent = self.root
                        .parent()
                        .and_then(|p| p.parent())
                        .and_then(|p| p.parent())
                        .is_some();
                    if has_great_grandparent {
                        self.go_parent();
                        self.focused_col = 2;
                    }
                }
            }

            AppAction::OpenSearch => {
                // Search from whichever directory is currently selected in the focused column.
                let search_root = match self.focused_col {
                    0 => self.grandparent_entries.get(self.grandparent_cursor)
                        .filter(|e| e.is_dir)
                        .map(|e| e.path.clone())
                        .or_else(|| self.root.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf()))
                        .unwrap_or_else(|| self.root.clone()),
                    1 => self.parent_entries.get(self.parent_cursor)
                        .filter(|e| e.is_dir)
                        .map(|e| e.path.clone())
                        .or_else(|| self.root.parent().map(|p| p.to_path_buf()))
                        .unwrap_or_else(|| self.root.clone()),
                    _ => self.entries.get(self.cursor)
                        .filter(|e| e.is_dir)
                        .map(|e| e.path.clone())
                        .unwrap_or_else(|| self.root.clone()),
                };
                self.search_root = search_root.clone();
                self.mode = AppMode::Search;
                self.search_query.clear();
                self.entries.clear();
                self.search_loading = true;
                self.search_cache.clear();

                let (tx, rx) = mpsc::channel();
                self.search_rx = Some(rx);
                std::thread::spawn(move || {
                    let results: SearchResult = fsx::read_dir_sorted(&search_root)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|e| (e.name, e.path, e.is_dir))
                        .collect();
                    let _ = tx.send(results);
                });
            }

            AppAction::CloseSearch => {
                self.mode = AppMode::Browse;
                self.search_query.clear();
                self.search_rx = None;
                self.search_loading = false;
                self.search_cache.clear();
                self.load_entries();
                self.cursor = 0;
                self.scroll_offset = 0;
                self.focused_col = 2;
            }

            AppAction::SearchConfirm => {
                let selected = self.entries.get(self.cursor).map(|e| e.path.clone());
                let selected_is_file = self.entries.get(self.cursor).map(|e| !e.is_dir).unwrap_or(false);

                self.mode = AppMode::Browse;
                self.search_query.clear();
                self.search_rx = None;
                self.search_loading = false;
                self.search_cache.clear();

                if self.search_root != self.root {
                    self.root = self.search_root.clone();
                }
                self.load_entries();
                self.cursor = 0;
                self.scroll_offset = 0;

                if let Some(path) = selected {
                    if let Some(idx) = self.entries.iter().position(|e| e.path == path) {
                        self.cursor = idx;
                        self.update_scroll();
                        if selected_is_file {
                            self.preview_open = true;
                            self.load_preview(&path);
                            self.focus = Focus::Preview;
                        }
                    }
                }
            }

            AppAction::SearchInput(c) => {
                self.search_query.push(c);
                self.apply_search_filter();
                self.cursor = 0;
                self.scroll_offset = 0;
            }

            AppAction::SearchBackspace => {
                self.search_query.pop();
                self.apply_search_filter();
                self.cursor = 0;
                self.scroll_offset = 0;
            }

            AppAction::DeleteSelected => {
                let target = self.entries.get(self.cursor)
                    .filter(|e| !e.is_dir)
                    .map(|e| e.path.clone());
                if let Some(path) = target {
                    self.delete_target = Some(path);
                    self.mode = AppMode::DeleteConfirm;
                }
            }

            AppAction::ConfirmDelete => {
                if let Some(path) = self.delete_target.take() {
                    match fsx::delete_file(&path) {
                        Ok(_) => {
                            self.selected_path = None;
                            self.preview_content = PreviewContent::Empty;
                            self.preview_open = false;
                        }
                        Err(e) => {
                            self.preview_content = PreviewContent::Error(e.to_string());
                        }
                    }
                    self.mode = AppMode::Browse;
                    self.load_entries();
                    if self.cursor >= self.entries.len() && !self.entries.is_empty() {
                        self.cursor = self.entries.len() - 1;
                    }
                }
            }

            AppAction::CancelDelete => {
                self.delete_target = None;
                self.mode = AppMode::Browse;
            }

            AppAction::RenameSelected => {
                let path = self.entries.get(self.cursor)
                    .filter(|e| !e.is_dir)
                    .map(|e| e.path.clone());
                if let Some(path) = path {
                    self.rename_stem = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    self.rename_ext = path
                        .extension()
                        .map(|e| format!(".{}", e.to_string_lossy()))
                        .unwrap_or_default();
                    self.rename_target = Some(path);
                    self.rename_fresh = true;
                    self.rename_cancel_focused = false;
                    self.rename_ext_focused = false;
                    self.rename_ext_fresh = false;
                    self.mode = AppMode::Rename;
                }
            }

            AppAction::RenameInput(c) => {
                if self.rename_ext_focused {
                    let was_ext_fresh = self.rename_ext_fresh;
                    self.rename_ext_fresh = false;
                    if was_ext_fresh {
                        self.rename_ext = format!(".{}", c);
                    } else {
                        self.rename_ext.push(c);
                    }
                } else {
                    let was_fresh = self.rename_fresh;
                    self.rename_fresh = false;
                    if was_fresh {
                        self.rename_stem = c.to_string();
                    } else {
                        self.rename_stem.push(c);
                    }
                }
            }

            AppAction::RenameBackspace => {
                if self.rename_ext_focused {
                    if self.rename_ext_fresh {
                        self.rename_ext_fresh = false;
                    } else if self.rename_ext.len() > 1 {
                        self.rename_ext.pop();
                    } else {
                        self.rename_ext.clear();
                        self.rename_ext_focused = false;
                    }
                } else if self.rename_fresh {
                    self.rename_fresh = false;
                } else if !self.rename_stem.is_empty() {
                    self.rename_stem.pop();
                }
            }

            AppAction::RenameRight => {
                self.rename_fresh = false;
                if !self.rename_ext_focused && !self.rename_ext.is_empty() {
                    self.rename_ext_focused = true;
                    self.rename_ext_fresh = true;
                }
            }

            AppAction::RenameLeft => {
                self.rename_fresh = false;
                self.rename_ext_focused = false;
                self.rename_ext_fresh = false;
            }

            AppAction::RenameTab => {
                self.rename_cancel_focused = !self.rename_cancel_focused;
            }

            AppAction::ConfirmRename => {
                if self.rename_cancel_focused {
                    self.rename_target = None;
                    self.mode = AppMode::Browse;
                } else if let Some(path) = self.rename_target.take() {
                    let new_name = format!("{}{}", self.rename_stem.trim(), self.rename_ext);
                    if !new_name.is_empty() && new_name != "." {
                        match fsx::rename_file(&path, &new_name) {
                            Ok(_) => {
                                self.selected_path = None;
                                self.preview_content = PreviewContent::Empty;
                                self.preview_open = false;
                            }
                            Err(e) => {
                                self.preview_content = PreviewContent::Error(e.to_string());
                            }
                        }
                        self.load_entries();
                    }
                    self.mode = AppMode::Browse;
                }
                self.rename_stem.clear();
                self.rename_ext.clear();
            }

            AppAction::CancelRename => {
                self.rename_target = None;
                self.rename_stem.clear();
                self.rename_ext.clear();
                self.mode = AppMode::Browse;
            }

            AppAction::ToggleFocus => {
                if self.focus == Focus::Preview {
                    // Close preview pane and return to tree.
                    self.focus = Focus::Tree;
                    self.preview_open = false;
                    self.preview_content = PreviewContent::Empty;
                    self.selected_path = None;
                } else if self.preview_open {
                    self.focus = Focus::Preview;
                }
            }

            AppAction::PreviewScrollDown => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
            }
            AppAction::PreviewScrollUp => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
            }
            AppAction::PreviewPageDown => {
                let page = self.preview_page_size();
                self.preview_scroll = self.preview_scroll.saturating_add(page);
            }
            AppAction::PreviewPageUp => {
                let page = self.preview_page_size();
                self.preview_scroll = self.preview_scroll.saturating_sub(page);
            }
            AppAction::PreviewTop => {
                self.preview_scroll = 0;
            }
            AppAction::PreviewBottom => {
                let total = self.preview_line_count();
                let page = self.preview_page_size();
                self.preview_scroll = total.saturating_sub(page);
            }

            AppAction::NoOp => {}
        }
        Ok(())
    }

    /// Called every frame. Receives the result of the background preview thread
    /// and stores it in both the active preview slot and the cache.
    pub fn poll_preview_result(&mut self) {
        let ready = self
            .preview_result_rx
            .as_ref()
            .and_then(|rx| rx.try_recv().ok());
        if let Some((path, content)) = ready {
            self.preview_result_rx = None;
            if self.selected_path.as_deref() == Some(path.as_path()) {
                self.cache_preview(path, content.clone());
                self.preview_content = content;
            }
        }
    }

    pub fn poll_search_cache(&mut self) {
        let ready = self
            .search_rx
            .as_ref()
            .and_then(|rx| rx.try_recv().ok());
        if let Some(results) = ready {
            self.search_cache = results
                .into_iter()
                .map(|(name, path, is_dir)| CacheEntry { name, path, is_dir })
                .collect();
            self.search_rx = None;
            self.search_loading = false;
            self.apply_search_filter();
        }
    }
}
