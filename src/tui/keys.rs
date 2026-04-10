use std::sync::{Arc, Mutex};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};

use crate::session;
use crate::shell::context::ShellContext;
use crate::shell::executor;

use super::ai_coordinator::AiCoordinatorHandle;
use super::app::App;

pub async fn handle_key(
    app: Arc<Mutex<App>>,
    ctx: Arc<ShellContext>,
    coordinator: &AiCoordinatorHandle,
    key: crossterm::event::KeyEvent,
    learn_mode: bool,
) -> Result<bool> {
    let code = key.code;
    let mods = key.modifiers;

    match code {
        KeyCode::Esc => return Ok(true),
        KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => return Ok(true),
        KeyCode::Tab => {
            let mut st = app.lock().unwrap();
            st.apply_tab();
            drop(st);
            coordinator.notify_input_changed(&app, learn_mode);
        }
        KeyCode::Up => {
            let mut st = app.lock().unwrap();
            st.cycle_up();
        }
        KeyCode::Down => {
            let mut st = app.lock().unwrap();
            st.cycle_down();
        }
        KeyCode::PageUp => {
            let mut st = app.lock().unwrap();
            st.diag_scroll = st.diag_scroll.saturating_sub(3);
        }
        KeyCode::PageDown => {
            let mut st = app.lock().unwrap();
            st.diag_scroll = st.diag_scroll.saturating_add(3);
        }
        KeyCode::Enter => {
            let (cmd, dry) = {
                let st = app.lock().unwrap();
                (st.input.trim().to_string(), st.dry_run)
            };
            if cmd.is_empty() {
                return Ok(false);
            }

            if dry {
                let mut st = app.lock().unwrap();
                st.status_line = format!("dry-run: {cmd}");
                st.run_output.clear();
                return Ok(false);
            }

            match executor::execute_capture(&ctx, &cmd) {
                Ok((out, err, code)) => {
                    let mut st = app.lock().unwrap();
                    st.status_line = format!("exit {code}");
                    let mut buf = String::new();
                    if !out.is_empty() {
                        buf.push_str(&out);
                    }
                    if !err.is_empty() {
                        if !buf.is_empty() {
                            buf.push('\n');
                        }
                        buf.push_str(&err);
                    }
                    if buf.len() > 8000 {
                        buf.truncate(8000);
                        buf.push_str("\n… (truncated)");
                    }
                    st.run_output = buf;
                    session::record(&cmd, &cmd, true, Some(code));
                    st.input.clear();
                    st.refresh_shell();
                    st.trans_cmds.clear();
                    st.trans_expl.clear();
                    st.diagnostic.clear();
                }
                Err(e) => {
                    let mut st = app.lock().unwrap();
                    st.status_line = format!("spawn error: {e:#}");
                }
            }
            coordinator.notify_input_changed(&app, learn_mode);
        }
        KeyCode::Char(c) => {
            let mut st = app.lock().unwrap();
            st.input.push(c);
            st.refresh_shell();
            drop(st);
            coordinator.notify_input_changed(&app, learn_mode);
        }
        KeyCode::Backspace => {
            let mut st = app.lock().unwrap();
            st.input.pop();
            st.refresh_shell();
            drop(st);
            coordinator.notify_input_changed(&app, learn_mode);
        }
        _ => {}
    }
    Ok(false)
}
