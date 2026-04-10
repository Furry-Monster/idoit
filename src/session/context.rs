//! In-memory layered context for prompts: shell history file → terminal session log → idoit run buffer.

use crate::config::settings::Settings;
use crate::shell::context::ShellContext;
use crate::shell::history;

use super::terminal_log;
use super::SessionEntry;

/// How many lines to take from each layer (window ends at most recent).
pub const SHELL_HISTORY_SNIPPET: usize = 50;
pub const TERMINAL_SESSION_SNIPPET: usize = 40;

const MAX_IDOIT_RUN_BUFFER: usize = 64;

#[derive(Debug, Clone, Default)]
pub struct LayeredContext {
    /// From `~/.bash_history` / `HISTFILE` (oldest → newest within window).
    pub shell_history_file: Vec<String>,
    /// From `terminal_context.jsonl` (shell hooks; non-idoit commands).
    pub terminal_session: Vec<String>,
    /// Current idoit run (e.g. TUI); empty on plain CLI translate.
    pub idoit_this_run: Vec<SessionEntry>,
}

impl LayeredContext {
    pub fn gather(
        ctx: &ShellContext,
        settings: &Settings,
        idoit_this_run: Option<&[SessionEntry]>,
    ) -> Self {
        terminal_log::trim_log_file();

        let hist_override = {
            let p = settings.behavior.history_path.trim();
            if p.is_empty() {
                None
            } else {
                Some(p)
            }
        };

        let shell_history_file =
            history::recent_shell_command_lines(ctx, hist_override, SHELL_HISTORY_SNIPPET)
                .unwrap_or_default();

        let terminal_session =
            terminal_log::read_terminal_session_commands(TERMINAL_SESSION_SNIPPET);

        let idoit_this_run = idoit_this_run
            .map(<[SessionEntry]>::to_vec)
            .unwrap_or_default();

        Self {
            shell_history_file,
            terminal_session,
            idoit_this_run,
        }
    }

    /// Text block for model prompts (oldest context first).
    pub fn format_block(&self) -> String {
        let mut s = String::new();

        if !self.shell_history_file.is_empty() {
            s.push_str("### Shell history file (recent, oldest first in this list)\n");
            for c in &self.shell_history_file {
                s.push_str("- ");
                s.push_str(c);
                s.push('\n');
            }
            s.push('\n');
        }

        if !self.terminal_session.is_empty() {
            s.push_str("### This terminal (recent non-idoit commands, from shell hooks)\n");
            for c in &self.terminal_session {
                s.push_str("- ");
                s.push_str(c);
                s.push('\n');
            }
            s.push('\n');
        }

        if !self.idoit_this_run.is_empty() {
            s.push_str("### This idoit session (current run)\n");
            for e in &self.idoit_this_run {
                s.push_str("- input: ");
                s.push_str(&e.input);
                s.push_str(" → command: ");
                s.push_str(&e.command);
                if e.executed {
                    s.push_str(" (executed");
                    if let Some(c) = e.exit_code {
                        s.push_str(&format!(", exit {c}"));
                    }
                    s.push(')');
                }
                s.push('\n');
            }
        }

        s
    }
}

pub fn push_run_buffer(buf: &mut Vec<SessionEntry>, entry: SessionEntry) {
    buf.push(entry);
    if buf.len() > MAX_IDOIT_RUN_BUFFER {
        buf.drain(0..buf.len() - MAX_IDOIT_RUN_BUFFER);
    }
}
