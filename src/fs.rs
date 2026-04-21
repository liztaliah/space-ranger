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

    let bytes = fs::read(path)?;

    // Avoid rendering binary files in the preview pane. Lossy UTF-8 decoding
    // can retain control characters that corrupt terminal output.
    if bytes.contains(&0) {
        anyhow::bail!("Binary file cannot be previewed");
    }

    // Decode as UTF-8 (lossy for invalid sequences), then sanitize remaining
    // control characters so only printable text plus common whitespace is shown.
    let text = String::from_utf8_lossy(&bytes);
    let safe: String = text
        .chars()
        .map(|c| match c {
            '\n' | '\r' | '\t' => c,
            _ if c.is_control() => ' ',
            _ => c,
        })
        .collect();

    Ok(safe)
}

pub fn delete_file(path: &Path) -> Result<()> {
    fs::remove_file(path)?;
    Ok(())
}
