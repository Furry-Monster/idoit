//! `idoit last` — re-run the last idoit-generated command from session history.

use anyhow::Result;
use console::style;

use crate::cli;
use crate::config::settings::Settings;
use crate::session;
use crate::shell::context::ShellContext;
use crate::shell::executor;

pub async fn run(settings: &Settings, auto_yes: bool) -> Result<()> {
    let cmd = session::last_command_string()
        .ok_or_else(|| anyhow::anyhow!("no previous idoit command found in session history"))?;

    println!();
    println!("  {} {}", style("$").dim(), style(&cmd).green().bold());
    println!();

    let ctx = ShellContext::detect(&settings.behavior.shell);

    if !cli::confirm::confirm_shell_execution(auto_yes, settings.behavior.auto_confirm, &cmd)? {
        return Ok(());
    }

    let result = executor::execute(&ctx, &cmd)?;
    cli::output::print_execution_result(result.exit_code);
    Ok(())
}
