use std::path::PathBuf;

use anyhow::{Context, Result};

use super::context::ShellContext;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub command: String,
}

pub fn last_command(ctx: &ShellContext, history_override: Option<&str>) -> Result<HistoryEntry> {
    // Set in precmd / PROMPT_COMMAND / fish postexec: the command that *finished* last.
    // (__IDOIT_LAST_CMD is the line about to run, so a child idoit would see "idoit ..." otherwise.)
    if let Ok(cmd) = std::env::var("__IDOIT_COMPLETED_CMD") {
        if !cmd.is_empty() {
            return Ok(HistoryEntry { command: cmd });
        }
    }
    // Legacy hook name (wrong for subprocesses; keep as fallback)
    if let Ok(cmd) = std::env::var("__IDOIT_LAST_CMD") {
        if !cmd.is_empty() {
            return Ok(HistoryEntry { command: cmd });
        }
    }

    let path = history_file_path(ctx, history_override)?;

    if !path.exists() {
        anyhow::bail!(
            "shell history file not found at {}.\n\
             Tip: run `eval \"$(idoit init {})\"` in your shell config for better --fix support.",
            path.display(),
            ctx.shell,
        );
    }

    let raw = std::fs::read(&path)
        .with_context(|| format!("failed to read shell history at {}", path.display()))?;
    let content = String::from_utf8_lossy(&raw);

    let shell = ctx.shell.as_str();
    let entry = match shell {
        "fish" => parse_fish_history(&content),
        "zsh" => parse_zsh_history(&content),
        _ => parse_bash_history(&content),
    };

    entry.ok_or_else(|| anyhow::anyhow!("shell history is empty"))
}

pub fn recent_error_output() -> Option<String> {
    // Read stderr captured by shell hook
    let stderr_file = std::env::var("__IDOIT_LAST_STDERR").ok()?;
    let content = std::fs::read_to_string(&stderr_file).ok()?;
    if content.is_empty() {
        None
    } else {
        // Truncate very long output to keep prompt reasonable
        Some(if content.len() > 2000 {
            format!("{}...(truncated)", &content[..2000])
        } else {
            content
        })
    }
}

pub fn last_exit_code() -> Option<i32> {
    std::env::var("__IDOIT_LAST_EXIT")
        .ok()
        .and_then(|s| s.parse().ok())
}

pub fn default_history_path(shell: &str) -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;

    match shell {
        "zsh" => Ok(home.join(".zsh_history")),
        "fish" => {
            let fish_dir = std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".local/share"));
            Ok(fish_dir.join("fish/fish_history"))
        }
        _ => Ok(home.join(".bash_history")),
    }
}

/// Recent commands from the on-disk shell history file, oldest-first within the window.
pub fn recent_shell_command_lines(
    ctx: &ShellContext,
    history_override: Option<&str>,
    limit: usize,
) -> Result<Vec<String>> {
    let path = history_file_path(ctx, history_override)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read(&path)
        .with_context(|| format!("failed to read shell history at {}", path.display()))?;
    let content = String::from_utf8_lossy(&raw);
    let shell = ctx.shell.as_str();
    let mut cmds = match shell {
        "fish" => list_fish_commands_chrono(&content),
        "zsh" => list_zsh_commands_chrono(&content),
        _ => list_bash_commands_chrono(&content),
    };
    if cmds.len() > limit {
        cmds = cmds.split_off(cmds.len() - limit);
    }
    Ok(cmds)
}

fn list_bash_commands_chrono(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|s| s.to_string())
        .collect()
}

fn list_fish_commands_chrono(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in content.lines() {
        if let Some(cmd) = line.strip_prefix("- cmd: ") {
            let c = cmd.trim();
            if !c.is_empty() {
                out.push(c.to_string());
            }
        }
    }
    out
}

fn list_zsh_commands_chrono(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Vec<String> = Vec::new();

    let flush = |current: &mut Vec<String>, out: &mut Vec<String>| {
        if current.is_empty() {
            return;
        }
        let full = current.join("\n");
        let cmd = if full.starts_with(": ") {
            full.splitn(2, ';')
                .nth(1)
                .unwrap_or(&full)
                .trim()
                .to_string()
        } else {
            full.trim().to_string()
        };
        if !cmd.is_empty() {
            out.push(cmd);
        }
        current.clear();
    };

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let is_header =
            line.starts_with(": ") || (!line.starts_with(' ') && !line.starts_with('\t'));
        if is_header {
            flush(&mut current, &mut out);
            current.push(line.to_string());
        } else {
            current.push(line.to_string());
        }
    }
    flush(&mut current, &mut out);
    out
}

pub fn history_file_path(ctx: &ShellContext, history_override: Option<&str>) -> Result<PathBuf> {
    if let Some(path) = history_override {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    if let Ok(histfile) = std::env::var("HISTFILE") {
        return Ok(PathBuf::from(histfile));
    }

    default_history_path(&ctx.shell)
}

fn parse_bash_history(content: &str) -> Option<HistoryEntry> {
    content
        .lines()
        .rev()
        .find(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .map(|cmd| HistoryEntry {
            command: cmd.trim().to_string(),
        })
}

fn parse_zsh_history(content: &str) -> Option<HistoryEntry> {
    let mut entries: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();

    for line in content.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        let is_start =
            line.starts_with(": ") || (!line.starts_with(' ') && !line.starts_with('\t'));
        if is_start {
            current.push(line);
            current.reverse();
            let full = current.join("\n");
            let cmd = if full.starts_with(": ") {
                full.splitn(2, ';').nth(1).unwrap_or(&full)
            } else {
                &full
            };
            let cmd = cmd.trim().to_string();
            if !cmd.is_empty() {
                entries.push(cmd);
            }
            current.clear();
            if !entries.is_empty() {
                break;
            }
        } else {
            current.push(line);
        }
    }

    entries
        .into_iter()
        .next()
        .map(|command| HistoryEntry { command })
}

fn parse_fish_history(content: &str) -> Option<HistoryEntry> {
    let mut last_cmd = None;
    for line in content.lines() {
        if let Some(cmd) = line.strip_prefix("- cmd: ") {
            last_cmd = Some(cmd.trim().to_string());
        }
    }
    last_cmd.map(|command| HistoryEntry { command })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_history() {
        let content = "ls -la\ncd /tmp\ngit status\n";
        let entry = parse_bash_history(content).unwrap();
        assert_eq!(entry.command, "git status");
    }

    #[test]
    fn test_bash_history_with_comments() {
        let content = "ls\n#1234567890\ngrep foo bar\n";
        let entry = parse_bash_history(content).unwrap();
        assert_eq!(entry.command, "grep foo bar");
    }

    #[test]
    fn test_zsh_extended_history() {
        let content = ": 1234567890:0;ls -la\n: 1234567891:0;git diff\n";
        let entry = parse_zsh_history(content).unwrap();
        assert_eq!(entry.command, "git diff");
    }

    #[test]
    fn test_zsh_plain_history() {
        let content = "ls -la\ngit diff\n";
        let entry = parse_zsh_history(content).unwrap();
        assert_eq!(entry.command, "git diff");
    }

    #[test]
    fn test_fish_history() {
        let content = "- cmd: ls -la\n  when: 1234567890\n- cmd: git status\n  when: 1234567891\n";
        let entry = parse_fish_history(content).unwrap();
        assert_eq!(entry.command, "git status");
    }

    #[test]
    fn test_empty_history() {
        assert!(parse_bash_history("").is_none());
        assert!(parse_zsh_history("").is_none());
        assert!(parse_fish_history("").is_none());
    }

    #[test]
    fn test_list_zsh_chrono_order() {
        let content = ": 1:0;ls\n: 2:0;git status\n";
        let v = list_zsh_commands_chrono(content);
        assert_eq!(v, vec!["ls", "git status"]);
    }
}
