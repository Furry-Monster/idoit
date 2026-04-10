use console::style;

use crate::ai::types::AiCommandResponse;

pub fn print_command(resp: &AiCommandResponse) {
    println!();
    println!(
        "  {} {}",
        style("$").dim(),
        style(&resp.command).green().bold()
    );
    println!("  {} {}", style("→").cyan(), style(&resp.explanation).dim());

    if !resp.missing_tools.is_empty() {
        println!();
        println!(
            "  {} missing tools: {}",
            style("⚠").yellow(),
            style(resp.missing_tools.join(", ")).yellow()
        );
    }

    println!();
}

pub fn print_teaching(teaching: &str) {
    let bar = style("─").cyan().dim();
    let label = style(" learn ").cyan();
    println!("{bar}{bar}{bar}{label}{bar}{bar}{bar}");
    println!();
    for line in teaching.lines() {
        println!("  {}", style(line).dim());
    }
    println!();
    println!(
        "{}",
        style("──────────────────").cyan().dim()
    );
    println!();
}

pub fn print_execution_result(exit_code: i32) {
    if exit_code == 0 {
        println!("\n  {} {}", style("✓").green().bold(), style("done").dim());
    } else {
        println!(
            "\n  {} {}",
            style("✗").red().bold(),
            style(format!("exited with code {exit_code}")).dim()
        );
    }
}

pub fn print_dry_run_notice() {
    println!(
        "  {}",
        style("(dry run — command not executed)").dim().italic()
    );
}

pub fn print_error(msg: &str) {
    eprintln!("{} {msg}", style("error:").red().bold());
}

pub fn print_fix_context(last_command: &str) {
    println!();
    println!(
        "  {} {}",
        style("last command:").dim(),
        style(last_command).yellow()
    );
}

#[allow(dead_code)]
pub fn print_banner() {
    println!(
        "  {} {}",
        style("idoit").bold().cyan(),
        style("— just do it!").dim()
    );
}
