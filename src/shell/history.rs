use std::path::PathBuf;

use anyhow::{Context, Result};

use super::context::ShellContext;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub command: String,
}

pub fn last_command(ctx: &ShellContext) -> Result<HistoryEntry> {
    let path = history_file_path(ctx)?;

    if !path.exists() {
        anyhow::bail!(
            "shell history file not found at {}. Make sure your shell is configured to save history.",
            path.display()
        );
    }

    // Read as bytes first — zsh can use metafied (binary) encoding
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

fn history_file_path(ctx: &ShellContext) -> Result<PathBuf> {
    if let Ok(histfile) = std::env::var("HISTFILE") {
        return Ok(PathBuf::from(histfile));
    }

    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;

    match ctx.shell.as_str() {
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
    // Handle multi-line commands (continuation lines start with whitespace after
    // a line ending with backslash). We walk backwards through logical entries.
    let mut entries: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();

    for line in content.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }

        // A new history entry starts with `: ` (extended format) or a non-whitespace char
        let is_start = line.starts_with(": ") || (!line.starts_with(' ') && !line.starts_with('\t'));

        if is_start {
            current.push(line);
            current.reverse();
            let full = current.join("\n");
            // Strip the extended history prefix `: timestamp:duration;`
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

    entries.into_iter().next().map(|command| HistoryEntry { command })
}

fn parse_fish_history(content: &str) -> Option<HistoryEntry> {
    // fish history format:
    // - cmd: the_command
    //   when: timestamp
    //   paths:
    //     - /some/path
    let mut last_cmd = None;
    for line in content.lines() {
        if let Some(cmd) = line.strip_prefix("- cmd: ") {
            last_cmd = Some(cmd.trim().to_string());
        }
    }
    last_cmd.map(|command| HistoryEntry { command })
}

#[allow(dead_code)]
pub fn recent_error_output() -> Option<String> {
    // Shell history doesn't store stderr. In the future we could integrate
    // with shell preexec/precmd hooks to capture exit codes and stderr.
    None
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
}
