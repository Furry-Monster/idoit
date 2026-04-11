use anyhow::{bail, Result};
use console::style;

use crate::ai::client::AiClient;
use crate::ai::prompt;
use crate::cli::{candidates, confirm, output, spinner};
use crate::config::settings::Settings;
use crate::macros;
use crate::parser::GlobalOpts;
use crate::session;
use crate::shell::context::ShellContext;
use crate::shell::executor;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    user_input: &str,
    settings: &Settings,
    client: &AiClient,
    ctx: &ShellContext,
    anyway: bool,
    learn: bool,
    dry_run: bool,
    auto_yes: bool,
) -> Result<()> {
    let mut system = prompt::translate_system(ctx, anyway);
    if learn || settings.behavior.learn_by_default {
        system.push_str(prompt::learn_suffix());
    }

    let layered = session::context::LayeredContext::gather(ctx, settings, None);
    let user_with_ctx = prompt::with_shell_context(user_input, &layered.format_block());

    let model = client.model_name(settings);
    let spin = spinner::Spinner::new("thinking...");
    let result = client
        .ask_command(&system, &user_with_ctx, &model, settings, Some(&spin))
        .await;
    spin.finish();

    let result = result?;
    let resp = &result.response;

    if settings.ui.verbose {
        output::print_verbose_info(client.provider_name(), &model, result.elapsed);
    }

    if !resp.missing_tools.is_empty() && !anyway {
        output::print_command(resp);
        bail!(
            "required tools not found: {}. Use --anyway to proceed regardless.",
            resp.missing_tools.join(", ")
        );
    }

    output::print_command(resp);

    if let Some(ref teaching) = resp.teaching {
        if !teaching.is_empty() {
            output::print_teaching(teaching);
        }
    }

    let cmd_options = candidates::ordered_command_options(&resp.command, &resp.alternates);
    let skip_select = dry_run || auto_yes || settings.behavior.auto_confirm;
    let idx = confirm::pick_command_index(&cmd_options, skip_select)?;
    let chosen = cmd_options[idx].clone();
    if cmd_options.len() > 1 && idx != 0 {
        output::print_selected_alternate_command(&chosen);
    }

    if dry_run {
        output::print_dry_run_notice();
        session::record(user_input, &chosen, false, None);
        return Ok(());
    }

    if anyway && !resp.missing_tools.is_empty() && !confirm::confirm_anyway()? {
        session::record(user_input, &chosen, false, None);
        return Ok(());
    }

    if !confirm::confirm_shell_execution(
        auto_yes,
        settings.behavior.auto_confirm,
        &chosen,
    )? {
        session::record(user_input, &chosen, false, None);
        return Ok(());
    }

    let exec_result = executor::execute(ctx, &chosen)?;
    output::print_execution_result(exec_result.exit_code);
    session::record(user_input, &chosen, true, Some(exec_result.exit_code));

    Ok(())
}

/// Wired from `idoit run`, bare prompt, etc.: expand macros, build client/context, call [`run`].
pub async fn run_from_cli(g: &GlobalOpts, settings: &Settings, prompt: &str) -> Result<()> {
    let client = AiClient::from_settings(settings, g.provider)?;
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
    run(
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
