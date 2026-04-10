//! Single long-lived task: trailing debounce on input `watch`, then AI calls.
//! Results are published via `watch` with a monotonic `seq` so every completion
//! notifies the UI (watch suppresses sends when `PartialEq` matches).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::watch;

use crate::ai::client::AiClient;
use crate::ai::prompt;
use crate::ai::types::AiCommandResponse;
use crate::config::settings::Settings;
use crate::macros;
use crate::session::{self, SessionEntry};
use crate::shell::context::ShellContext;

use super::app::App;

const DEBOUNCE: Duration = Duration::from_millis(400);

#[derive(Clone, PartialEq, Eq)]
pub struct InputTick {
    pub gen: u64,
    pub line: String,
}

#[derive(Clone)]
pub struct TranslateUpdate {
    pub seq: u64,
    pub gen: u64,
    pub result: Result<AiCommandResponse, String>,
}

impl PartialEq for TranslateUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.seq == other.seq
    }
}

impl Eq for TranslateUpdate {}

#[derive(Clone)]
pub struct DiagUpdate {
    pub seq: u64,
    pub gen: u64,
    pub text: Result<String, String>,
}

impl PartialEq for DiagUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.seq == other.seq
    }
}

impl Eq for DiagUpdate {}

pub struct AiCoordinatorHandle {
    input_tx: watch::Sender<InputTick>,
}

impl AiCoordinatorHandle {
    /// Bump generation, set pending flags, notify the coordinator (debounced).
    pub fn notify_input_changed(&self, app: &Mutex<App>, learn_mode: bool) {
        let tick = {
            let mut st = app.lock().unwrap();
            let line = st.input.clone();
            let gen = st.bump_ai_gen();
            st.trans_pending = line.trim().len() >= 2;
            st.diag_pending = learn_mode;
            InputTick { gen, line }
        };
        let _ = self.input_tx.send(tick);
    }
}

pub fn spawn_ai_coordinator(
    settings: Arc<Settings>,
    client: Arc<AiClient>,
    ctx: Arc<ShellContext>,
    app: Arc<Mutex<App>>,
    ai_live: Arc<AtomicU64>,
    anyway: bool,
    learn_mode: bool,
    translate_tx: watch::Sender<Option<TranslateUpdate>>,
    diag_tx: watch::Sender<Option<DiagUpdate>>,
) -> AiCoordinatorHandle {
    let (input_tx, input_rx) = watch::channel(InputTick {
        gen: 0,
        line: String::new(),
    });

    let input_tx_ret = input_tx.clone();
    tokio::spawn(coordinator_loop(
        input_rx,
        settings,
        client,
        ctx,
        app,
        ai_live,
        anyway,
        learn_mode,
        translate_tx,
        diag_tx,
    ));

    AiCoordinatorHandle {
        input_tx: input_tx_ret,
    }
}

async fn coordinator_loop(
    mut input_rx: watch::Receiver<InputTick>,
    settings: Arc<Settings>,
    client: Arc<AiClient>,
    ctx: Arc<ShellContext>,
    app: Arc<Mutex<App>>,
    ai_live: Arc<AtomicU64>,
    anyway: bool,
    learn_mode: bool,
    translate_tx: watch::Sender<Option<TranslateUpdate>>,
    diag_tx: watch::Sender<Option<DiagUpdate>>,
) {
    let mut out_seq: u64 = 0;

    loop {
        if input_rx.changed().await.is_err() {
            break;
        }

        loop {
            let deadline = tokio::time::Instant::now() + DEBOUNCE;
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                res = input_rx.changed() => {
                    if res.is_err() {
                        return;
                    }
                }
            }
        }

        let tick = input_rx.borrow().clone();
        if ai_live.load(Ordering::SeqCst) != tick.gen {
            continue;
        }

        let gen = tick.gen;
        let line_trim = tick.line.trim();

        let expanded_line = if line_trim.len() >= 2 {
            macros::expand(line_trim).text
        } else {
            String::new()
        };

        let idoit_snap = app.lock().unwrap().idoit_run.clone();
        let ctx_block =
            session::context::LayeredContext::gather(&ctx, &settings, Some(&idoit_snap))
                .format_block();

        let sys_translate = prompt::translate_system(&ctx, anyway);
        let model = client.model_name(&settings);

        let empty_translate = || {
            Ok(AiCommandResponse {
                command: String::new(),
                explanation: String::new(),
                missing_tools: vec![],
                confidence: 0.0,
                teaching: None,
                alternates: vec![],
            })
        };

        let (translate_result, diag_result) = if learn_mode {
            if line_trim.len() < 2 {
                let t_res = empty_translate();
                let d_res = if line_trim.is_empty() {
                    Ok("note: start typing a command or describe what you want to do".into())
                } else {
                    let diag_user = prompt::with_shell_context(&expanded_line, &ctx_block);
                    let sys_diag = prompt::tui_learn_diagnostic_system(&ctx);
                    client
                        .ask_freeform(&sys_diag, &diag_user, &model, &settings)
                        .await
                        .map(|(s, _)| s)
                        .map_err(|e| e.to_string())
                };
                (t_res, d_res)
            } else {
                let user_t = prompt::with_shell_context(&expanded_line, &ctx_block);
                let user_d = user_t.clone();
                let sys_diag = prompt::tui_learn_diagnostic_system(&ctx);
                let (t_res, d_res) = tokio::join!(
                    async {
                        client
                            .ask_command(&sys_translate, &user_t, &model, &settings, None)
                            .await
                            .map(|a| a.response)
                            .map_err(|e| e.to_string())
                    },
                    async {
                        client
                            .ask_freeform(&sys_diag, &user_d, &model, &settings)
                            .await
                            .map(|(s, _)| s)
                            .map_err(|e| e.to_string())
                    }
                );
                (t_res, d_res)
            }
        } else if line_trim.len() < 2 {
            (empty_translate(), Ok(String::new()))
        } else {
            let user_t = prompt::with_shell_context(&expanded_line, &ctx_block);
            let t_res = client
                .ask_command(&sys_translate, &user_t, &model, &settings, None)
                .await
                .map(|a| a.response)
                .map_err(|e| e.to_string());
            (t_res, Ok(String::new()))
        };

        if ai_live.load(Ordering::SeqCst) != gen {
            continue;
        }

        if let Ok(ref resp) = translate_result {
            if line_trim.len() >= 2 && !resp.command.trim().is_empty() {
                let entry = SessionEntry {
                    ts: chrono::Utc::now().to_rfc3339(),
                    input: expanded_line.clone(),
                    command: resp.command.clone(),
                    executed: false,
                    exit_code: None,
                };
                let mut g = app.lock().unwrap();
                session::push_run_buffer(&mut g.idoit_run, entry);
            }
        }

        out_seq = out_seq.wrapping_add(1);
        let _ = translate_tx.send(Some(TranslateUpdate {
            seq: out_seq,
            gen,
            result: translate_result,
        }));

        if learn_mode {
            out_seq = out_seq.wrapping_add(1);
            let _ = diag_tx.send(Some(DiagUpdate {
                seq: out_seq,
                gen,
                text: diag_result,
            }));
        }
    }
}
