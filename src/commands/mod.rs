//! One module per CLI subcommand (plus `dispatch` for routing, first-run, and global options).
//! Handlers delegate to `ai`, `config`, `shell`, `session`, `macros`, `tui`, and `cli`.

pub mod config_cmd;
pub mod dispatch;
pub mod explain;
pub mod fix;
pub mod init;
pub mod last;
pub mod macro_cmd;
pub mod prompt_cmd;
pub mod refine;
pub mod run;
pub mod setup;
pub mod translate;
pub mod tui_cmd;
