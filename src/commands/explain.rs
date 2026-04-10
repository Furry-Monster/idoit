use anyhow::Result;

use crate::ai::client::AiClient;
use crate::ai::prompt;
use crate::config::settings::Settings;
use crate::shell::context::ShellContext;
use crate::ui::{output, spinner};

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
