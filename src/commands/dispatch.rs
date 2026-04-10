//! Dispatches parsed [`crate::cli::Cli`] to the appropriate command handler.

use std::sync::Arc;

use anyhow::Result;
use console::style;

use crate::ai::client::AiClient;
use crate::cli::{Cli, Commands, GlobalOpts};
use crate::config;
use crate::config::settings::Settings;
use crate::macros;
use crate::session;
use crate::shell::context::ShellContext;
use crate::tui;
use crate::ui;

use super::{explain, fix, refine, setup, translate};

/// Re-run the last generated command from session history (subcommand: `last`).
pub async fn run_last(settings: &Settings) -> Result<()> {
    let cmd = session::last_command_string()
        .ok_or_else(|| anyhow::anyhow!("no previous idoit command found in session history"))?;

    println!();
    println!("  {} {}", style("$").dim(), style(&cmd).green().bold());
    println!();

    let ctx = ShellContext::detect(&settings.behavior.shell);

    if !ui::confirm::confirm_execution(settings.behavior.auto_confirm)? {
        return Ok(());
    }

    let result = crate::shell::executor::execute(&ctx, &cmd)?;
    ui::output::print_execution_result(result.exit_code);
    Ok(())
}

pub async fn run(cli: Cli) -> Result<()> {
    let g = cli.global.clone();

    match &cli.command {
        Some(Commands::Init { shell }) => {
            print!("{}", crate::shell::init::generate(shell));
            return Ok(());
        }
        Some(Commands::Setup) => {
            return setup::run();
        }
        _ => {}
    }

    if !config::exists() {
        println!(
            "  {} first launch detected. Running setup...",
            style("→").cyan()
        );
        setup::run()?;
    }

    let settings = config::load()?;

    let no_color = std::env::var("NO_COLOR").is_ok();
    if !settings.ui.color || no_color {
        console::set_colors_enabled(false);
        console::set_colors_enabled_stderr(false);
    }

    match cli.command {
        None => {
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
        Some(Commands::Init { .. }) | Some(Commands::Setup) => unreachable!("handled above"),
        Some(Commands::Config) => config::show_config(&settings),
        Some(Commands::Last) => run_last(&settings).await,
        Some(Commands::Macro { name, body }) => {
            let text = Cli::join_prompt(&body);
            if text.is_empty() {
                ui::output::print_error(
                    "macro needs a body. Usage: idoit macro NAME words describing the task…",
                );
                std::process::exit(1);
            }
            macros::save(&name, &text)?;
            println!(
                "  {} saved macro @{} → \"{}\"",
                style("✓").green().bold(),
                style(&name).cyan().bold(),
                text
            );
            Ok(())
        }
        Some(Commands::Tui { learn }) => {
            let learn_mode = g.learn || learn;
            let settings = Arc::new(settings);
            let client = Arc::new(AiClient::from_settings(&settings, g.provider.as_deref())?);
            let ctx = Arc::new(ShellContext::detect(&settings.behavior.shell));
            tui::run(settings, client, ctx, learn_mode, g.anyway, g.dry_run).await
        }
        Some(Commands::Fix) => {
            let client = AiClient::from_settings(&settings, g.provider.as_deref())?;
            let ctx = ShellContext::detect(&settings.behavior.shell);
            let learn = g.learn || settings.behavior.learn_by_default;
            fix::run(&settings, &client, &ctx, learn, g.dry_run, g.yes).await
        }
        Some(Commands::Explain { command }) => {
            let cmd_line = Cli::join_prompt(&command);
            if cmd_line.is_empty() {
                ui::output::print_error(
                    "explain needs a command. Example: idoit explain 'find . -name \"*.rs\"'",
                );
                eprintln!();
                eprintln!("  Usage: idoit explain <shell command…>");
                std::process::exit(1);
            }
            let client = AiClient::from_settings(&settings, g.provider.as_deref())?;
            let ctx = ShellContext::detect(&settings.behavior.shell);
            explain::run(&cmd_line, &settings, &client, &ctx).await
        }
        Some(Commands::Refine { text }) => {
            let refinement = Cli::join_prompt(&text);
            if refinement.is_empty() {
                ui::output::print_error(
                    "refine needs text. Example: idoit refine \"only under src\"",
                );
                std::process::exit(1);
            }
            let refinement = macros::expand(&refinement).text;
            let client = AiClient::from_settings(&settings, g.provider.as_deref())?;
            let ctx = ShellContext::detect(&settings.behavior.shell);
            refine::run(&refinement, &settings, &client, &ctx, g.dry_run, g.yes).await
        }
        Some(Commands::Run { prompt }) => {
            let prompt = Cli::join_prompt(&prompt);
            if prompt.is_empty() {
                ui::output::print_error(
                    "run needs a prompt. Example: idoit run find all TODO in src",
                );
                std::process::exit(1);
            }
            run_translate(&g, &settings, &prompt).await
        }
        Some(Commands::Prompt(parts)) => {
            let prompt = Cli::join_prompt_os(&parts);
            if prompt.is_empty() {
                use clap::CommandFactory;
                Cli::command().print_help()?;
                println!();
                return Ok(());
            }
            run_translate(&g, &settings, &prompt).await
        }
    }
}

async fn run_translate(g: &GlobalOpts, settings: &Settings, prompt: &str) -> Result<()> {
    let client = AiClient::from_settings(settings, g.provider.as_deref())?;
    let ctx = ShellContext::detect(&settings.behavior.shell);
    let learn = g.learn || settings.behavior.learn_by_default;
    let expanded = macros::expand(prompt);
    if !expanded.used.is_empty() {
        println!(
            "  {} {}",
            style("macro:").dim(),
            style(&expanded.text).dim().italic()
        );
    }
    translate::run(
        &expanded.text,
        settings,
        &client,
        &ctx,
        g.anyway,
        learn,
        g.dry_run,
        g.yes,
    )
    .await
}
