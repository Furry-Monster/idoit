mod ai;
mod macros;
mod cli;
mod commands;
mod config;
mod session;
mod shell;
mod tui;
mod ui;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use console::style;

use cli::Cli;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        ui::output::print_error(&format!("{e:#}"));
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    // idoit --init <shell> or idoit init <shell>
    if let Some(ref shell_name) = cli.init {
        print!("{}", shell::init::generate(shell_name));
        return Ok(());
    }
    if let Some(shell_name) = cli.is_init_subcommand() {
        print!("{}", shell::init::generate(shell_name));
        return Ok(());
    }

    // idoit setup — interactive configuration
    if cli.args.first().map(|s| s.as_str()) == Some("setup") {
        return commands::setup::run();
    }

    // First launch: no config yet, run setup
    if !config::exists() {
        println!(
            "  {} first launch detected. Running setup...",
            style("→").cyan()
        );
        commands::setup::run()?;
    }

    let settings = config::load()?;

    // Wire ui.color — respect config and NO_COLOR standard
    let no_color = std::env::var("NO_COLOR").is_ok();
    if !settings.ui.color || no_color {
        console::set_colors_enabled(false);
        console::set_colors_enabled_stderr(false);
    }

    // idoit --config
    if cli.config {
        return config::show_config(&settings);
    }

    // idoit --last — re-execute last generated command
    if cli.last {
        return run_last(&settings).await;
    }

    // idoit --macro <name> <text...>
    if let Some(ref name) = cli.macro_name {
        if !cli.has_prompt() {
            ui::output::print_error(
                "--macro requires macro body. Usage: idoit --macro <name> <text...>  (then use @name in prompts)",
            );
            std::process::exit(1);
        }
        macros::save(name, &cli.prompt())?;
        println!(
            "  {} saved macro @{} → \"{}\"",
            style("✓").green().bold(),
            style(name).cyan().bold(),
            cli.prompt()
        );
        return Ok(());
    }

    // Full-screen TUI: `idoit --tui` (optional `-l` / `--learn` inside tui::run)
    let use_tui = cli.tui
        && !cli.fix
        && !cli.refine
        && !cli.explain
        && !cli.last
        && cli.macro_name.is_none();

    if cli.tui && cli.has_prompt() {
        ui::output::print_error(
            "--tui runs the full-screen UI and cannot be combined with a prompt. \
             Use: idoit --tui   or   idoit --tui --learn",
        );
        std::process::exit(1);
    }

    if use_tui {
        let settings = Arc::new(settings);
        let client = Arc::new(ai::client::AiClient::from_settings(
            &settings,
            cli.provider.as_deref(),
        )?);
        let ctx = Arc::new(shell::context::ShellContext::detect(
            &settings.behavior.shell,
        ));
        return tui::run(settings, client, ctx, cli.learn, cli.anyway, cli.dry_run).await;
    }

    // Check for prompt or actionable flags
    if !cli.has_prompt() && !cli.fix && !cli.refine {
        if cli.explain {
            ui::output::print_error("--explain requires a command to explain");
            eprintln!();
            eprintln!("  Usage: idoit --explain 'find . -name \"*.rs\"'");
            std::process::exit(1);
        }

        use clap::CommandFactory;
        Cli::command().print_help()?;
        println!();
        return Ok(());
    }

    let client = ai::client::AiClient::from_settings(&settings, cli.provider.as_deref())?;
    let ctx = shell::context::ShellContext::detect(&settings.behavior.shell);
    let learn = cli.learn || settings.behavior.learn_by_default;

    // idoit --fix
    if cli.fix {
        return commands::fix::run(&settings, &client, &ctx, learn, cli.dry_run, cli.yes).await;
    }

    // idoit --refine <refinement>
    if cli.refine {
        let refinement = macros::expand(&cli.prompt()).text;
        return commands::refine::run(
            &refinement,
            &settings,
            &client,
            &ctx,
            cli.dry_run,
            cli.yes,
        )
        .await;
    }

    // idoit --explain <command>
    if cli.explain {
        return commands::explain::run(&cli.prompt(), &settings, &client, &ctx).await;
    }

    let prompt = cli.prompt();
    let expanded = macros::expand(&prompt);
    if !expanded.used.is_empty() {
        println!(
            "  {} {}",
            style("macro:").dim(),
            style(&expanded.text).dim().italic()
        );
    }

    // idoit <prompt...>
    commands::translate::run(
        &expanded.text,
        &settings,
        &client,
        &ctx,
        cli.anyway,
        learn,
        cli.dry_run,
        cli.yes,
    )
    .await
}

async fn run_last(settings: &config::settings::Settings) -> Result<()> {
    let cmd = session::last_command_string()
        .ok_or_else(|| anyhow::anyhow!("no previous idoit command found in session history"))?;

    println!();
    println!("  {} {}", style("$").dim(), style(&cmd).green().bold());
    println!();

    let ctx = shell::context::ShellContext::detect(&settings.behavior.shell);

    if !ui::confirm::confirm_execution(settings.behavior.auto_confirm)? {
        return Ok(());
    }

    let result = shell::executor::execute(&ctx, &cmd)?;
    ui::output::print_execution_result(result.exit_code);
    Ok(())
}
