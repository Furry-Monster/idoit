//! Lines appended by shell integration (`idoit init`): recent non-idoit commands
//! in this terminal. Format per line: `ISO8601<TAB>command` (single-line commands).

use std::path::PathBuf;

const MAX_TAIL_LINES: usize = 500;

pub fn terminal_context_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("idoit")
        .join("terminal_context.jsonl")
}

/// Returns commands in chronological order (oldest of the window first).
pub fn read_terminal_session_commands(limit: usize) -> Vec<String> {
    let path = terminal_context_path();
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };

    let mut cmds: Vec<String> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((_, cmd)) = line.split_once('\t') {
            let c = cmd.trim();
            if !c.is_empty() {
                cmds.push(c.to_string());
            }
        }
    }

    if cmds.len() > limit {
        cmds = cmds.split_off(cmds.len() - limit);
    }
    cmds
}

/// Truncate log if it grows too large (best-effort).
pub fn trim_log_file() {
    let path = terminal_context_path();
    let Ok(content) = std::fs::read_to_string(&path) else {
        return;
    };
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= MAX_TAIL_LINES {
        return;
    }
    let keep: Vec<&str> = lines[lines.len() - MAX_TAIL_LINES..].to_vec();
    let new = keep.join("\n");
    let _ = std::fs::write(&path, format!("{new}\n"));
}
