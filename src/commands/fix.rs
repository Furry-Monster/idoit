use anyhow::Result;

use crate::ai::client::AiClient;
use crate::ai::prompt;
use crate::config::settings::Settings;
use crate::shell::context::ShellContext;
use crate::shell::{executor, history};
use crate::ui::{confirm, output, spinner};

pub async fn run(
    settings: &Settings,
    client: &AiClient,
    ctx: &ShellContext,
    learn: bool,
    dry_run: bool,
    auto_yes: bool,
) -> Result<()> {
    let entry = history::last_command(ctx)?;
    output::print_fix_context(&entry.command);

    let error_output = history::recent_error_output().unwrap_or_default();

    let mut system = prompt::fix_system(ctx);
    if learn || settings.behavior.learn_by_default {
        system.push_str(prompt::learn_suffix());
    }

    let user_msg = prompt::fix_user_message(&entry.command, &error_output);
    let model = client.model_name(settings);

    let spin = spinner::Spinner::new("diagnosing...");
    let resp = client.ask_command(&system, &user_msg, &model).await;
    spin.finish();

    let resp = resp?;

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

    if !confirm::confirm_execution(auto_yes || settings.behavior.auto_confirm)? {
        return Ok(());
    }

    let result = executor::execute(ctx, &resp.command)?;
    output::print_execution_result(result.exit_code);

    Ok(())
}
