//! On-disk idoit-only history (`history.json`) for `last` and `refine`.
//! Migrated from legacy `history.jsonl` on first load.

use std::fs::{self, File};
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::SessionEntry;

const MAX_ENTRIES: usize = 2000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct HistoryFile {
    entries: Vec<SessionEntry>,
}

fn idoit_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("idoit")
}

pub fn history_json_path() -> PathBuf {
    idoit_data_dir().join("history.json")
}

fn history_jsonl_path() -> PathBuf {
    idoit_data_dir().join("history.jsonl")
}

fn atomic_write(path: &std::path::Path, data: &HistoryFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create {}", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
        serde_json::to_writer(&mut f, data).with_context(|| "serialize history.json")?;
        f.sync_all().ok();
    }
    fs::rename(&tmp, path).with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

fn load_history_file() -> HistoryFile {
    let json_path = history_json_path();
    if json_path.exists() {
        if let Ok(raw) = fs::read_to_string(&json_path) {
            if let Ok(h) = serde_json::from_str::<HistoryFile>(&raw) {
                return h;
            }
        }
    }

    let jsonl_path = history_jsonl_path();
    if jsonl_path.exists() {
        let mut entries = Vec::new();
        if let Ok(content) = fs::read_to_string(&jsonl_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(e) = serde_json::from_str::<SessionEntry>(line) {
                    entries.push(e);
                }
            }
        }
        if entries.len() > MAX_ENTRIES {
            entries = entries.split_off(entries.len() - MAX_ENTRIES);
        }
        let hf = HistoryFile { entries };
        if atomic_write(&json_path, &hf).is_ok() {
            let _ = fs::remove_file(&jsonl_path);
        }
        return hf;
    }

    HistoryFile::default()
}

pub fn record(input: &str, command: &str, executed: bool, exit_code: Option<i32>) {
    let entry = SessionEntry {
        ts: chrono::Utc::now().to_rfc3339(),
        input: input.to_string(),
        command: command.to_string(),
        executed,
        exit_code,
    };
    let mut hf = load_history_file();
    hf.entries.push(entry);
    if hf.entries.len() > MAX_ENTRIES {
        hf.entries.drain(0..hf.entries.len() - MAX_ENTRIES);
    }
    let _ = atomic_write(&history_json_path(), &hf);
}

pub fn last_entry() -> Option<SessionEntry> {
    load_history_file().entries.last().cloned()
}

pub fn last_command_string() -> Option<String> {
    last_entry().map(|e| e.command)
}
