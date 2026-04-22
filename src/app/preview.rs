//! File preview loading: background thread dispatch, result polling, LRU cache.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

use crate::fs as fsx;
use crate::highlight::Highlighter;
use crate::markdown::render_markdown;
use super::{AppState, PreviewContent};

impl AppState {
    /// Half the visible preview height — used for ctrl+d / ctrl+u page scrolling.
    pub(super) fn preview_page_size(&self) -> usize {
        let h = self.terminal_size.1 as usize;
        let inner = h.saturating_sub(4); // subtract borders + hint bar
        (inner / 2).max(1)
    }

    pub(super) fn preview_line_count(&self) -> usize {
        match &self.preview_content {
            PreviewContent::Highlighted(lines) | PreviewContent::Markdown(lines) => lines.len(),
            _ => 0,
        }
    }

    /// Kick off a background thread to read and render `path`. Returns immediately;
    /// the result arrives via `poll_preview_result` on a future frame.
    ///
    /// `pub(in crate::app)` rather than `pub(super)` because it is called from
    /// both `nav.rs` and `mod.rs`, which are siblings, not parent/child.
    pub(in crate::app) fn load_preview(&mut self, path: &Path) {
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
    pub(super) fn cache_preview(&mut self, path: PathBuf, content: PreviewContent) {
        self.preview_cache.retain(|(p, _)| p != &path);
        self.preview_cache.push((path, content));
        const MAX_CACHE: usize = 20;
        if self.preview_cache.len() > MAX_CACHE {
            self.preview_cache.remove(0);
        }
    }
}
