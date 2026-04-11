use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use super::context::ShellContext;

#[derive(Debug)]
pub struct ExecutionResult {
    pub exit_code: i32,
    #[allow(dead_code)]
    pub success: bool,
}

fn shell_command(shell_exe: &str, script: &str) -> Command {
    let mut cmd = Command::new(shell_exe);
    #[cfg(windows)]
    {
        let lower = shell_exe.to_ascii_lowercase();
        if lower.ends_with("cmd.exe") || lower == "cmd" {
            cmd.arg("/c").arg(script);
        } else if lower.contains("powershell.exe")
            || lower.contains("\\pwsh.exe")
            || lower.ends_with("pwsh.exe")
            || lower == "pwsh"
            || lower == "powershell"
        {
            cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        } else {
            cmd.arg("-c").arg(script);
        }
    }
    #[cfg(not(windows))]
    {
        cmd.arg("-c").arg(script);
    }
    cmd
}

pub fn execute(ctx: &ShellContext, command: &str) -> Result<ExecutionResult> {
    let shell = ctx.shell_executable();
    let shell_ref = shell.as_ref();

    let mut child = shell_command(shell_ref, command)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn {shell_ref}"))?;

    let status = child.wait().with_context(|| "command execution failed")?;
    let exit_code = status.code().unwrap_or(1);

    Ok(ExecutionResult {
        exit_code,
        success: status.success(),
    })
}

#[allow(dead_code)]
pub fn execute_capture(ctx: &ShellContext, command: &str) -> Result<(String, String, i32)> {
    let shell = ctx.shell_executable();
    let shell_ref = shell.as_ref();

    let output = shell_command(shell_ref, command)
        .output()
        .with_context(|| format!("failed to spawn {shell_ref}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(1);

    Ok((stdout, stderr, code))
}

#[cfg(test)]
mod tests {
    use super::execute_capture;
    use crate::shell::context::ShellContext;

    #[cfg(unix)]
    #[test]
    fn execute_capture_runs_shell_command() {
        let ctx = ShellContext {
            os: "unix".into(),
            shell: "sh".into(),
            cwd: "/".into(),
            available_tools: vec![],
            home: "/".into(),
        };
        let (out, err, code) = execute_capture(&ctx, "echo idoit_test_marker").unwrap();
        assert_eq!(code, 0, "stderr={err}");
        assert!(out.contains("idoit_test_marker"), "out={out}");
    }

    #[cfg(windows)]
    #[test]
    fn execute_capture_runs_cmd() {
        let ctx = ShellContext {
            os: "windows".into(),
            shell: "cmd".into(),
            cwd: "C:\\".into(),
            available_tools: vec![],
            home: "C:\\".into(),
        };
        let (out, err, code) = execute_capture(&ctx, "echo idoit_test_marker").unwrap();
        assert_eq!(code, 0, "stderr={err}");
        assert!(out.contains("idoit_test_marker"), "out={out}");
    }
}
