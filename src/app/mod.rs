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
use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::text::Line;

use crate::fs as fsx;
use crate::highlight::Highlighter;
use crate::input::AppAction;

// Internal type for the search background thread's results. Uses plain tuples
// so it crosses the thread boundary without requiring CacheEntry to be Send.
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
}

/// Which panel currently receives keyboard input.
#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Tree,
    Preview,
}

/// A single visible row in the directory tree.
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    /// Nesting level — used for indentation in the tree panel.
    pub depth: usize,
    pub is_expanded: bool,
}

/// What the preview panel is currently showing.
#[derive(Clone)]
pub enum PreviewContent {
    Empty,
    /// Background thread is loading/highlighting the file.
    Loading,
    Error(String),
    /// Pre-rendered syntax-highlighted lines. Stored as owned Spans ('static)
    /// so rendering is zero-cost — highlighting only runs on file selection.
    Highlighted(Vec<Line<'static>>),
    Markdown(Vec<Line<'static>>),
}

pub struct AppState {
    /// Current root directory displayed in the tree panel.
    pub root: PathBuf,
    /// Flat list of currently visible tree nodes (expanded dirs inline their children).
    pub entries: Vec<DirEntry>,
    /// Index of the highlighted row in `entries`.
    pub cursor: usize,
    /// First visible row — kept in sync with cursor by update_scroll().
    pub scroll_offset: usize,
    pub selected_path: Option<PathBuf>,
    pub preview_content: PreviewContent,
    pub preview_scroll: usize,
    pub mode: AppMode,
    pub search_query: String,
    pub delete_target: Option<PathBuf>,
    pub terminal_size: (u16, u16),
    /// Shared with background preview threads — initialized on first preview.
    pub highlighter: Option<Arc<Mutex<Highlighter>>>,
    pub should_quit: bool,
    pub focus: Focus,
    /// True while the background search-cache walk is still running.
    pub search_loading: bool,
    /// Flat list of every file under root, built once per search session.
    search_cache: Vec<CacheEntry>,
    /// Receives the completed cache from the background thread.
    search_rx: Option<Receiver<SearchResult>>,
    /// File to preview once the debounce delay has elapsed.
    preview_pending_path: Option<PathBuf>,
    preview_pending_since: Option<Instant>,
    /// Receives the result of the in-flight background preview load.
    preview_result_rx: Option<Receiver<(PathBuf, PreviewContent)>>,
    /// Recently rendered previews keyed by path — avoids re-reading disk on revisit.
    preview_cache: Vec<(PathBuf, PreviewContent)>,
}

impl AppState {
    pub fn new(root: PathBuf) -> Result<Self> {
        let mut state = Self {
            root: root.clone(),
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            selected_path: None,
            preview_content: PreviewContent::Empty,
            preview_scroll: 0,
            mode: AppMode::Browse,
            search_query: String::new(),
            delete_target: None,
            terminal_size: (80, 24),
            highlighter: None,
            should_quit: false,
            focus: Focus::Tree,
            search_loading: false,
            search_cache: Vec::new(),
            search_rx: None,
            preview_pending_path: None,
            preview_pending_since: None,
            preview_result_rx: None,
            preview_cache: Vec::new(),
        };
        state.load_entries();
        Ok(state)
    }

    /// Apply an action produced by `input::map_key`. This is the only place
    /// that mutates AppState — all UI code is read-only.
    pub fn apply(&mut self, action: AppAction) -> Result<()> {
        match action {
            AppAction::Quit => self.should_quit = true,
            AppAction::CursorUp => self.move_cursor(-1),
            AppAction::CursorDown => self.move_cursor(1),
            AppAction::EnterOrExpand => self.enter_or_expand()?,
            AppAction::ParentDir => self.go_parent(),

            AppAction::OpenSearch => {
                self.mode = AppMode::Search;
                self.search_query.clear();
                self.entries.clear();
                self.search_loading = true;
                self.search_cache.clear();

                // Search only the selected directory's immediate contents.
                // If the cursor is not on a directory, fall back to current root.
                let search_root = self
                    .entries
                    .get(self.cursor)
                    .filter(|entry| entry.is_dir)
                    .map(|entry| entry.path.clone())
                    .unwrap_or_else(|| self.root.clone());

                // Build the one-level cache in a background thread so UI input
                // remains responsive even for very large directories.
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
            }

            AppAction::SearchInput(c) => {
                self.search_query.push(c);
                // Filter runs against the in-memory cache — no disk I/O per keystroke.
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
                if let Some(path) = &self.selected_path {
                    if path.is_file() {
                        self.delete_target = Some(path.clone());
                        self.mode = AppMode::DeleteConfirm;
                    }
                }
            }

            AppAction::ConfirmDelete => {
                if let Some(path) = self.delete_target.take() {
                    match fsx::delete_file(&path) {
                        Ok(_) => {
                            self.selected_path = None;
                            self.preview_content = PreviewContent::Empty;
                        }
                        Err(e) => {
                            self.preview_content = PreviewContent::Error(e.to_string());
                        }
                    }
                    self.mode = AppMode::Browse;
                    self.load_entries();
                    // Clamp cursor in case the deleted file was at the end of the list.
                    if self.cursor >= self.entries.len() && !self.entries.is_empty() {
                        self.cursor = self.entries.len() - 1;
                    }
                }
            }

            AppAction::CancelDelete => {
                self.delete_target = None;
                self.mode = AppMode::Browse;
            }

            AppAction::ToggleFocus => {
                if self.focus == Focus::Preview {
                    self.focus = Focus::Tree;
                } else if !matches!(
                    self.preview_content,
                    PreviewContent::Empty | PreviewContent::Loading
                ) {
                    // Only allow focusing the preview when there's something to read.
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
                // Scroll to the last page rather than the very last line, so the
                // final lines sit at the top of the panel rather than the bottom.
                self.preview_scroll = total.saturating_sub(page);
            }

            AppAction::NoOp => {}
        }
        Ok(())
    }

    /// Called every frame. Fires `load_preview` once the cursor has been still
    /// for `delay` — avoids blocking the event loop while scrolling past files.
    pub fn poll_preview(&mut self, delay: Duration) {
        let ready = self
            .preview_pending_since
            .map(|since| since.elapsed() >= delay)
            .unwrap_or(false);

        if ready {
            if let Some(path) = self.preview_pending_path.take() {
                self.preview_pending_since = None;
                self.load_preview(&path);
            }
        }
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
            // Discard stale results if the cursor moved away while loading.
            if self.selected_path.as_deref() == Some(path.as_path()) {
                self.cache_preview(path, content.clone());
                self.preview_content = content;
            }
        }
    }

    /// Called every frame from the event loop. Non-blocking: if the background
    /// search thread hasn't finished yet, this is a no-op.
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
