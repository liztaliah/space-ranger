# CLAUDE.md

## Project

A keyboard-driven terminal file browser built in Rust with [ratatui](https://github.com/ratatui-org/ratatui). Port of a Python/Textual app.

## Setup

```bash
cargo build
cargo build --release   # optimized + stripped binary (~2MB)
```

## Commands

```bash
# Run (defaults to current directory)
cargo run
./target/debug/fbrowse [/optional/path]

# Tests
cargo test

# Lint
cargo clippy
cargo fmt
```

## Architecture

### Module layout

```
src/
├── main.rs        # Terminal init/teardown, panic hook, event loop
├── app.rs         # AppState struct + all state transitions (apply())
├── fs.rs          # Filesystem helpers: read_dir_sorted, read_file_text, delete_file
├── highlight.rs   # syntect → ratatui Span conversion; lazy-init Highlighter
├── markdown.rs    # termimad → ratatui Line conversion
├── input.rs       # map_key(KeyEvent, &AppMode, &Focus) → AppAction (pure, unit-tested)
└── ui/
    ├── mod.rs     # Top-level render() + layout
    ├── theme.rs   # Color constants (matches original browser.tcss palette)
    ├── tree.rs    # Left panel: directory tree as ratatui List
    ├── preview.rs # Right panel: syntax-highlighted or markdown file preview
    ├── search.rs  # Bottom search bar
    ├── modal.rs   # Delete confirmation overlay
    └── hints.rs   # Key binding hints bar
```

### Key types (app.rs)

- `AppState` — single source of truth; mutated only via `apply(AppAction)`
- `AppMode` — `Browse | Search | DeleteConfirm`
- `Focus` — `Tree | Preview` (which panel receives keyboard input)
- `PreviewContent` — `Empty | Error(String) | Highlighted(Vec<Line>) | Markdown(Vec<Line>)`

### Event loop pattern

Immediate-mode: every frame redraws from `AppState`. No retained widget tree.

```
loop {
    state.poll_search_cache()   // non-blocking: receive background walkdir result
    terminal.draw(render)
    poll(50ms) → key event → map_key → AppAction → state.apply()
}
```

### Directory tree

Stored as a flat `Vec<DirEntry>` with `depth` for indentation. Expanding a dir splices children in-place; collapsing drains them. No tree widget — rendered as a `List`.

### Search

Pressing `/` spawns a background thread that walks the directory tree with `walkdir`. The UI shows "Scanning…" until the thread sends results back via `mpsc::channel`. Subsequent keystrokes filter the in-memory cache — no further I/O.

### Syntax highlighting

`syntect` is lazy-initialized on first file preview (not at startup) to keep startup instant. Pre-renders to `Vec<Line<'static>>` (owned Spans) so rendering is zero-cost.

## Keybindings

**Tree focused (default):**
| Key | Action |
|-----|--------|
| `j`/`k` or `↑`/`↓` | Move cursor |
| `l`/`→`/`Enter` | Enter directory / load preview |
| `h`/`←` | Go to parent directory |
| `Tab` | Focus preview panel (if file loaded) |
| `/` | Open search |
| `d` | Delete selected file (with confirmation) |
| `q` | Quit |

**Preview focused:**
| Key | Action |
|-----|--------|
| `j`/`k` | Scroll one line |
| `ctrl+d`/`ctrl+u` | Half-page down/up |
| `g`/`G` | Jump to top/bottom |
| `Tab`, `h`, or `Esc` | Return focus to tree |
| `q` | Quit |

**Search mode:**
| Key | Action |
|-----|--------|
| Any character | Filter results |
| `Backspace` | Delete last character |
| `Enter`/`Esc` | Close search, return to tree |

## Dependency notes

Rust 1.83.0 compatibility requires pinned versions in `Cargo.lock`:
- `instability` pinned to `0.3.6` (ratatui 0.29+ pulls in 0.3.12 which needs rustc 1.88)
- `unicode-segmentation` pinned to `1.12.0`
- `syntect` uses `regex-fancy` feature (pure Rust regex, no C bindings)

Do not run `cargo update` without checking these pins still hold.
