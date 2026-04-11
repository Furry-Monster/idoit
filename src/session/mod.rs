//! Session data for the model and `idoit last`: layered context, on-disk `history.json`, terminal log tail.
//!
//! `persisted` and `terminal_log` are private; the supported API is `SessionEntry`, `context`, and the `record` / `last_*` / `push_run_buffer` re-exports below.

use serde::{Deserialize, Serialize};

/// One idoit interaction stored in `history.json` (and optionally in the in-memory run buffer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub ts: String,
    pub input: String,
    pub command: String,
    pub executed: bool,
    pub exit_code: Option<i32>,
}

pub mod context;
mod persisted;
mod terminal_log;

pub use context::push_run_buffer;
pub use persisted::{last_command_string, last_entry, record};
