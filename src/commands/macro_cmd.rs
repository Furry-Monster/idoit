//! `idoit macro NAME …` — save a prompt macro as `@NAME`.

use anyhow::Result;
use console::style;

use crate::macros;
use crate::parser::Args;

pub fn run(name: &str, body: &[String]) -> Result<()> {
    let text = Args::join_prompt(body);
    if text.is_empty() {
        anyhow::bail!("macro needs a body. Usage: idoit macro NAME words describing the task…");
    }
    macros::save(name, &text)?;
    println!(
        "  {} saved macro @{} → \"{}\"",
        style("✓").green().bold(),
        style(name).cyan().bold(),
        text
    );
    Ok(())
}
