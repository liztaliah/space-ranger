//! Thin wrappers around std::fs that return anyhow::Result.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

pub struct FsEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

/// Read a directory and return its entries sorted: directories first,
/// then files, each group sorted case-insensitively by name.
pub fn read_dir_sorted(path: &Path) -> Result<Vec<FsEntry>> {
    let mut entries: Vec<FsEntry> = fs::read_dir(path)?
        .filter_map(|e| e.ok()) // silently skip entries we can't read
        .map(|e| {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            let is_dir = path.is_dir();
            FsEntry { name, path, is_dir }
        })
        .collect();

    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}

/// Read a file's contents as a UTF-8 string.
/// Returns an error (shown in the preview panel) for files over 1MB to avoid
/// blocking the UI on huge logs or binary files.
pub fn read_file_text(path: &Path) -> Result<String> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > 1_000_000 {
        anyhow::bail!("File too large to preview (>1MB)");
    }
    // Use from_utf8_lossy so binary files display as text with replacement
    // characters rather than returning an error.
    let bytes = fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn delete_file(path: &Path) -> Result<()> {
    fs::remove_file(path)?;
    Ok(())
}
