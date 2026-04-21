//! Entry point: terminal setup/teardown, panic hook, and the main event loop.
//!
//! Uses ratatui's immediate-mode model: every iteration redraws the full UI
//! from AppState, then blocks up to 50ms waiting for a key event.

mod app;
mod fs;
mod highlight;
mod input;
mod markdown;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::AppState;

fn main() -> Result<()> {
    let root = match std::env::args().nth(1) {
        Some(p) => {
            let path = PathBuf::from(p);
            if !path.is_dir() {
                eprintln!("Error: '{}' is not a directory", path.display());
                std::process::exit(1);
            }
            path
        }
        None => std::env::current_dir()?,
    };

    // Restore the terminal if the app panics — without this, a panic in raw mode
    // leaves the terminal in an unusable state (no echo, alternate screen stuck).
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stderr(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
        original_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, root);

    // Always restore the terminal, even if run() returned an error.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, root: PathBuf) -> Result<()> {
    let mut state = AppState::new(root)?;

    loop {
        state.poll_preview(Duration::from_millis(150));
        state.poll_preview_result();
        // Check whether the background search-cache thread has finished.
        // This is non-blocking (try_recv) so it never stalls the render loop.
        state.poll_search_cache();

        terminal.draw(|f| ui::render(f, &state))?;

        // Block for up to 50ms; if no event arrives we redraw anyway (handles
        // things like the search loading indicator resolving between keystrokes).
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    let action = input::map_key(key, &state.mode, &state.focus);
                    state.apply(action)?;
                    if state.should_quit {
                        break;
                    }
                }
                Event::Resize(w, h) => {
                    state.terminal_size = (w, h);
                }
                _ => {}
            }
        }
    }

    Ok(())
}
