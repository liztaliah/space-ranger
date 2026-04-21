//! Application state and all state transitions.
//!
//! `AppState` is the single source of truth for the entire UI. The render
//! functions in `ui/` read it; `apply()` is the only way to mutate it.
//! This makes the data flow easy to follow: key event → AppAction → apply().

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::text::Line;

use crate::fs as fsx;
use crate::highlight::Highlighter;
use crate::input::AppAction;
use crate::markdown::render_markdown;

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
                        .map(|e| {
                            let name = e.name;
                            let path = e.path;
                            let is_dir = e.is_dir;
                            (name, path, is_dir)
                        })
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

    /// Half the visible preview height — used for ctrl+d / ctrl+u page scrolling.
    fn preview_page_size(&self) -> usize {
        let h = self.terminal_size.1 as usize;
        let inner = h.saturating_sub(4); // subtract borders + hint bar
        (inner / 2).max(1)
    }

    fn preview_line_count(&self) -> usize {
        match &self.preview_content {
            PreviewContent::Highlighted(lines) | PreviewContent::Markdown(lines) => lines.len(),
            _ => 0,
        }
    }

    fn move_cursor(&mut self, delta: i32) {
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        let new = (self.cursor as i32 + delta).clamp(0, len as i32 - 1) as usize;
        self.cursor = new;
        self.update_scroll();

        let entry = &self.entries[self.cursor];
        if !entry.is_dir {
            // Debounce: record intent but don't load yet — poll_preview fires
            // after the cursor has been still for the debounce delay.
            self.preview_pending_path = Some(entry.path.clone());
            self.preview_pending_since = Some(Instant::now());
        } else {
            self.preview_pending_path = None;
            self.preview_pending_since = None;
            self.selected_path = None;
            self.preview_content = PreviewContent::Empty;
        }
    }

    /// Keep scroll_offset so the cursor row is always visible.
    fn update_scroll(&mut self) {
        let tree_height = self.tree_height();
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + tree_height {
            self.scroll_offset = self.cursor + 1 - tree_height;
        }
    }

    fn tree_height(&self) -> usize {
        let h = self.terminal_size.1 as usize;
        if h > 4 { h - 4 } else { 1 }
    }

    fn enter_or_expand(&mut self) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }
        let idx = self.cursor;
        let is_dir = self.entries[idx].is_dir;
        let is_expanded = self.entries[idx].is_expanded;
        let path = self.entries[idx].path.clone();

        if is_dir {
            if is_expanded {
                self.collapse_dir(idx);
            } else {
                self.expand_dir(idx)?;
            }
        } else {
            self.load_preview(&path);
        }
        Ok(())
    }

    /// Insert the directory's children into `entries` directly after `idx`.
    /// Using splice() instead of repeated insert() keeps this O(n) not O(n²).
    fn expand_dir(&mut self, idx: usize) -> Result<()> {
        self.entries[idx].is_expanded = true;
        let parent_depth = self.entries[idx].depth;
        let parent_path = self.entries[idx].path.clone();

        let children = fsx::read_dir_sorted(&parent_path).unwrap_or_default();
        let new_entries: Vec<DirEntry> = children
            .into_iter()
            .map(|e| DirEntry {
                name: e.name,
                path: e.path,
                is_dir: e.is_dir,
                depth: parent_depth + 1,
                is_expanded: false,
            })
            .collect();

        self.entries.splice(idx + 1..idx + 1, new_entries);
        Ok(())
    }

    /// Remove all descendants of the directory at `idx` from `entries`.
    /// Descendants are identified by having a depth greater than the directory's.
    fn collapse_dir(&mut self, idx: usize) {
        self.entries[idx].is_expanded = false;
        let depth = self.entries[idx].depth;
        let mut end = idx + 1;
        while end < self.entries.len() && self.entries[end].depth > depth {
            end += 1;
        }
        self.entries.drain(idx + 1..end);
        // Keep the cursor pointing at the same logical item after the drain.
        if self.cursor > idx && self.cursor < end {
            self.cursor = idx;
        } else if self.cursor >= end {
            self.cursor -= end - idx - 1;
        }
        self.update_scroll();
    }

    fn go_parent(&mut self) {
        if let Some(parent) = self.root.parent().map(|p| p.to_path_buf()) {
            self.root = parent;
            self.load_entries();
            self.cursor = 0;
            self.scroll_offset = 0;
            self.selected_path = None;
            self.preview_content = PreviewContent::Empty;
            self.focus = Focus::Tree;
        }
    }

    /// (Re)load the top-level entries for the current root directory.
    fn load_entries(&mut self) {
        self.entries = match fsx::read_dir_sorted(&self.root) {
            Ok(children) => children
                .into_iter()
                .map(|e| DirEntry {
                    name: e.name,
                    path: e.path,
                    is_dir: e.is_dir,
                    depth: 0,
                    is_expanded: false,
                })
                .collect(),
            Err(_) => Vec::new(),
        };
    }

    /// Filter search_cache by the current query string (case-insensitive substring match).
    fn apply_search_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.entries = self
            .search_cache
            .iter()
            .filter(|e| e.name.to_lowercase().contains(&query))
            .map(|e| DirEntry {
                name: e.name.clone(),
                path: e.path.clone(),
                is_dir: e.is_dir,
                depth: 0,
                is_expanded: false,
            })
            .collect();
    }

    /// Kick off a background thread to read and render `path`. Returns immediately;
    /// the result arrives via `poll_preview_result` on a future frame.
    fn load_preview(&mut self, path: &Path) {
        let path_buf = path.to_path_buf();
        self.selected_path = Some(path_buf.clone());
        self.preview_scroll = 0;

        // Cache hit: no I/O needed.
        if let Some(idx) = self.preview_cache.iter().position(|(p, _)| p == &path_buf) {
            self.preview_content = self.preview_cache[idx].1.clone();
            return;
        }

        // Cancel any in-flight load for a previous file.
        self.preview_result_rx = None;
        self.preview_content = PreviewContent::Loading;

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let is_markdown = ext == "md" || ext == "markdown";
        let preview_width = (self.terminal_size.0 as f32 * 0.7) as u16;

        // Ensure the shared highlighter exists before spawning so the Arc is ready.
        if !is_markdown {
            self.highlighter
                .get_or_insert_with(|| Arc::new(Mutex::new(Highlighter::new())));
        }
        let highlighter = self.highlighter.as_ref().map(Arc::clone);

        let (tx, rx) = mpsc::channel();
        self.preview_result_rx = Some(rx);

        std::thread::spawn(move || {
            let content = match fsx::read_file_text(&path_buf) {
                Ok(text) => {
                    if is_markdown {
                        PreviewContent::Markdown(render_markdown(&text, preview_width))
                    } else if let Some(h) = highlighter {
                        let lines = h.lock().unwrap().highlight_file(&text, &ext);
                        PreviewContent::Highlighted(lines)
                    } else {
                        PreviewContent::Error("Highlighter unavailable".to_string())
                    }
                }
                Err(e) => PreviewContent::Error(e.to_string()),
            };
            let _ = tx.send((path_buf, content));
        });
    }

    /// Insert `content` into the preview cache under `path`, evicting the oldest
    /// entry when the cache exceeds its capacity.
    fn cache_preview(&mut self, path: PathBuf, content: PreviewContent) {
        self.preview_cache.retain(|(p, _)| p != &path);
        self.preview_cache.push((path, content));
        const MAX_CACHE: usize = 20;
        if self.preview_cache.len() > MAX_CACHE {
            self.preview_cache.remove(0);
        }
    }
}
