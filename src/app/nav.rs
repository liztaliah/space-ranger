//! Directory navigation: cursor movement, enter/go-parent, parent column tracking.

use anyhow::Result;

use crate::fs as fsx;
use super::{AppState, DirEntry, Focus, PreviewContent};

impl AppState {
    /// Move the cursor within whichever column is currently focused.
    pub(super) fn move_cursor(&mut self, delta: i32) {
        match self.focused_col {
            0 => {
                let len = self.grandparent_entries.len();
                if len == 0 { return; }
                self.grandparent_cursor = (self.grandparent_cursor as i32 + delta)
                    .clamp(0, len as i32 - 1) as usize;
            }
            1 => {
                let len = self.parent_entries.len();
                if len == 0 { return; }
                self.parent_cursor = (self.parent_cursor as i32 + delta)
                    .clamp(0, len as i32 - 1) as usize;
            }
            _ => {
                let len = self.entries.len();
                if len == 0 { return; }
                self.cursor = (self.cursor as i32 + delta)
                    .clamp(0, len as i32 - 1) as usize;
                self.update_scroll();
            }
        }
    }

    /// Enter on a directory navigates into it; Enter on a file opens the preview pane.
    /// Uses the cursor of whichever column is currently focused.
    pub(super) fn enter_or_expand(&mut self) -> Result<()> {
        let (is_dir, path) = match self.focused_col {
            0 => {
                if self.grandparent_entries.is_empty() { return Ok(()); }
                let e = &self.grandparent_entries[self.grandparent_cursor];
                (e.is_dir, e.path.clone())
            }
            1 => {
                if self.parent_entries.is_empty() { return Ok(()); }
                let e = &self.parent_entries[self.parent_cursor];
                (e.is_dir, e.path.clone())
            }
            _ => {
                if self.entries.is_empty() { return Ok(()); }
                let e = &self.entries[self.cursor];
                (e.is_dir, e.path.clone())
            }
        };

        if is_dir {
            if self.focused_col == 2 {
                // Col 2: slide window right, reusing already-loaded columns.
                self.grandparent_entries = std::mem::take(&mut self.parent_entries);
                self.grandparent_cursor = self.parent_cursor;
                self.parent_entries = std::mem::take(&mut self.entries);
                self.parent_cursor = self.cursor;
                self.root = path.clone();
                self.entries = Self::read_entries(&path);
                self.cursor = 0;
                self.scroll_offset = 0;
            } else {
                // Col 0/1: navigate into the selected dir, but stay focused on the
                // same column — the ancestor columns refresh from disk and will
                // naturally show the same directories (since the new root is a child
                // of the focused column's directory). Only col 2 gains new content.
                self.root = path.clone();
                self.load_entries();
                self.cursor = 0;
                self.scroll_offset = 0;
                // focused_col intentionally NOT reset — stays where the user is.
            }
        } else {
            self.preview_open = true;
            self.preview_scroll = 0;
            self.load_preview(&path);
            self.focus = Focus::Preview;
            self.focused_col = 2;
        }
        Ok(())
    }

    pub(super) fn go_parent(&mut self) {
        if let Some(parent) = self.root.parent().map(|p| p.to_path_buf()) {
            let old_root = self.root.clone();
            self.root = parent.clone();

            // Slide the window left: parent → current, grandparent → parent.
            // Only the new grandparent requires a disk read.
            let new_cursor = self.parent_entries.iter()
                .position(|e| e.path == old_root || e.path.file_name() == old_root.file_name())
                .unwrap_or(self.parent_cursor);
            self.entries = std::mem::take(&mut self.parent_entries);
            self.cursor = new_cursor;
            self.scroll_offset = 0;
            self.update_scroll();

            self.parent_entries = std::mem::take(&mut self.grandparent_entries);
            self.parent_cursor = self.grandparent_cursor;

            if let Some(grandparent) = parent.parent() {
                self.grandparent_entries = Self::read_entries(grandparent);
                self.grandparent_cursor = self.grandparent_entries.iter()
                    .position(|e| e.path == parent || e.path.file_name() == parent.file_name())
                    .unwrap_or(0);
            } else {
                self.grandparent_entries = Vec::new();
                self.grandparent_cursor = 0;
            }

            self.selected_path = None;
            self.preview_content = PreviewContent::Empty;
            self.preview_open = false;
            self.focus = Focus::Tree;
        }
    }

    /// (Re)load the entries for the current root and refresh the ancestor columns.
    pub(super) fn load_entries(&mut self) {
        self.entries = Self::read_entries(&self.root.clone());
        self.load_parent_entries();
    }

    /// Read the parent and grandparent directories, tracking which entries
    /// correspond to the current root and its parent respectively.
    pub(super) fn load_parent_entries(&mut self) {
        if let Some(parent) = self.root.parent() {
            self.parent_entries = Self::read_entries(parent);
            self.parent_cursor = self
                .parent_entries
                .iter()
                .position(|e| e.path == self.root || e.path.file_name() == self.root.file_name())
                .unwrap_or(0);

            if let Some(grandparent) = parent.parent() {
                self.grandparent_entries = Self::read_entries(grandparent);
                self.grandparent_cursor = self
                    .grandparent_entries
                    .iter()
                    .position(|e| e.path == parent || e.path.file_name() == parent.file_name())
                    .unwrap_or(0);
            } else {
                self.grandparent_entries = Vec::new();
                self.grandparent_cursor = 0;
            }
        } else {
            self.parent_entries = Vec::new();
            self.parent_cursor = 0;
            self.grandparent_entries = Vec::new();
            self.grandparent_cursor = 0;
        }
    }

    fn read_entries(path: &std::path::Path) -> Vec<DirEntry> {
        match fsx::read_dir_sorted(path) {
            Ok(children) => children
                .into_iter()
                .map(|e| DirEntry { name: e.name, path: e.path, is_dir: e.is_dir, depth: 0 })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

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
