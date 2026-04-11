//! Dispatches parsed [`crate::parser::Args`] to the appropriate command handler.

use std::sync::Arc;

use anyhow::Result;
use console::style;

use crate::ai::client::AiClient;
use crate::config;
use crate::parser::{Args, Commands};
use crate::shell::context::ShellContext;

use super::{
    config_cmd, explain, fix, init, last, macro_cmd, prompt_cmd, refine, run, setup, tui_cmd,
};

pub async fn run(args: Args) -> Result<()> {
    let g = args.global.clone();

    match &args.command {
        Some(Commands::Init { shell }) => {
            return init::run(shell);
        }
        Some(Commands::Setup) => {
            return setup::run();
        }
        Some(Commands::Config { cmd }) => {
            return config_cmd::run(cmd.as_ref());
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

    match args.command {
        None => {
            use clap::CommandFactory;
            Args::command().print_help()?;
            println!();
            Ok(())
        }
        Some(Commands::Init { .. }) | Some(Commands::Setup) | Some(Commands::Config { .. }) => {
            unreachable!("handled above")
        }
        Some(Commands::Last) => last::run(&settings, g.yes).await,
        Some(Commands::Macro { name, body }) => macro_cmd::run(&name, &body),
        Some(Commands::Tui { learn }) => {
            let learn_mode = g.learn || learn;
            let settings = Arc::new(settings);
            let client = Arc::new(AiClient::from_settings(&settings, g.provider)?);
            let ctx = Arc::new(ShellContext::detect(&settings.behavior.shell));
            tui_cmd::run(settings, client, ctx, learn_mode, g.anyway, g.dry_run).await
        }
        Some(Commands::Fix) => {
            let client = AiClient::from_settings(&settings, g.provider)?;
            let ctx = ShellContext::detect(&settings.behavior.shell);
            let learn = g.learn || settings.behavior.learn_by_default;
            fix::run(&settings, &client, &ctx, learn, g.dry_run, g.yes).await
        }
        Some(Commands::Explain { command }) => {
            let client = AiClient::from_settings(&settings, g.provider)?;
            let ctx = ShellContext::detect(&settings.behavior.shell);
            explain::run_cli(&command, &settings, &client, &ctx).await
        }
        Some(Commands::Refine { text }) => {
            let client = AiClient::from_settings(&settings, g.provider)?;
            let ctx = ShellContext::detect(&settings.behavior.shell);
            refine::run_cli(&text, &settings, &client, &ctx, g.dry_run, g.yes).await
        }
        Some(Commands::Run { prompt }) => run::run(&g, &settings, &prompt).await,
        Some(Commands::Prompt(parts)) => prompt_cmd::run(&g, &settings, &parts).await,
    }
}
