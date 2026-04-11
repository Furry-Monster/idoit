use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::ai::types::AiCommandResponse;
use crate::session::SessionEntry;

use super::completion::{ghost_suffix, shell_candidates, split_last_token};

pub struct App {
    pub input: String,
    pub learn_mode: bool,
    pub dry_run: bool,
    /// Generation counter; incremented on each input change.
    pub ai_gen: u64,
    /// Latest `ai_gen` — coordinator compares after debounce.
    pub ai_live: Arc<AtomicU64>,
    pub shell_cands: Vec<String>,
    pub shell_idx: usize,
    pub trans_cmds: Vec<String>,
    pub trans_idx: usize,
    pub trans_expl: String,
    pub diagnostic: String,
    pub trans_pending: bool,
    pub diag_pending: bool,
    pub status_line: String,
    pub run_output: String,
    pub diag_scroll: u16,
    /// This TUI run only: suggestions / executes (layer 3 for `LayeredContext`).
    pub idoit_run: Vec<SessionEntry>,
}

impl App {
    pub fn new(learn_mode: bool, dry_run: bool, ai_live: Arc<AtomicU64>) -> Self {
        Self {
            input: String::new(),
            learn_mode,
            dry_run,
            ai_gen: 0,
            ai_live,
            shell_cands: Vec::new(),
            shell_idx: 0,
            trans_cmds: Vec::new(),
            trans_idx: 0,
            trans_expl: String::new(),
            diagnostic: String::new(),
            trans_pending: false,
            diag_pending: false,
            status_line: String::new(),
            run_output: String::new(),
            diag_scroll: 0,
            idoit_run: Vec::new(),
        }
    }

    pub fn bump_ai_gen(&mut self) -> u64 {
        self.ai_gen = self.ai_gen.wrapping_add(1);
        self.ai_live.store(self.ai_gen, Ordering::SeqCst);
        self.ai_gen
    }

    pub fn refresh_shell(&mut self) {
        self.shell_cands = shell_candidates(&self.input);
        if self.shell_cands.is_empty() {
            self.shell_idx = 0;
        } else {
            self.shell_idx %= self.shell_cands.len();
        }
    }

    pub fn shell_ghost(&self) -> Option<String> {
        if self.shell_cands.is_empty() {
            return None;
        }
        let (_, token) = split_last_token(&self.input);
        let cand = &self.shell_cands[self.shell_idx];
        ghost_suffix(&token, cand)
    }

    pub fn effective_translation(&self) -> Option<&str> {
        self.trans_cmds.get(self.trans_idx).map(|s| s.as_str())
    }

    pub fn apply_tab(&mut self) {
        if let Some(_suf) = self.shell_ghost() {
            let (prefix, token) = split_last_token(&self.input);
            let cand = &self.shell_cands[self.shell_idx];
            if token.is_empty() || cand.starts_with(&token) {
                self.input = format!("{prefix}{cand}");
            }
            self.refresh_shell();
            return;
        }
        if let Some(cmd) = self.effective_translation().map(|s| s.to_string()) {
            self.input = cmd;
            self.refresh_shell();
        }
    }

    pub fn cycle_up(&mut self) {
        if self.shell_cands.len() > 1 {
            self.shell_idx = self.shell_idx.saturating_sub(1);
            return;
        }
        if self.trans_cmds.len() > 1 {
            self.trans_idx = (self.trans_idx + self.trans_cmds.len() - 1) % self.trans_cmds.len();
        }
    }

    pub fn cycle_down(&mut self) {
        if self.shell_cands.len() > 1 {
            self.shell_idx = (self.shell_idx + 1) % self.shell_cands.len();
            return;
        }
        if self.trans_cmds.len() > 1 {
            self.trans_idx = (self.trans_idx + 1) % self.trans_cmds.len();
        }
    }

    pub fn apply_ai_translate(&mut self, gen: u64, resp: AiCommandResponse) {
        if gen != self.ai_gen {
            return;
        }
        self.trans_pending = false;
        if resp.command.trim().is_empty() {
            self.trans_cmds.clear();
            self.trans_expl.clear();
            return;
        }
        let mut cmds = vec![resp.command.clone()];
        for a in resp.alternates {
            let t = a.trim();
            if !t.is_empty() && !cmds.iter().any(|c| c == t) {
                cmds.push(t.to_string());
            }
        }
        self.trans_cmds = cmds;
        self.trans_idx = 0;
        self.trans_expl = resp.explanation;
    }

    pub fn apply_ai_err(&mut self, gen: u64, err: String) {
        if gen != self.ai_gen {
            return;
        }
        self.trans_pending = false;
        self.trans_cmds.clear();
        self.trans_expl = err;
    }

    pub fn apply_diag(&mut self, gen: u64, text: String, done: bool) {
        if gen != self.ai_gen {
            return;
        }
        self.diagnostic = text;
        self.diag_pending = !done;
    }

    pub fn apply_diag_err(&mut self, gen: u64, err: String) {
        if gen != self.ai_gen {
            return;
        }
        self.diag_pending = false;
        self.diagnostic = format!("error: could not fetch diagnostic\nnote: {err}");
    }
}
