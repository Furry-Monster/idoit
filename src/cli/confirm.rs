use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Select};

use crate::shell::command_safety::ExecSafetyReport;

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

/// One confirmation step with stricter defaults when the command is compound, has a heredoc, or matches risk heuristics.
pub fn confirm_shell_execution(
    auto_yes: bool,
    auto_confirm_setting: bool,
    cmd: &str,
) -> Result<bool> {
    let skip_prompts = auto_yes || auto_confirm_setting;
    let report = ExecSafetyReport::analyze(cmd);

    if skip_prompts {
        if report.has_heredoc {
            super::output::print_exec_safety_warning(
                "command contains a heredoc (<<); review before re-running with -y if needed",
            );
        }
        for reason in &report.high_risk_reasons {
            super::output::print_exec_safety_warning(reason);
        }
        if report.segments.len() > 1 {
            println!();
            println!(
                "  {} runs {} shell steps (&&, ||, ;, pipes, or newlines) — all execute together.",
                style("ℹ").dim(),
                style(report.segments.len()).cyan()
            );
            for (i, seg) in report.segments.iter().enumerate() {
                println!("      {}. {}", i + 1, style(seg).dim());
            }
            println!();
        }
        return Ok(true);
    }

    if report.has_heredoc {
        println!();
        super::output::print_exec_safety_warning("heredoc (<<) detected — review carefully.");
    }
    if !report.high_risk_reasons.is_empty() {
        println!();
        for reason in &report.high_risk_reasons {
            super::output::print_exec_safety_warning(reason);
        }
    }
    if report.segments.len() > 1 {
        println!();
        println!(
            "  {} This will run {} connected steps:",
            style("!").yellow(),
            report.segments.len()
        );
        for (i, seg) in report.segments.iter().enumerate() {
            println!("      {}. {}", i + 1, style(seg).dim());
        }
        println!();
    }

    if !report.needs_strict_default() {
        return confirm_execution(false);
    }

    Ok(Confirm::new()
        .with_prompt("  Execute? (see warnings above)")
        .default(false)
        .interact()?)
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
