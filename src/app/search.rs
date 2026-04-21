//! Search filtering: applies the current query against the in-memory cache.

use super::{AppState, DirEntry};

impl AppState {
    /// Filter search_cache by the current query string (case-insensitive substring match).
    pub(super) fn apply_search_filter(&mut self) {
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
}
