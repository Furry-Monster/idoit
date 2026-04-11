use anyhow::Result;

use crate::ai::client::AiClient;
use crate::ai::prompt;
use crate::cli::{output, spinner};
use crate::config::settings::Settings;
use crate::parser::Args;
use crate::shell::context::ShellContext;

/// `idoit explain …` from argv tail.
pub async fn run_cli(
    command: &[String],
    settings: &Settings,
    client: &AiClient,
    ctx: &ShellContext,
) -> Result<()> {
    let cmd_line = Args::join_prompt(command);
    if cmd_line.is_empty() {
        anyhow::bail!(
            "explain needs a command. Example: idoit explain 'find . -name \"*.rs\"'\n\n  Usage: idoit explain <shell command…>"
        );
    }
    run(&cmd_line, settings, client, ctx).await
}

pub async fn run(
    command_to_explain: &str,
    settings: &Settings,
    client: &AiClient,
    ctx: &ShellContext,
) -> Result<()> {
    let system = prompt::explain_system(ctx);
    let model = client.model_name(settings);

    let spin = spinner::Spinner::new("analyzing...");
    let (content, elapsed) = client
        .ask_freeform(&system, command_to_explain, &model, settings)
        .await?;
    spin.finish();

    if settings.ui.verbose {
        output::print_verbose_info(client.provider_name(), &model, elapsed);
    }

    output::print_explain(&content);

    Ok(())
}
