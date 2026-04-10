use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

const GIT_SUBS: &[&str] = &[
    "add",
    "am",
    "annotate",
    "apply",
    "archive",
    "bisect",
    "blame",
    "branch",
    "bundle",
    "checkout",
    "cherry-pick",
    "clean",
    "clone",
    "commit",
    "config",
    "describe",
    "diff",
    "fetch",
    "format-patch",
    "grep",
    "gui",
    "init",
    "log",
    "merge",
    "mv",
    "notes",
    "pull",
    "push",
    "rebase",
    "reset",
    "restore",
    "revert",
    "rm",
    "show",
    "stash",
    "status",
    "submodule",
    "switch",
    "tag",
    "worktree",
];

const CARGO_SUBS: &[&str] = &[
    "add",
    "bench",
    "build",
    "b",
    "check",
    "c",
    "clean",
    "clippy",
    "doc",
    "d",
    "fetch",
    "fix",
    "fmt",
    "generate-lockfile",
    "git-checkout",
    "help",
    "init",
    "install",
    "locate-project",
    "login",
    "logout",
    "metadata",
    "new",
    "owner",
    "package",
    "pkgid",
    "publish",
    "read-manifest",
    "remove",
    "report",
    "run",
    "r",
    "rustc",
    "rustdoc",
    "search",
    "test",
    "t",
    "tree",
    "uninstall",
    "update",
    "vendor",
    "verify-project",
    "version",
    "yank",
];

fn path_executables() -> &'static BTreeSet<String> {
    static PATH_EXECUTABLES: OnceLock<BTreeSet<String>> = OnceLock::new();
    PATH_EXECUTABLES.get_or_init(|| {
        let mut out = BTreeSet::new();
        let path_var = env::var("PATH").unwrap_or_default();
        for dir in env::split_paths(&path_var) {
            if let Ok(entries) = fs::read_dir(&dir) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.is_file() && is_executable(&p) {
                        if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                            if !name.is_empty() && !name.contains('/') {
                                out.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        out
    })
}

#[cfg(unix)]
fn is_executable(path: &PathBuf) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &PathBuf) -> bool {
    true
}

/// Returns candidate completions for the **last** token (full replacement words, not suffix-only).
pub fn shell_candidates(line: &str) -> Vec<String> {
    let line = line;
    let ends_space = line.ends_with(' ') && !line.trim().is_empty();
    let words: Vec<&str> = line.split_whitespace().collect();

    if words.is_empty() {
        return Vec::new();
    }

    // Only complete first word (command) or second word for known tools.
    if words.len() > 2 && !ends_space {
        return Vec::new();
    }

    if words.len() >= 2 && !ends_space {
        let cmd = words[0];
        let token = words[words.len() - 1];
        return match cmd {
            "git" => filter_prefix(GIT_SUBS, token),
            "cargo" => filter_prefix(CARGO_SUBS, token),
            _ => Vec::new(),
        };
    }

    if ends_space && words.len() == 1 {
        let cmd = words[0];
        return match cmd {
            "git" => GIT_SUBS.iter().map(|s| (*s).to_string()).collect(),
            "cargo" => CARGO_SUBS.iter().map(|s| (*s).to_string()).collect(),
            _ => Vec::new(),
        };
    }

    // Single token — PATH executables
    if words.len() == 1 && !ends_space {
        let token = words[0];
        return path_executables()
            .iter()
            .filter(|name| name.starts_with(token))
            .take(80)
            .cloned()
            .collect();
    }

    Vec::new()
}

fn filter_prefix(subs: &[&str], token: &str) -> Vec<String> {
    let mut v: Vec<String> = subs
        .iter()
        .filter(|s| s.starts_with(token))
        .map(|s| (*s).to_string())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// `(prefix_including_spaces, last_token)` for ghost suffix rendering.
pub fn split_last_token(line: &str) -> (String, String) {
    if line.is_empty() {
        return (String::new(), String::new());
    }
    if line.ends_with(' ') {
        return (line.to_string(), String::new());
    }
    match line.rfind(' ') {
        None => (String::new(), line.to_string()),
        Some(i) => (line[..=i].to_string(), line[i + 1..].to_string()),
    }
}

/// Ghost suffix after typed token (e.g. `i` + `nit` -> completes `init`).
pub fn ghost_suffix(token: &str, candidate: &str) -> Option<String> {
    if candidate.starts_with(token) && candidate != token {
        Some(candidate[token.len()..].to_string())
    } else if token.is_empty() && !candidate.is_empty() {
        Some(candidate.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghost_suffix_init() {
        assert_eq!(ghost_suffix("i", "init"), Some("nit".into()));
    }

    #[test]
    fn split_last() {
        let (p, t) = split_last_token("git i");
        assert_eq!(p, "git ");
        assert_eq!(t, "i");
    }
}
