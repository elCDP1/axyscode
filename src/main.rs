mod app;
mod editor;
mod input;
mod tree;
mod ui;

use anyhow::Result;
use crossterm::{
    cursor::Show,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

/// Guarantees the terminal returns to its normal state no matter what: if
/// something fails in between (mounting the app, reading the root directory,
/// a panic...), this guard's `Drop` still runs and keeps the user's terminal
/// from being left broken (no echo, alternate screen) after an unexpected
/// exit. Previously, a `?` that cut out of `main` before the manual cleanup
/// at the end left the terminal in that state.
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        // If this second step fails, the guard never gets constructed and
        // its Drop wouldn't run — so if anything goes wrong here, we undo
        // the raw mode by hand before propagating the error.
        if let Err(e) = execute!(io::stdout(), EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(e.into());
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // In a Drop you can't propagate errors, and we're closing anyway at
        // this point: the most reasonable thing is to try and continue; there
        // is no better "error handling" possible.
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    }
}

fn main() -> Result<()> {
    let _guard = TerminalGuard::new()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let root = std::env::current_dir()?;
    let mut app = app::App::new(root)?;

    // `_guard` is destroyed when leaving this function by any path
    // (success, error via `?`, or panic), restoring the terminal always.
    app.run(&mut terminal)
}
