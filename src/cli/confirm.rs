use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Select};

pub fn confirm_execution(auto_yes: bool) -> Result<bool> {
    if auto_yes {
        return Ok(true);
    }

    let confirmed = Confirm::new()
        .with_prompt("  Execute?")
        .default(true)
        .interact()?;

    Ok(confirmed)
}

pub fn confirm_anyway() -> Result<bool> {
    let confirmed = Confirm::new()
        .with_prompt("  This may install or modify things. Continue?")
        .default(false)
        .interact()?;

    Ok(confirmed)
}

#[allow(dead_code)]
pub enum CommandAction {
    Execute,
    Copy,
    Cancel,
}

#[allow(dead_code)]
pub fn confirm_with_copy(auto_yes: bool, command: &str) -> Result<CommandAction> {
    if auto_yes {
        return Ok(CommandAction::Execute);
    }

    let items = &["Execute", "Copy to clipboard", "Cancel"];
    let selection = Select::new()
        .with_prompt("  Action")
        .items(items)
        .default(0)
        .interact()?;

    match selection {
        0 => Ok(CommandAction::Execute),
        1 => {
            if super::clipboard::copy(command) {
                println!("  {} copied to clipboard", style("✓").green().bold());
            } else {
                println!(
                    "  {} clipboard not available (install xclip, xsel, or wl-copy)",
                    style("⚠").yellow()
                );
            }
            Ok(CommandAction::Copy)
        }
        _ => Ok(CommandAction::Cancel),
    }
}
