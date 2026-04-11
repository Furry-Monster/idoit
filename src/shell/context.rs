use std::env;
use std::path::PathBuf;

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

    pub fn shell_path(&self) -> &str {
        if self.shell.contains('/') {
            &self.shell
        } else {
            match self.shell.as_str() {
                "bash" => "/bin/bash",
                "zsh" => "/bin/zsh",
                "fish" => "/usr/bin/fish",
                _ => "/bin/sh",
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
    env::var("SHELL")
        .unwrap_or_else(|_| "/bin/sh".to_string())
        .rsplit('/')
        .next()
        .unwrap_or("sh")
        .to_string()
}

fn detect_available_tools() -> Vec<String> {
    // Batch check using PATH lookup instead of spawning N `which` processes
    let path_var = env::var("PATH").unwrap_or_default();
    let path_dirs: Vec<PathBuf> = env::split_paths(&path_var).collect();

    COMMON_TOOLS
        .iter()
        .filter(|tool| path_dirs.iter().any(|dir| dir.join(tool).is_file()))
        .map(|t| t.to_string())
        .collect()
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

    #[test]
    fn shell_path_named_shells() {
        let mut c = dummy();
        c.shell = "bash".into();
        assert_eq!(c.shell_path(), "/bin/bash");
        c.shell = "zsh".into();
        assert_eq!(c.shell_path(), "/bin/zsh");
        c.shell = "fish".into();
        assert_eq!(c.shell_path(), "/usr/bin/fish");
        c.shell = "sh".into();
        assert_eq!(c.shell_path(), "/bin/sh");
    }

    #[test]
    fn shell_path_absolute_passthrough() {
        let mut c = dummy();
        c.shell = "/opt/homebrew/bin/fish".into();
        assert_eq!(c.shell_path(), "/opt/homebrew/bin/fish");
    }
}
