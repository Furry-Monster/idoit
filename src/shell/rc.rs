//! Append or update a marked idoit block in the user shell rc file.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const BLOCK_START: &str = "# >>> idoit shell integration (managed by idoit setup) >>>";
const BLOCK_END: &str = "# <<< idoit shell integration <<<";

fn rc_path_for_home(home: &Path, shell: &str) -> Option<PathBuf> {
    Some(match shell {
        "bash" => home.join(".bashrc"),
        "zsh" => home.join(".zshrc"),
        "fish" => home.join(".config/fish/config.fish"),
        "sh" => home.join(".profile"),
        _ => return None,
    })
}

pub fn rc_path(shell: &str) -> Option<PathBuf> {
    dirs::home_dir().and_then(|h| rc_path_for_home(&h, shell))
}

fn integration_line(shell: &str) -> Option<&'static str> {
    Some(match shell {
        "bash" => r#"eval "$(idoit init bash)""#,
        "zsh" => r#"eval "$(idoit init zsh)""#,
        "fish" => "idoit init fish | source",
        "sh" => r#"eval "$(idoit init sh)""#,
        _ => return None,
    })
}

fn block_for_shell(shell: &str) -> Option<String> {
    let line = integration_line(shell)?;
    Some(format!("\n{BLOCK_START}\n{line}\n{BLOCK_END}\n"))
}

fn strip_existing_block(content: &mut String) {
    while let Some(start) = content.find(BLOCK_START) {
        let rest = &content[start..];
        let Some(end_rel) = rest.find(BLOCK_END) else {
            content.truncate(start);
            return;
        };
        let end = start + end_rel + BLOCK_END.len();
        let tail = content[end..].trim_start_matches('\n').to_string();
        content.truncate(start);
        content.push_str(&tail);
    }
}

/// Merges existing rc text with a single idoit block for `shell` (for tests and reuse).
fn merge_rc_content(existing: &str, shell: &str) -> Option<String> {
    let block = block_for_shell(shell)?;
    let mut content = existing.to_string();
    strip_existing_block(&mut content);
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&block);
    Some(content)
}

fn apply_in_home(home: &Path, shell: &str) -> Result<PathBuf> {
    let path = rc_path_for_home(home, shell).context("no rc file mapping for this shell")?;
    let prior = if path.exists() {
        fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?
    } else {
        String::new()
    };
    let merged = merge_rc_content(&prior, shell).context("unsupported shell for rc install")?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create directory {}", parent.display()))?;
    }

    fs::write(&path, merged).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

/// Ensures exactly one idoit block is present at the end of the rc file for this shell.
pub fn apply(shell: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("no home directory")?;
    apply_in_home(&home, shell)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rc_path_for_home_bash_zsh_fish_sh() {
        let home = Path::new("/tmp/fakehome");
        assert_eq!(
            rc_path_for_home(home, "bash").unwrap(),
            home.join(".bashrc")
        );
        assert_eq!(rc_path_for_home(home, "zsh").unwrap(), home.join(".zshrc"));
        assert_eq!(
            rc_path_for_home(home, "fish").unwrap(),
            home.join(".config/fish/config.fish")
        );
        assert_eq!(rc_path_for_home(home, "sh").unwrap(), home.join(".profile"));
        assert!(rc_path_for_home(home, "pwsh").is_none());
    }

    #[test]
    fn merge_rc_content_appends_block() {
        let out = merge_rc_content("export FOO=1\n", "bash").unwrap();
        assert!(out.contains(BLOCK_START));
        assert!(out.contains(r#"eval "$(idoit init bash)""#));
        assert!(out.trim_end().ends_with(BLOCK_END));
        assert!(out.contains("export FOO=1"));
        assert_eq!(out.matches(BLOCK_START).count(), 1);
    }

    #[test]
    fn merge_rc_content_replaces_previous_block() {
        let first = merge_rc_content("", "bash").unwrap();
        let second = merge_rc_content(&first, "zsh").unwrap();
        assert_eq!(second.matches(BLOCK_START).count(), 1);
        assert!(second.contains(r#"eval "$(idoit init zsh)""#));
        assert!(!second.contains(r#"eval "$(idoit init bash)""#));
    }

    #[test]
    fn merge_rc_content_strips_unclosed_block() {
        let broken = format!("before\n{BLOCK_START}\neval broken\n");
        let out = merge_rc_content(&broken, "fish").unwrap();
        assert_eq!(out.matches(BLOCK_START).count(), 1);
        assert!(out.contains("idoit init fish | source"));
        assert!(!out.contains("eval broken"));
    }

    #[test]
    fn merge_rc_content_adds_newline_before_block_when_needed() {
        let out = merge_rc_content("line", "sh").unwrap();
        assert!(out.contains("line\n"));
        assert!(out.contains(BLOCK_START));
    }

    #[test]
    fn merge_rc_content_none_for_unknown_shell() {
        assert!(merge_rc_content("", "elvish").is_none());
    }

    #[test]
    fn apply_in_home_creates_nested_fish_config() {
        let tmp = std::env::temp_dir().join(format!(
            "idoit-rc-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&tmp).unwrap();
        let path = apply_in_home(&tmp, "fish").unwrap();
        assert_eq!(path, tmp.join(".config/fish/config.fish"));
        let body = fs::read_to_string(&path).unwrap();
        assert!(body.contains("idoit init fish | source"));
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn apply_in_home_idempotent() {
        let tmp = std::env::temp_dir().join(format!(
            "idoit-rc-idem-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&tmp).unwrap();
        apply_in_home(&tmp, "bash").unwrap();
        apply_in_home(&tmp, "bash").unwrap();
        let body = fs::read_to_string(tmp.join(".bashrc")).unwrap();
        assert_eq!(body.matches(BLOCK_START).count(), 1);
        fs::remove_dir_all(&tmp).unwrap();
    }
}
