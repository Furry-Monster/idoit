mod ai;
mod cli;
mod commands;
mod config;
mod shell;
mod ui;

use anyhow::Result;
use clap::Parser;

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
    let settings = config::load()?;

    if cli.config {
        return config::show_config(&settings);
    }

    // No input and no actionable flag: print help
    if !cli.has_prompt() && !cli.fix {
        if cli.learn {
            ui::output::print_error("--learn requires a prompt or --fix");
            eprintln!();
            eprintln!("  Usage: idoit --learn <command or description>");
            eprintln!("         idoit --learn --fix");
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

    if cli.fix {
        return commands::fix::run(&settings, &client, &ctx, learn, cli.dry_run, cli.yes).await;
    }

    commands::translate::run(
        &cli.prompt(),
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
