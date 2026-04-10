use std::io::{stdout, Stdout};

use anyhow::Result;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use ratatui::prelude::*;

/// Restores terminal state on drop (raw mode off, leave alt screen, show cursor).
pub struct TerminalRestore;

impl Drop for TerminalRestore {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        let _ = stdout().execute(crossterm::cursor::Show);
    }
}

/// Raw mode + alternate screen + hide hardware cursor (TUI draws its own caret).
pub fn enter_tui_screen() -> Result<Stdout> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, crossterm::cursor::Hide)?;
    Ok(out)
}

pub fn new_terminal(out: Stdout) -> Result<Terminal<CrosstermBackend<Stdout>>> {
    Ok(Terminal::new(CrosstermBackend::new(out))?)
}
