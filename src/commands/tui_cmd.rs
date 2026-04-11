//! `idoit tui` — full-screen interactive UI entrypoint.

use std::sync::Arc;

use anyhow::Result;

use crate::ai::client::AiClient;
use crate::config::settings::Settings;
use crate::shell::context::ShellContext;

pub async fn run(
    settings: Arc<Settings>,
    client: Arc<AiClient>,
    ctx: Arc<ShellContext>,
    learn_mode: bool,
    anyway: bool,
    dry_run: bool,
) -> Result<()> {
    crate::tui::run(settings, client, ctx, learn_mode, anyway, dry_run).await
}
