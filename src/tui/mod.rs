mod completion;

use std::io::stdout;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use completion::{ghost_suffix, shell_candidates, split_last_token};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use futures::StreamExt;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::ai::client::AiClient;
use crate::ai::prompt;
use crate::ai::types::AiCommandResponse;
use crate::config::settings::Settings;
use crate::session;
use crate::shell::context::ShellContext;
use crate::shell::executor;

struct TerminalRestore;

impl Drop for TerminalRestore {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        let _ = stdout().execute(crossterm::cursor::Show);
    }
}

enum UiMsg {
    Translate {
        gen: u64,
        result: Result<AiCommandResponse, String>,
    },
    Diagnostic {
        gen: u64,
        text: Result<String, String>,
    },
}

struct App {
    input: String,
    learn_mode: bool,
    dry_run: bool,
    /// Generation counter; incremented on each input change.
    ai_gen: u64,
    /// Latest `ai_gen` — workers compare after debounce.
    ai_live: Arc<AtomicU64>,
    shell_cands: Vec<String>,
    shell_idx: usize,
    trans_cmds: Vec<String>,
    trans_idx: usize,
    trans_expl: String,
    diagnostic: String,
    trans_pending: bool,
    diag_pending: bool,
    status_line: String,
    run_output: String,
    diag_scroll: u16,
}

impl App {
    fn new(learn_mode: bool, dry_run: bool, ai_live: Arc<AtomicU64>) -> Self {
        Self {
            input: String::new(),
            learn_mode,
            dry_run,
            ai_gen: 0,
            ai_live,
            shell_cands: Vec::new(),
            shell_idx: 0,
            trans_cmds: Vec::new(),
            trans_idx: 0,
            trans_expl: String::new(),
            diagnostic: String::new(),
            trans_pending: false,
            diag_pending: false,
            status_line: String::new(),
            run_output: String::new(),
            diag_scroll: 0,
        }
    }

    fn bump_ai_gen(&mut self) -> u64 {
        self.ai_gen = self.ai_gen.wrapping_add(1);
        self.ai_live.store(self.ai_gen, Ordering::SeqCst);
        self.ai_gen
    }

    fn refresh_shell(&mut self) {
        self.shell_cands = shell_candidates(&self.input);
        if self.shell_cands.is_empty() {
            self.shell_idx = 0;
        } else {
            self.shell_idx %= self.shell_cands.len();
        }
    }

    fn shell_ghost(&self) -> Option<String> {
        if self.shell_cands.is_empty() {
            return None;
        }
        let (_, token) = split_last_token(&self.input);
        let cand = &self.shell_cands[self.shell_idx];
        ghost_suffix(&token, cand)
    }

    fn effective_translation(&self) -> Option<&str> {
        self.trans_cmds.get(self.trans_idx).map(|s| s.as_str())
    }

    fn apply_tab(&mut self) {
        if let Some(_suf) = self.shell_ghost() {
            let (prefix, token) = split_last_token(&self.input);
            let cand = &self.shell_cands[self.shell_idx];
            if token.is_empty() {
                self.input = format!("{prefix}{cand}");
            } else if cand.starts_with(&token) {
                self.input = format!("{prefix}{cand}");
            }
            self.refresh_shell();
            return;
        }
        if let Some(cmd) = self.effective_translation().map(|s| s.to_string()) {
            self.input = cmd;
            self.refresh_shell();
        }
    }

    fn cycle_up(&mut self) {
        if self.shell_cands.len() > 1 {
            self.shell_idx = self.shell_idx.saturating_sub(1);
            return;
        }
        if self.trans_cmds.len() > 1 {
            self.trans_idx = (self.trans_idx + self.trans_cmds.len() - 1) % self.trans_cmds.len();
        }
    }

    fn cycle_down(&mut self) {
        if self.shell_cands.len() > 1 {
            self.shell_idx = (self.shell_idx + 1) % self.shell_cands.len();
            return;
        }
        if self.trans_cmds.len() > 1 {
            self.trans_idx = (self.trans_idx + 1) % self.trans_cmds.len();
        }
    }

    fn apply_ai_translate(&mut self, gen: u64, resp: AiCommandResponse) {
        if gen != self.ai_gen {
            return;
        }
        self.trans_pending = false;
        if resp.command.trim().is_empty() {
            self.trans_cmds.clear();
            self.trans_expl.clear();
            return;
        }
        let mut cmds = vec![resp.command.clone()];
        for a in resp.alternates {
            let t = a.trim();
            if !t.is_empty() && !cmds.iter().any(|c| c == t) {
                cmds.push(t.to_string());
            }
        }
        self.trans_cmds = cmds;
        self.trans_idx = 0;
        self.trans_expl = resp.explanation;
    }

    fn apply_ai_err(&mut self, gen: u64, err: String) {
        if gen != self.ai_gen {
            return;
        }
        self.trans_pending = false;
        self.trans_cmds.clear();
        self.trans_expl = err;
    }

    fn apply_diag(&mut self, gen: u64, text: String) {
        if gen != self.ai_gen {
            return;
        }
        self.diag_pending = false;
        self.diagnostic = text;
    }

    fn apply_diag_err(&mut self, gen: u64, err: String) {
        if gen != self.ai_gen {
            return;
        }
        self.diag_pending = false;
        self.diagnostic = format!("error: could not fetch diagnostic\nnote: {err}");
    }
}

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
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, crossterm::cursor::Hide)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;

    let ai_live = Arc::new(AtomicU64::new(0));
    let app = Arc::new(Mutex::new(App::new(
        learn_mode,
        dry_run,
        Arc::clone(&ai_live),
    )));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<UiMsg>();

    let mut reader = crossterm::event::EventStream::new();

    schedule_ai_jobs(
        Arc::clone(&app),
        Arc::clone(&settings),
        Arc::clone(&client),
        Arc::clone(&ctx),
        Arc::clone(&ai_live),
        &tx,
        anyway,
        learn_mode,
    );

    loop {
        {
            let g = app.lock().unwrap();
            terminal.draw(|f| draw(f, &g))?;
        }

        tokio::select! {
            Some(msg) = rx.recv() => {
                let mut st = app.lock().unwrap();
                match msg {
                    UiMsg::Translate { gen, result } => match result {
                        Ok(r) => st.apply_ai_translate(gen, r),
                        Err(e) => st.apply_ai_err(gen, e),
                    },
                    UiMsg::Diagnostic { gen, text } => match text {
                        Ok(t) => st.apply_diag(gen, t),
                        Err(e) => st.apply_diag_err(gen, e),
                    },
                }
            }
            maybe_ev = reader.next() => {
                match maybe_ev {
                    Some(Ok(Event::Resize(_, _))) => {}
                    Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                        if handle_key(
                            Arc::clone(&app),
                            Arc::clone(&settings),
                            Arc::clone(&client),
                            Arc::clone(&ctx),
                            Arc::clone(&ai_live),
                            &tx,
                            key,
                            anyway,
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

fn schedule_ai_jobs(
    app: Arc<Mutex<App>>,
    settings: Arc<Settings>,
    client: Arc<AiClient>,
    ctx: Arc<ShellContext>,
    ai_live: Arc<AtomicU64>,
    tx: &tokio::sync::mpsc::UnboundedSender<UiMsg>,
    anyway: bool,
    learn_mode: bool,
) {
    let (line, gen) = {
        let mut st = app.lock().unwrap();
        let line = st.input.clone();
        let gen = st.bump_ai_gen();
        st.trans_pending = line.trim().len() >= 2;
        st.diag_pending = learn_mode;
        (line, gen)
    };

    let line_translate = line.clone();
    let line_diagnostic = line.clone();

    let my_gen = gen;
    let live = Arc::clone(&ai_live);
    let tx_t = tx.clone();
    let settings_t = Arc::clone(&settings);
    let client_t = Arc::clone(&client);
    let ctx_t = Arc::clone(&ctx);

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        if live.load(Ordering::SeqCst) != my_gen {
            return;
        }
        if line_translate.trim().len() < 2 {
            let _ = tx_t.send(UiMsg::Translate {
                gen: my_gen,
                result: Ok(AiCommandResponse {
                    command: String::new(),
                    explanation: String::new(),
                    missing_tools: vec![],
                    confidence: 0.0,
                    teaching: None,
                    alternates: vec![],
                }),
            });
            return;
        }
        let sys = prompt::translate_system_tui(&ctx_t, anyway);
        let model = client_t.model_name(&settings_t);
        let res = client_t
            .ask_command(&sys, &line_translate, &model, &settings_t, None)
            .await
            .map(|a| a.response)
            .map_err(|e| e.to_string());
        let _ = tx_t.send(UiMsg::Translate {
            gen: my_gen,
            result: res,
        });
    });

    if learn_mode {
        let tx_d = tx.clone();
        let settings_d = Arc::clone(&settings);
        let client_d = Arc::clone(&client);
        let ctx_d = Arc::clone(&ctx);
        let live_d = Arc::clone(&ai_live);

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(450)).await;
            if live_d.load(Ordering::SeqCst) != my_gen {
                return;
            }
            if line_diagnostic.trim().is_empty() {
                let _ = tx_d.send(UiMsg::Diagnostic {
                    gen: my_gen,
                    text: Ok("note: start typing a command or describe what you want to do".into()),
                });
                return;
            }
            let sys = prompt::tui_learn_diagnostic_system(&ctx_d);
            let model = client_d.model_name(&settings_d);
            let text = client_d
                .ask_freeform(&sys, &line_diagnostic, &model, &settings_d)
                .await
                .map(|(s, _)| s)
                .map_err(|e| e.to_string());
            let _ = tx_d.send(UiMsg::Diagnostic { gen: my_gen, text });
        });
    }
}

async fn handle_key(
    app: Arc<Mutex<App>>,
    settings: Arc<Settings>,
    client: Arc<AiClient>,
    ctx: Arc<ShellContext>,
    ai_live: Arc<AtomicU64>,
    tx: &tokio::sync::mpsc::UnboundedSender<UiMsg>,
    key: crossterm::event::KeyEvent,
    anyway: bool,
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
            schedule_ai_jobs(
                Arc::clone(&app),
                Arc::clone(&settings),
                Arc::clone(&client),
                Arc::clone(&ctx),
                Arc::clone(&ai_live),
                tx,
                anyway,
                learn_mode,
            );
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
            schedule_ai_jobs(
                Arc::clone(&app),
                Arc::clone(&settings),
                Arc::clone(&client),
                Arc::clone(&ctx),
                Arc::clone(&ai_live),
                tx,
                anyway,
                learn_mode,
            );
        }
        KeyCode::Char(c) => {
            let mut st = app.lock().unwrap();
            st.input.push(c);
            st.refresh_shell();
            drop(st);
            schedule_ai_jobs(
                Arc::clone(&app),
                Arc::clone(&settings),
                Arc::clone(&client),
                Arc::clone(&ctx),
                Arc::clone(&ai_live),
                tx,
                anyway,
                learn_mode,
            );
        }
        KeyCode::Backspace => {
            let mut st = app.lock().unwrap();
            st.input.pop();
            st.refresh_shell();
            drop(st);
            schedule_ai_jobs(
                Arc::clone(&app),
                Arc::clone(&settings),
                Arc::clone(&client),
                Arc::clone(&ctx),
                Arc::clone(&ai_live),
                tx,
                anyway,
                learn_mode,
            );
        }
        _ => {}
    }
    Ok(false)
}

fn draw(f: &mut Frame<'_>, app: &App) {
    let area = f.area();
    let bg = Color::Rgb(18, 18, 24);
    f.render_widget(Block::default().style(Style::default().bg(bg)), area);

    let main_h = if app.learn_mode {
        (area.height * 55 / 100)
            .max(16)
            .min(area.height.saturating_sub(2))
    } else {
        (area.height * 32 / 100)
            .max(11)
            .min(area.height.saturating_sub(2))
    };

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(main_h),
            Constraint::Min(0),
        ])
        .split(area);

    let mid = vert[1];
    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(mid);

    let box_area = horiz[1];

    let title = if app.learn_mode {
        " idoit — learn "
    } else {
        " idoit "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(Color::Rgb(28, 28, 36)));

    let inner = block.inner(box_area);
    f.render_widget(block, box_area);

    let ghost = app.shell_ghost().unwrap_or_default();
    let line1 = Line::from(vec![
        Span::styled("$ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            app.input.as_str(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(ghost, Style::default().fg(Color::Rgb(90, 90, 100))),
    ]);

    let trans_line = if !app.trans_cmds.is_empty() {
        let cur = app.effective_translation().unwrap_or("");
        let hint = if app.trans_pending { " …" } else { "" };
        let idx = if app.trans_cmds.len() > 1 {
            format!("   [{}/{}]", app.trans_idx + 1, app.trans_cmds.len())
        } else {
            String::new()
        };
        Line::from(vec![
            Span::styled("→ ", Style::default().fg(Color::Rgb(100, 160, 220))),
            Span::styled(
                format!("{cur}{hint}"),
                Style::default().fg(Color::Rgb(70, 130, 90)),
            ),
            Span::styled(idx, Style::default().fg(Color::DarkGray)),
        ])
    } else if app.trans_pending {
        Line::from(vec![Span::styled(
            "→ … translating",
            Style::default().fg(Color::DarkGray),
        )])
    } else if !app.trans_expl.is_empty() && app.trans_cmds.is_empty() {
        Line::from(vec![Span::styled(
            app.trans_expl.as_str(),
            Style::default().fg(Color::Rgb(200, 120, 120)),
        )])
    } else {
        Line::default()
    };

    let expl = if !app.trans_expl.is_empty() && !app.trans_cmds.is_empty() {
        Line::from(vec![Span::styled(
            app.trans_expl.as_str(),
            Style::default().fg(Color::Rgb(140, 140, 155)),
        )])
    } else {
        Line::default()
    };

    let help = Line::from(vec![Span::styled(
        "Tab: accept ghost / translation · ↑↓: cycle · Enter: run line · PgUp/PgDn: scroll preview · Esc: quit",
        Style::default().fg(Color::Rgb(75, 75, 88)),
    )]);

    let chunks = if app.learn_mode {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(4),
                Constraint::Length(1),
            ])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(4),
                Constraint::Length(1),
            ])
            .split(inner)
    };

    f.render_widget(Paragraph::new(line1).wrap(Wrap { trim: true }), chunks[0]);
    f.render_widget(
        Paragraph::new(trans_line).wrap(Wrap { trim: true }),
        chunks[1],
    );
    f.render_widget(Paragraph::new(expl).wrap(Wrap { trim: true }), chunks[2]);

    let out_txt = format!("{}\n{}", app.status_line, app.run_output);
    f.render_widget(
        Paragraph::new(out_txt.trim_end())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Rgb(200, 200, 175))),
        chunks[3],
    );

    if app.learn_mode {
        let diag_area = chunks[4];
        let diag_block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                " preview (rustc-style) ",
                Style::default().fg(Color::Yellow),
            ))
            .border_style(Style::default().fg(Color::Rgb(70, 70, 55)));
        let diag_inner = diag_block.inner(diag_area);
        f.render_widget(diag_block, diag_area);

        let diag_text = if app.diag_pending && app.diagnostic.is_empty() {
            "… analyzing input".to_string()
        } else {
            app.diagnostic.clone()
        };
        f.render_widget(
            Paragraph::new(diag_text)
                .wrap(Wrap { trim: true })
                .scroll((app.diag_scroll, 0))
                .style(Style::default().fg(Color::Rgb(220, 218, 200))),
            diag_inner,
        );
        f.render_widget(Paragraph::new(help).wrap(Wrap { trim: true }), chunks[5]);
    } else {
        f.render_widget(Paragraph::new(help).wrap(Wrap { trim: true }), chunks[4]);
    }
}
