use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use super::context::ShellContext;

#[derive(Debug)]
pub struct ExecutionResult {
    pub exit_code: i32,
    #[allow(dead_code)]
    pub success: bool,
}

pub fn execute(ctx: &ShellContext, command: &str) -> Result<ExecutionResult> {
    let shell = ctx.shell_path();

    let mut child = Command::new(shell)
        .arg("-c")
        .arg(command)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn {shell}"))?;

    let status = child.wait().with_context(|| "command execution failed")?;
    let exit_code = status.code().unwrap_or(1);

    Ok(ExecutionResult {
        exit_code,
        success: status.success(),
    })
}

#[allow(dead_code)]
pub fn execute_capture(ctx: &ShellContext, command: &str) -> Result<(String, String, i32)> {
    let shell = ctx.shell_path();

    let output = Command::new(shell)
        .arg("-c")
        .arg(command)
        .output()
        .with_context(|| format!("failed to spawn {shell}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(1);

    Ok((stdout, stderr, code))
}

#[cfg(all(test, unix))]
mod tests {
    use super::execute_capture;
    use crate::shell::context::ShellContext;

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
}
