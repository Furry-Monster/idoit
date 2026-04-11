//! Single long-lived task: trailing debounce on input `watch`, then AI calls.
//! Results are published via `watch` with a monotonic `seq` so every completion
//! notifies the UI (watch suppresses sends when `PartialEq` matches).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;

use crate::ai::client::AiClient;
use crate::ai::prompt;
use crate::ai::types::AiCommandResponse;
use crate::config::settings::Settings;
use crate::macros;
use crate::session::context::LayeredContextCache;
use crate::session::{self, SessionEntry};
use crate::shell::context::ShellContext;

use super::app::App;

const DIAG_THROTTLE: Duration = Duration::from_millis(50);

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
    /// When false, more chunks may follow for this request.
    pub done: bool,
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

fn bump_seq(seq: &AtomicU64) -> u64 {
    seq.fetch_add(1, Ordering::SeqCst) + 1
}

/// Streams learn-mode diagnostic text with ~50ms UI batching.
#[allow(clippy::too_many_arguments)]
async fn run_learn_diag_stream(
    client: Arc<AiClient>,
    settings: Arc<Settings>,
    sys_diag: String,
    user_diag: String,
    model: String,
    gen: u64,
    ai_live: Arc<AtomicU64>,
    cancel: CancellationToken,
    seq: Arc<AtomicU64>,
    diag_tx: watch::Sender<Option<DiagUpdate>>,
) -> Result<String, String> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let c2 = cancel.clone();
    let stream_h = tokio::spawn({
        let client = Arc::clone(&client);
        let settings = Arc::clone(&settings);
        async move {
            client
                .ask_freeform_stream(
                    &sys_diag,
                    &user_diag,
                    &model,
                    &settings,
                    Some(&c2),
                    move |d| {
                        let _ = tx.send(d.to_string());
                    },
                )
                .await
        }
    });

    let mut acc = String::new();
    let mut tick = tokio::time::interval(DIAG_THROTTLE);
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    tick.tick().await;

    loop {
        if ai_live.load(Ordering::SeqCst) != gen {
            cancel.cancel();
            let _ = stream_h.await;
            return Err("cancelled".into());
        }

        tokio::select! {
            biased;
            m = rx.recv() => {
                match m {
                    Some(s) => acc.push_str(&s),
                    None => {
                        let join = stream_h.await;
                        let res = match join {
                            Ok(Ok((full, _))) => Ok(full),
                            Ok(Err(e)) => Err(e.to_string()),
                            Err(j) => Err(j.to_string()),
                        };

                        if ai_live.load(Ordering::SeqCst) == gen {
                            match &res {
                                Ok(full) => {
                                    let s = bump_seq(seq.as_ref());
                                    let _ = diag_tx.send(Some(DiagUpdate {
                                        seq: s,
                                        gen,
                                        text: Ok(full.clone()),
                                        done: true,
                                    }));
                                }
                                Err(e) => {
                                    let s = bump_seq(seq.as_ref());
                                    let _ = diag_tx.send(Some(DiagUpdate {
                                        seq: s,
                                        gen,
                                        text: Err(e.clone()),
                                        done: true,
                                    }));
                                }
                            }
                        }
                        return res;
                    }
                }
            }
            _ = tick.tick(), if !acc.is_empty() => {
                if ai_live.load(Ordering::SeqCst) == gen {
                    let s = bump_seq(seq.as_ref());
                    let _ = diag_tx.send(Some(DiagUpdate {
                        seq: s,
                        gen,
                        text: Ok(acc.clone()),
                        done: false,
                    }));
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
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

#[allow(clippy::too_many_arguments)]
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
    let mut ctx_cache = LayeredContextCache::new();
    let mut round_cancel: Option<CancellationToken> = None;

    'coord: loop {
        if input_rx.changed().await.is_err() {
            break;
        }

        // Trailing debounce only: after each keystroke, wait `tui_debounce_ms` with no further
        // changes before calling the model. (Older logic used a 300ms floor on follow-up waits,
        // which made low `ui.tui_debounce_ms` settings ineffective and added hundreds of ms after
        // fast typing.)
        let base = Duration::from_millis(settings.ui.tui_debounce_ms.max(1));
        loop {
            let deadline = tokio::time::Instant::now() + base;
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

        if let Some(c) = round_cancel.take() {
            c.cancel();
        }
        let cancel = CancellationToken::new();
        round_cancel = Some(cancel.clone());

        let gen = tick.gen;
        let line_trim = tick.line.trim();

        let expanded_line = if line_trim.len() >= 2 {
            macros::expand(line_trim).text
        } else {
            String::new()
        };

        let idoit_snap = app.lock().unwrap().idoit_run.clone();
        let ctx_block = ctx_cache
            .gather(&ctx, &settings, Some(&idoit_snap))
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

        if learn_mode {
            if line_trim.len() < 2 {
                let t_res = empty_translate();
                let d_res = if line_trim.is_empty() {
                    Ok("note: start typing a command or describe what you want to do".into())
                } else {
                    let diag_user = prompt::with_shell_context(&expanded_line, &ctx_block);
                    let sys_diag = prompt::tui_learn_diagnostic_system(&ctx);
                    client
                        .ask_freeform(&sys_diag, &diag_user, &model, &settings, Some(&cancel))
                        .await
                        .map(|(s, _)| s)
                        .map_err(|e| e.to_string())
                };

                if ai_live.load(Ordering::SeqCst) != gen {
                    continue;
                }

                if let Ok(ref resp) = t_res {
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
                    result: t_res,
                }));

                out_seq = out_seq.wrapping_add(1);
                let _ = diag_tx.send(Some(DiagUpdate {
                    seq: out_seq,
                    gen,
                    text: d_res,
                    done: true,
                }));
                continue;
            }

            let user_t = prompt::with_shell_context(&expanded_line, &ctx_block);
            let user_d = user_t.clone();
            let sys_diag = prompt::tui_learn_diagnostic_system(&ctx);

            let seq_atomic = Arc::new(AtomicU64::new(out_seq));
            let translate_cancel = cancel.clone();
            let translate_client = Arc::clone(&client);
            let translate_settings = Arc::clone(&settings);
            let sys_tr = sys_translate.clone();
            let model_tr = model.clone();

            let translate_h = tokio::spawn(async move {
                translate_client
                    .ask_command(
                        &sys_tr,
                        &user_t,
                        &model_tr,
                        &translate_settings,
                        None,
                        Some(&translate_cancel),
                    )
                    .await
                    .map(|a| a.response)
                    .map_err(|e| e.to_string())
            });

            let diag_client = Arc::clone(&client);
            let diag_settings = Arc::clone(&settings);
            let diag_cancel = cancel.clone();
            let diag_seq = Arc::clone(&seq_atomic);
            let diag_ai_live = Arc::clone(&ai_live);
            let diag_tx_clone = diag_tx.clone();
            let sys_d = sys_diag.clone();
            let user_d_clone = user_d.clone();
            let model_d = model.clone();

            let diag_h = tokio::spawn(async move {
                run_learn_diag_stream(
                    diag_client,
                    diag_settings,
                    sys_d,
                    user_d_clone,
                    model_d,
                    gen,
                    diag_ai_live,
                    diag_cancel,
                    diag_seq,
                    diag_tx_clone,
                )
                .await
            });

            let t_res = translate_h.await.unwrap_or_else(|j| Err(j.to_string()));

            if ai_live.load(Ordering::SeqCst) == gen {
                if let Ok(ref resp) = t_res {
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

                let tr_seq = bump_seq(seq_atomic.as_ref());
                let _ = translate_tx.send(Some(TranslateUpdate {
                    seq: tr_seq,
                    gen,
                    result: t_res,
                }));
            }

            let _ = diag_h.await;

            out_seq = seq_atomic.load(Ordering::SeqCst);
            if ai_live.load(Ordering::SeqCst) != gen {
                continue 'coord;
            }
            continue;
        }

        if line_trim.len() < 2 {
            let translate_result = empty_translate();
            out_seq = out_seq.wrapping_add(1);
            let _ = translate_tx.send(Some(TranslateUpdate {
                seq: out_seq,
                gen,
                result: translate_result,
            }));
            continue;
        }

        let user_t = prompt::with_shell_context(&expanded_line, &ctx_block);
        let t_res = client
            .ask_command(
                &sys_translate,
                &user_t,
                &model,
                &settings,
                None,
                Some(&cancel),
            )
            .await
            .map(|a| a.response)
            .map_err(|e| e.to_string());

        if ai_live.load(Ordering::SeqCst) != gen {
            continue;
        }

        if let Ok(ref resp) = t_res {
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
            result: t_res,
        }));
    }
}
