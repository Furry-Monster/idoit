//! Host shell integration: detected context, history file parsing, command execution, `init` scripts, rc snippets, safety split.

pub mod command_safety;
pub mod context;
pub mod executor;
pub mod history;
pub mod init;
pub mod rc;
