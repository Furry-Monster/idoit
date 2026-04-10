use anyhow::Result;
use console::style;
use dialoguer::Select;

use crate::config;
use crate::config::settings::Settings;

pub fn run() -> Result<()> {
    println!();
    println!(
        "  {} {}",
        style("idoit").bold().cyan(),
        style("— setup wizard").dim()
    );
    println!();

    let providers = &["openai", "anthropic", "gemini", "ollama"];
    let selection = Select::new()
        .with_prompt("  Select AI provider")
        .items(providers)
        .default(0)
        .interact()?;

    let provider = providers[selection];

    let mut settings = Settings::default();
    settings.ai.provider = provider.to_string();

    match provider {
        "openai" => {
            let cfg = &settings.ai.openai;
            println!(
                "  {} set {} in your shell environment",
                style("→").cyan(),
                style(&cfg.api_key_env).yellow()
            );
            println!("  {} default model: {}", style("→").cyan(), style(&cfg.model).dim());
        }
        "anthropic" => {
            let cfg = &settings.ai.anthropic;
            println!(
                "  {} set {} in your shell environment",
                style("→").cyan(),
                style(&cfg.api_key_env).yellow()
            );
            println!("  {} default model: {}", style("→").cyan(), style(&cfg.model).dim());
        }
        "gemini" => {
            let cfg = &settings.ai.gemini;
            println!(
                "  {} set {} in your shell environment",
                style("→").cyan(),
                style(&cfg.api_key_env).yellow()
            );
            println!("  {} default model: {}", style("→").cyan(), style(&cfg.model).dim());
        }
        "ollama" => {
            let cfg = &settings.ai.ollama;
            println!(
                "  {} using Ollama at {}",
                style("→").cyan(),
                style(&cfg.host).dim()
            );
            println!("  {} default model: {}", style("→").cyan(), style(&cfg.model).dim());
        }
        _ => {}
    }

    config::save(&settings)?;

    println!();
    println!(
        "  {} config saved to {}",
        style("✓").green().bold(),
        style(config::config_path().display()).dim()
    );
    println!();

    Ok(())
}
