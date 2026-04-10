use std::process::{Command, Stdio};
use std::io::Write;

/// Copy text to the system clipboard. Returns true on success.
#[allow(dead_code)]
pub fn copy(text: &str) -> bool {
    let candidates = if cfg!(target_os = "macos") {
        vec!["pbcopy"]
    } else if cfg!(target_os = "linux") {
        vec!["wl-copy", "xclip", "xsel"]
    } else {
        vec!["clip"]
    };

    for tool in &candidates {
        if let Ok(mut child) = Command::new(tool)
            .args(clipboard_args(tool))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(text.as_bytes());
            }
            if let Ok(status) = child.wait() {
                if status.success() {
                    return true;
                }
            }
        }
    }

    false
}

fn clipboard_args(tool: &str) -> Vec<&str> {
    match tool {
        "xclip" => vec!["-selection", "clipboard"],
        "xsel" => vec!["--clipboard", "--input"],
        _ => vec![],
    }
}
