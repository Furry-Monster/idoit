mod ai_coordinator;
mod app;
mod completion;
mod draw;
mod keys;
mod terminal;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, KeyEventKind};
use futures::StreamExt;
use tokio::sync::watch;
use tokio::time::MissedTickBehavior;

use crate::ai::client::AiClient;
use crate::config::settings::Settings;
use crate::shell::context::ShellContext;

use ai_coordinator::{spawn_ai_coordinator, TranslateUpdate};
use app::App;
use terminal::TerminalRestore;

/// Full-screen idoit TUI. Exits on Esc / Ctrl+C.
pub async fn run(
    settings: Arc<Settings>,
    client: Arc<AiClient>,
    ctx: Arc<ShellContext>,
    learn_mode: bool,
    anyway: bool,
    dry_run: bool,
) -> Result<()> {
    let _restore = TerminalRestore;
    let out = terminal::enter_tui_screen()?;
    let mut terminal = terminal::new_terminal(out)?;

    let ai_live = Arc::new(AtomicU64::new(0));
    let app = Arc::new(Mutex::new(App::new(
        learn_mode,
        dry_run,
        Arc::clone(&ai_live),
    )));

    // Latest AI results only (watch overwrites; no unbounded queue).
    let (translate_tx, mut translate_rx) = watch::channel(None::<TranslateUpdate>);
    let (diag_tx, mut diag_rx) = watch::channel(None::<ai_coordinator::DiagUpdate>);

    let coordinator = spawn_ai_coordinator(
        Arc::clone(&settings),
        Arc::clone(&client),
        Arc::clone(&ctx),
        Arc::clone(&ai_live),
        anyway,
        learn_mode,
        translate_tx,
        diag_tx,
    );

    coordinator.notify_input_changed(&app, learn_mode);

    let cursor_visible = Arc::new(AtomicBool::new(true));
    let cursor_blink = Arc::clone(&cursor_visible);
    let mut blink = tokio::time::interval(Duration::from_millis(500));
    blink.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut reader = crossterm::event::EventStream::new();

    loop {
        {
            let g = app.lock().unwrap();
            terminal.draw(|f| {
                draw::draw(
                    f,
                    &g,
                    cursor_visible.load(Ordering::Relaxed),
                );
            })?;
        }

        tokio::select! {
            biased;
            res = translate_rx.changed() => {
                res?;
                if let Some(u) = translate_rx.borrow().clone() {
                    let mut st = app.lock().unwrap();
                    match u.result {
                        Ok(r) => st.apply_ai_translate(u.gen, r),
                        Err(e) => st.apply_ai_err(u.gen, e),
                    }
                }
            }
            res = diag_rx.changed(), if learn_mode => {
                res?;
                if let Some(d) = diag_rx.borrow().clone() {
                    let mut st = app.lock().unwrap();
                    match d.text {
                        Ok(t) => st.apply_diag(d.gen, t),
                        Err(e) => st.apply_diag_err(d.gen, e),
                    }
                }
            }
            _ = blink.tick() => {
                cursor_blink.fetch_xor(true, Ordering::Relaxed);
            }
            maybe_ev = reader.next() => {
                match maybe_ev {
                    Some(Ok(Event::Resize(_, _))) => {}
                    Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                        if keys::handle_key(
                            Arc::clone(&app),
                            Arc::clone(&ctx),
                            &coordinator,
                            key,
                            learn_mode,
                        )
                        .await?
                        {
                            break;
                        }
                    }
                    Some(Err(e)) => return Err(e.into()),
                    None => break,
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
