use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub ts: String,
    pub input: String,
    pub command: String,
    pub executed: bool,
    pub exit_code: Option<i32>,
}

fn history_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("idoit")
        .join("history.jsonl")
}

pub fn record(input: &str, command: &str, executed: bool, exit_code: Option<i32>) {
    let entry = SessionEntry {
        ts: chrono::Utc::now().to_rfc3339(),
        input: input.to_string(),
        command: command.to_string(),
        executed,
        exit_code,
    };

    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(line) = serde_json::to_string(&entry) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

pub fn last_entry() -> Option<SessionEntry> {
    let path = history_path();
    let content = std::fs::read_to_string(&path).ok()?;
    content
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .and_then(|line| serde_json::from_str(line).ok())
}

pub fn last_command_string() -> Option<String> {
    last_entry().map(|e| e.command)
}
