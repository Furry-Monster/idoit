use std::time::Duration;

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
    println!("{}", style("──────────────────").cyan().dim());
    println!();
}

pub fn print_explain(text: &str) {
    println!();
    for line in text.lines() {
        println!("  {}", line);
    }
    println!();
}

pub fn print_verbose_info(provider: &str, model: &str, elapsed: Duration) {
    println!(
        "  {} provider={}, model={}, time={:.1}s",
        style("ℹ").dim(),
        style(provider).dim(),
        style(model).dim(),
        elapsed.as_secs_f32()
    );
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

pub fn print_selected_alternate_command(cmd: &str) {
    println!(
        "  {} {}",
        style("selected:").dim(),
        style(cmd).green().bold()
    );
    println!();
}
