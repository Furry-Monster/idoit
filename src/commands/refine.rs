use anyhow::Result;

use crate::ai::client::AiClient;
use crate::ai::prompt;
use crate::cli::{confirm, output, spinner};
use crate::config::settings::Settings;
use crate::macros;
use crate::parser::Args;
use crate::session;
use crate::shell::context::ShellContext;
use crate::shell::executor;

/// `idoit refine …` from argv tail (macro expansion applied).
pub async fn run_cli(
    text: &[String],
    settings: &Settings,
    client: &AiClient,
    ctx: &ShellContext,
    dry_run: bool,
    auto_yes: bool,
) -> Result<()> {
    let refinement = Args::join_prompt(text);
    if refinement.is_empty() {
        anyhow::bail!("refine needs text. Example: idoit refine \"only under src\"");
    }
    let refinement = macros::expand(&refinement).text;
    run(
        &refinement,
        settings,
        client,
        ctx,
        dry_run,
        auto_yes,
    )
    .await
}

pub async fn run(
    refinement: &str,
    settings: &Settings,
    client: &AiClient,
    ctx: &ShellContext,
    dry_run: bool,
    auto_yes: bool,
) -> Result<()> {
    let last = session::last_entry()
        .ok_or_else(|| anyhow::anyhow!("no previous idoit command to refine. Run idoit first."))?;

    let system = prompt::refine_system(ctx);
    let layered = session::context::LayeredContext::gather(ctx, settings, None);
    let user_msg = prompt::refine_user_message(&last.input, &last.command, refinement);
    let user_msg = prompt::with_shell_context(&user_msg, &layered.format_block());
    let model = client.model_name(settings);

    let spin = spinner::Spinner::new("refining...");
    let result = client
        .ask_command(&system, &user_msg, &model, settings, Some(&spin))
        .await;
    spin.finish();

    let result = result?;
    let resp = &result.response;

    if settings.ui.verbose {
        output::print_verbose_info(client.provider_name(), &model, result.elapsed);
    }

    output::print_command(resp);

    if dry_run {
        output::print_dry_run_notice();
        session::record(refinement, &resp.command, false, None);
        return Ok(());
    }

    if !confirm::confirm_shell_execution(
        auto_yes,
        settings.behavior.auto_confirm,
        &resp.command,
    )? {
        session::record(refinement, &resp.command, false, None);
        return Ok(());
    }

    let exec_result = executor::execute(ctx, &resp.command)?;
    output::print_execution_result(exec_result.exit_code);
    session::record(refinement, &resp.command, true, Some(exec_result.exit_code));

    Ok(())
}
