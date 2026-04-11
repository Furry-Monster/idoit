//! Append or update a marked idoit block in the user shell rc file.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

const BLOCK_START: &str = "# >>> idoit shell integration (managed by idoit setup) >>>";
const BLOCK_END: &str = "# <<< idoit shell integration <<<";

pub fn rc_path(shell: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(match shell {
        "bash" => home.join(".bashrc"),
        "zsh" => home.join(".zshrc"),
        "fish" => home.join(".config/fish/config.fish"),
        "sh" => home.join(".profile"),
        _ => return None,
    })
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
    Some(format!(
        "\n{BLOCK_START}\n{line}\n{BLOCK_END}\n"
    ))
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

/// Ensures exactly one idoit block is present at the end of the rc file for this shell.
pub fn apply(shell: &str) -> Result<PathBuf> {
    let path = rc_path(shell).context("no rc file mapping for this shell")?;
    let block = block_for_shell(shell).context("unsupported shell for rc install")?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create directory {}", parent.display()))?;
    }

    let mut content = if path.exists() {
        fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?
    } else {
        String::new()
    };

    strip_existing_block(&mut content);
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&block);

    fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}
