use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::app::App;

pub fn draw(f: &mut Frame<'_>, app: &App, cursor_visible: bool) {
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
    let input_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let caret = if cursor_visible { "▍" } else { " " };
    let line1 = Line::from(vec![
        Span::styled("$ ", Style::default().fg(Color::DarkGray)),
        Span::styled(app.input.as_str(), input_style),
        Span::styled(caret, input_style),
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
        "Tab: accept ghost / translation · ↑↓: cycle (if alternates) · Enter: run line · PgUp/PgDn: scroll preview · Esc: quit",
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
