# space-ranger

A keyboard-driven terminal file browser built in Rust with [ratatui](https://github.com/ratatui-org/ratatui). Navigate your filesystem, preview files with syntax highlighting, search, rename, and delete — all without leaving the terminal.

## Features

- **Directory tree** — expand/collapse directories in place, navigate with vim-style keys or arrows
- **File preview** — syntax highlighting for code files, rendered markdown, loaded in the background so the UI stays responsive
- **Fuzzy-ish search** — press `/` to search the current directory; results filter as you type
- **Rename** — in-place rename with smart cursor: extension is protected until you explicitly edit it
- **Delete** — file deletion with confirmation prompt
- **Terminal-native** — respects your terminal's background color and color scheme

## Installation

### Requirements

- [Rust](https://rustup.rs) 1.83.0 or later

### Build from source

```bash
git clone https://github.com/liztaliah/space-ranger
cd space-ranger
cargo build --release
```

The compiled binary will be at `target/release/fbrowse`. You can move it anywhere on your `$PATH`:

```bash
cp target/release/fbrowse /usr/local/bin/fbrowse
```

## Usage

```bash
# Open in the current directory
fbrowse

# Open in a specific directory
fbrowse ~/Projects
```

## Keybindings

### Tree (default)

| Key | Action |
|-----|--------|
| `j` / `↓` | Move cursor down |
| `k` / `↑` | Move cursor up |
| `l` / `→` / `Enter` | Enter directory / load file preview |
| `h` / `←` | Go to parent directory |
| `Tab` | Focus preview panel |
| `/` | Open search |
| `r` | Rename selected file |
| `d` | Delete selected file |
| `q` | Quit |

### Preview panel

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll one line |
| `Ctrl+d` / `Ctrl+u` | Page down / up |
| `g` / `G` | Jump to top / bottom |
| `Tab` / `h` / `Esc` | Return focus to tree |
| `q` | Quit |

### Search

| Key | Action |
|-----|--------|
| Any character | Filter results |
| `j` / `k` | Move cursor through results |
| `Backspace` | Delete last character |
| `Enter` | Navigate to selected file |
| `Esc` | Close search |

### Rename dialog

| Key | Action |
|-----|--------|
| Any character | Edit filename (first keypress replaces) |
| `→` | Move cursor into extension field |
| `←` | Move cursor back to stem |
| `Backspace` | Delete character (or exit extension if only `.` remains) |
| `Tab` | Toggle between Rename / Cancel buttons |
| `Enter` | Confirm |
| `Esc` | Cancel |

## Development

```bash
# Run in debug mode
cargo run

# Run tests
cargo test

# Lint
cargo clippy && cargo fmt
```
