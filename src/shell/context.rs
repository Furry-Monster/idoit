use std::borrow::Cow;
use std::env;
use std::path::PathBuf;

/// Last path segment, treating both `/` and `\\` as separators (for `SHELL` / `ComSpec` strings).
pub(crate) fn shell_basename_from_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    let trimmed = path.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed
        .rsplit(&['/', '\\'][..])
        .next()
        .unwrap_or("bash")
        .to_string()
}

const COMMON_TOOLS: &[&str] = &[
    "grep",
    "rg",
    "find",
    "fd",
    "awk",
    "sed",
    "sort",
    "uniq",
    "wc",
    "head",
    "tail",
    "cat",
    "less",
    "tar",
    "gzip",
    "zip",
    "unzip",
    "curl",
    "wget",
    "jq",
    "xargs",
    "cut",
    "tr",
    "diff",
    "patch",
    "git",
    "docker",
    "ssh",
    "rsync",
    "make",
    "cmake",
    "python",
    "python3",
    "node",
    "npm",
    "cargo",
    "go",
    "java",
    "ruby",
    "perl",
    "ffmpeg",
    "convert",
    "ls",
    "cp",
    "mv",
    "rm",
    "mkdir",
    "chmod",
    "chown",
    "ln",
    "du",
    "df",
    "ps",
    "kill",
    "top",
    "htop",
    "systemctl",
];

#[derive(Debug, Clone)]
pub struct ShellContext {
    pub os: String,
    pub shell: String,
    pub cwd: String,
    pub available_tools: Vec<String>,
    #[allow(dead_code)]
    pub home: String,
}

impl ShellContext {
    pub fn detect(shell_override: &str) -> Self {
        let os = detect_os();
        let shell = if shell_override.is_empty() {
            detect_shell()
        } else {
            shell_override.to_string()
        };
        let cwd = env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .display()
            .to_string();
        let home = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .display()
            .to_string();
        let available_tools = detect_available_tools();

        Self {
            os,
            shell,
            cwd,
            available_tools,
            home,
        }
    }

    /// Executable path or name suitable for [`std::process::Command::new`].
    pub fn shell_executable(&self) -> Cow<'_, str> {
        if self.shell.contains('/') || self.shell.contains('\\') {
            return Cow::Borrowed(self.shell.as_str());
        }

        #[cfg(unix)]
        {
            match self.shell.as_str() {
                "bash" => Cow::Borrowed("/bin/bash"),
                "zsh" => Cow::Borrowed("/bin/zsh"),
                "fish" => Cow::Borrowed("/usr/bin/fish"),
                _ => Cow::Borrowed("/bin/sh"),
            }
        }

        #[cfg(windows)]
        {
            let s = self.shell.as_str();
            let lower = s.to_ascii_lowercase();
            match lower.as_str() {
                "cmd" | "cmd.exe" => Cow::Owned(
                    env::var("ComSpec").unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".into()),
                ),
                "powershell" | "powershell.exe" => Cow::Borrowed("powershell"),
                "pwsh" | "pwsh.exe" => Cow::Borrowed("pwsh"),
                _ => Cow::Borrowed(s),
            }
        }
    }
}

fn detect_os() -> String {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    format!("{os}/{arch}")
}

fn detect_shell() -> String {
    #[cfg(windows)]
    {
        if let Ok(s) = env::var("SHELL") {
            let b = shell_basename_from_path(&s);
            if !b.is_empty() {
                return b;
            }
        }
        let b = env::var("ComSpec")
            .map(|p| shell_basename_from_path(&p))
            .unwrap_or_default();
        if !b.is_empty() {
            return b;
        }
        "cmd.exe".into()
    }

    #[cfg(not(windows))]
    {
        env::var("SHELL")
            .unwrap_or_else(|_| "/bin/sh".to_string())
            .rsplit('/')
            .next()
            .unwrap_or("sh")
            .to_string()
    }
}

fn detect_available_tools() -> Vec<String> {
    // Batch check using PATH lookup instead of spawning N `which` processes
    let path_var = env::var("PATH").unwrap_or_default();
    let path_dirs: Vec<PathBuf> = env::split_paths(&path_var).collect();

    COMMON_TOOLS
        .iter()
        .filter(|tool| tool_on_path(tool, &path_dirs))
        .map(|t| t.to_string())
        .collect()
}

fn tool_on_path(tool: &str, path_dirs: &[PathBuf]) -> bool {
    #[cfg(windows)]
    {
        let exts: Vec<String> = env::var("PATHEXT")
            .map(|raw| {
                env::split_paths(&raw)
                    .filter_map(|p| p.to_str().map(|s| s.to_ascii_lowercase()))
                    .collect()
            })
            .unwrap_or_else(|_| vec![".exe".into(), ".cmd".into(), ".bat".into(), ".com".into()]);

        for dir in path_dirs {
            let base = dir.join(tool);
            if base.is_file() {
                return true;
            }
            for ext in &exts {
                let with_ext = dir.join(format!("{tool}{ext}"));
                if with_ext.is_file() {
                    return true;
                }
            }
        }
        false
    }

    #[cfg(not(windows))]
    {
        path_dirs.iter().any(|dir| dir.join(tool).is_file())
    }
}

#[cfg(test)]
mod tests {
    use super::ShellContext;

    fn dummy() -> ShellContext {
        ShellContext {
            os: "linux/x86_64".into(),
            shell: "bash".into(),
            cwd: "/tmp".into(),
            available_tools: vec![],
            home: "/home/u".into(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn shell_executable_named_shells() {
        let mut c = dummy();
        c.shell = "bash".into();
        assert_eq!(c.shell_executable().as_ref(), "/bin/bash");
        c.shell = "zsh".into();
        assert_eq!(c.shell_executable().as_ref(), "/bin/zsh");
        c.shell = "fish".into();
        assert_eq!(c.shell_executable().as_ref(), "/usr/bin/fish");
        c.shell = "sh".into();
        assert_eq!(c.shell_executable().as_ref(), "/bin/sh");
    }

    #[test]
    fn shell_executable_absolute_passthrough() {
        let mut c = dummy();
        c.shell = "/opt/homebrew/bin/fish".into();
        assert_eq!(c.shell_executable().as_ref(), "/opt/homebrew/bin/fish");
    }

    #[cfg(windows)]
    #[test]
    fn shell_executable_cmd_uses_comspec() {
        let mut c = dummy();
        c.shell = "cmd".into();
        let exe = c.shell_executable();
        assert!(exe.ends_with("cmd.exe") || exe == "cmd");
    }
}
