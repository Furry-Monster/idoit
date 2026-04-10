mod ai;
mod aliases;
mod cache;
mod cli;
mod commands;
mod config;
mod session;
mod shell;
mod ui;

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

    // idoit --init <shell> or idoit init <shell>
    if let Some(ref shell_name) = cli.init {
        print!("{}", shell::init::generate(shell_name));
        return Ok(());
    }
    if let Some(shell_name) = cli.is_init_subcommand() {
        print!("{}", shell::init::generate(shell_name));
        return Ok(());
    }

    // idoit setup — interactive config wizard
    if cli.args.first().map(|s| s.as_str()) == Some("setup") {
        return commands::config_wizard::run();
    }

    // idoit --last — re-execute last generated command
    if cli.last {
        return run_last(&settings).await;
    }

    // idoit --save <name> <prompt...>
    if let Some(ref alias_name) = cli.save {
        if !cli.has_prompt() {
            ui::output::print_error("--save requires a prompt. Usage: idoit --save <name> <description...>");
            std::process::exit(1);
        }
        aliases::save(alias_name, &cli.prompt())?;
        println!(
            "  {} saved alias {} → \"{}\"",
            style("✓").green().bold(),
            style(alias_name).cyan().bold(),
            cli.prompt()
        );
        return Ok(());
    }

    // Check for prompt or actionable flags
    if !cli.has_prompt() && !cli.fix && !cli.refine {
        if cli.learn {
            ui::output::print_error("--learn requires a prompt or --fix");
            eprintln!();
            eprintln!("  Usage: idoit --learn <command or description>");
            eprintln!("         idoit --learn --fix");
            std::process::exit(1);
        }
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
        return commands::refine::run(
            &cli.prompt(),
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

    // Check for alias match
    let prompt = cli.prompt();
    let resolved_prompt = if let Some(description) = aliases::resolve(&prompt) {
        println!(
            "  {} {}",
            style("alias:").dim(),
            style(&description).dim().italic()
        );
        description
    } else {
        prompt
    };

    // idoit <prompt...>
    commands::translate::run(
        &resolved_prompt,
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
    println!(
        "  {} {}",
        style("$").dim(),
        style(&cmd).green().bold()
    );
    println!();

    let ctx = shell::context::ShellContext::detect(&settings.behavior.shell);

    if !ui::confirm::confirm_execution(settings.behavior.auto_confirm)? {
        return Ok(());
    }

    let result = shell::executor::execute(&ctx, &cmd)?;
    ui::output::print_execution_result(result.exit_code);
    Ok(())
}
