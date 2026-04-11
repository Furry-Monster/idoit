use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Select};

/// When `skip_select` is true, always returns index `0`.
pub fn pick_command_index(options: &[String], skip_select: bool) -> Result<usize> {
    if options.is_empty() {
        anyhow::bail!("no command candidate from model");
    }
    if options.len() == 1 || skip_select {
        return Ok(0);
    }

    let items: Vec<&str> = options.iter().map(String::as_str).collect();
    let selection = Select::new()
        .with_prompt("  Command (↑↓)")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(selection)
}

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

#[cfg(test)]
mod tests {
    use super::{confirm_execution, pick_command_index};

    #[test]
    fn confirm_execution_auto_yes_skips_prompt() {
        assert!(confirm_execution(true).unwrap());
    }

    #[test]
    fn pick_command_index_skip_avoids_multi_item() {
        let v = vec!["a".into(), "b".into()];
        assert_eq!(pick_command_index(&v, true).unwrap(), 0);
    }

    #[test]
    fn pick_command_index_empty_errors() {
        let v: Vec<String> = vec![];
        assert!(pick_command_index(&v, true).is_err());
    }
}
