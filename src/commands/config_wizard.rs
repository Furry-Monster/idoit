use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Input, Password, Select};

use crate::config;
use crate::config::settings::Settings;
use crate::shell::history;

pub fn run() -> Result<()> {
    println!();
    println!(
        "  {} {}",
        style("idoit").bold().cyan(),
        style("— setup wizard").dim()
    );
    println!();

    let providers = &["openai", "anthropic", "gemini", "ollama"];
    let detected_shell = detect_shell();
    let selection = Select::new()
        .with_prompt("  Select AI provider")
        .items(providers)
        .default(0)
        .interact()?;

    let provider = providers[selection];

    let mut settings = config::load().unwrap_or_else(|_| Settings::default());
    settings.ai.provider = provider.to_string();

    match provider {
        "openai" => {
            let api_key_env = settings.ai.openai.api_key_env.clone();
            let model = settings.ai.openai.model.clone();
            let api_key = Password::new()
                .with_prompt("  OpenAI API key")
                .allow_empty_password(true)
                .interact()?;
            settings.ai.openai.api_key = api_key;
            println!(
                "  {} set {} in your shell environment",
                style("→").cyan(),
                style(&api_key_env).yellow()
            );
            println!(
                "  {} default model: {}",
                style("→").cyan(),
                style(&model).dim()
            );
        }
        "anthropic" => {
            let api_key_env = settings.ai.anthropic.api_key_env.clone();
            let model = settings.ai.anthropic.model.clone();
            let api_key = Password::new()
                .with_prompt("  Anthropic API key")
                .allow_empty_password(true)
                .interact()?;
            settings.ai.anthropic.api_key = api_key;
            println!(
                "  {} set {} in your shell environment",
                style("→").cyan(),
                style(&api_key_env).yellow()
            );
            println!(
                "  {} default model: {}",
                style("→").cyan(),
                style(&model).dim()
            );
        }
        "gemini" => {
            let api_key_env = settings.ai.gemini.api_key_env.clone();
            let model = settings.ai.gemini.model.clone();
            let api_key = Password::new()
                .with_prompt("  Gemini API key")
                .allow_empty_password(true)
                .interact()?;
            settings.ai.gemini.api_key = api_key;
            println!(
                "  {} set {} in your shell environment",
                style("→").cyan(),
                style(&api_key_env).yellow()
            );
            println!(
                "  {} default model: {}",
                style("→").cyan(),
                style(&model).dim()
            );
        }
        "ollama" => {
            let default_host = settings.ai.ollama.host.clone();
            let model = settings.ai.ollama.model.clone();
            let host: String = Input::new()
                .with_prompt("  Ollama host")
                .default(default_host)
                .interact_text()?;
            settings.ai.ollama.host = host;
            println!(
                "  {} using Ollama at {}",
                style("→").cyan(),
                style(&settings.ai.ollama.host).dim()
            );
            println!(
                "  {} default model: {}",
                style("→").cyan(),
                style(&model).dim()
            );
        }
        _ => {}
    }

    let shells = &["bash", "zsh", "fish", "sh"];
    let default_shell_idx = shells
        .iter()
        .position(|s| *s == detected_shell)
        .unwrap_or(0);
    let shell_idx = Select::new()
        .with_prompt("  Select current shell")
        .items(shells)
        .default(default_shell_idx)
        .interact()?;
    settings.behavior.shell = shells[shell_idx].to_string();
    settings.behavior.history_path = choose_history_path(shells[shell_idx])?;

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

fn detect_shell() -> String {
    std::env::var("SHELL")
        .unwrap_or_else(|_| "/bin/bash".to_string())
        .rsplit('/')
        .next()
        .unwrap_or("bash")
        .to_string()
}

fn choose_history_path(shell: &str) -> Result<String> {
    let default = history::default_history_path(shell)?;
    if default.exists() {
        println!(
            "  {} detected history: {}",
            style("→").cyan(),
            style(default.display()).dim()
        );
        return Ok(default.display().to_string());
    }

    println!(
        "  {} could not detect a valid {} history path",
        style("!").yellow(),
        shell
    );

    loop {
        let path: String = Input::new()
            .with_prompt("  Enter shell history path")
            .default(default.display().to_string())
            .interact_text()?;

        let p = std::path::Path::new(&path);
        if p.exists() {
            return Ok(path);
        }

        let use_anyway = Confirm::new()
            .with_prompt("  Path does not exist yet. Save anyway?")
            .default(false)
            .interact()?;
        if use_anyway {
            return Ok(path);
        }
    }
}
