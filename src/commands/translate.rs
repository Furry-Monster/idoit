use anyhow::{bail, Result};

use crate::ai::client::AiClient;
use crate::ai::prompt;
use crate::config::settings::Settings;
use crate::shell::context::ShellContext;
use crate::shell::executor;
use crate::ui::{confirm, output, spinner};

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

    let model = client.model_name(settings);
    let spin = spinner::Spinner::new("thinking...");
    let resp = client.ask_command(&system, user_input, &model).await;
    spin.finish();

    let resp = resp?;

    if !resp.missing_tools.is_empty() && !anyway {
        output::print_command(&resp);
        bail!(
            "required tools not found: {}. Use --anyway to proceed regardless.",
            resp.missing_tools.join(", ")
        );
    }

    output::print_command(&resp);

    if let Some(ref teaching) = resp.teaching {
        if !teaching.is_empty() {
            output::print_teaching(teaching);
        }
    }

    if dry_run {
        output::print_dry_run_notice();
        return Ok(());
    }

    let should_confirm = if anyway && !resp.missing_tools.is_empty() {
        confirm::confirm_anyway()?
    } else {
        confirm::confirm_execution(auto_yes || settings.behavior.auto_confirm)?
    };

    if !should_confirm {
        return Ok(());
    }

    let result = executor::execute(ctx, &resp.command)?;
    output::print_execution_result(result.exit_code);

    Ok(())
}
