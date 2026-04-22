//! Directory tree navigation: cursor movement, expand/collapse, parent traversal.

use anyhow::Result;
use std::time::Instant;

use crate::fs as fsx;
use super::{AppState, DirEntry, Focus, PreviewContent};

impl AppState {
    pub(super) fn move_cursor(&mut self, delta: i32) {
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
            // Clear immediately so the old file's content doesn't stay visible
            // during the debounce window while the user is still scrolling.
            self.selected_path = None;
            self.preview_content = PreviewContent::Empty;
        } else {
            self.preview_pending_path = None;
            self.preview_pending_since = None;
            self.selected_path = None;
            self.preview_content = PreviewContent::Empty;
        }
    }

    pub(super) fn enter_or_expand(&mut self) -> Result<()> {
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

    pub(super) fn go_parent(&mut self) {
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
    pub(super) fn load_entries(&mut self) {
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

    /// Keep scroll_offset so the cursor row is always visible.
    pub(super) fn update_scroll(&mut self) {
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
}
