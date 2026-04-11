use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Input, Select};

use crate::config;
use crate::config::settings::{AiProviderId, Settings};
use crate::shell::{history, rc};

pub fn run() -> Result<()> {
    println!();
    println!(
        "  {} {}",
        style("idoit").bold().cyan(),
        style("— setup").dim()
    );
    println!();

    let provider_labels = &["openai", "anthropic", "gemini", "deepseek", "ollama"];
    let provider_ids = [
        AiProviderId::OpenAi,
        AiProviderId::Anthropic,
        AiProviderId::Gemini,
        AiProviderId::DeepSeek,
        AiProviderId::Ollama,
    ];
    let detected_shell = detect_shell();
    let selection = Select::new()
        .with_prompt("  Select AI provider")
        .items(provider_labels)
        .default(0)
        .interact()?;

    let provider = provider_ids[selection];

    let mut settings = config::load().unwrap_or_else(|_| Settings::default());
    settings.ai.provider = provider;

    match provider {
        AiProviderId::OpenAi => {
            let api_key_env = settings.ai.openai.api_key_env.clone();
            let model = settings.ai.openai.model.clone();
            let api_key: String = Input::new()
                .with_prompt("  OpenAI API key")
                .allow_empty(true)
                .interact_text()?;
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
        AiProviderId::Anthropic => {
            let api_key_env = settings.ai.anthropic.api_key_env.clone();
            let model = settings.ai.anthropic.model.clone();
            let api_key: String = Input::new()
                .with_prompt("  Anthropic API key")
                .allow_empty(true)
                .interact_text()?;
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
        AiProviderId::Gemini => {
            let api_key_env = settings.ai.gemini.api_key_env.clone();
            let model = settings.ai.gemini.model.clone();
            let api_key: String = Input::new()
                .with_prompt("  Gemini API key")
                .allow_empty(true)
                .interact_text()?;
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
        AiProviderId::DeepSeek => {
            let api_key_env = settings.ai.deepseek.api_key_env.clone();
            let model = settings.ai.deepseek.model.clone();
            let api_key: String = Input::new()
                .with_prompt("  DeepSeek API key")
                .allow_empty(true)
                .interact_text()?;
            settings.ai.deepseek.api_key = api_key;
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
        AiProviderId::Ollama => {
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

    let shell_name = shells[shell_idx];
    if let Some(rc_path) = rc::rc_path(shell_name) {
        let prompt = format!("  Add idoit shell hooks to {}?", rc_path.display());
        if Confirm::new()
            .with_prompt(prompt)
            .default(true)
            .interact()?
        {
            match rc::apply(shell_name) {
                Ok(p) => {
                    println!(
                        "  {} updated {}",
                        style("✓").green().bold(),
                        style(p.display()).dim()
                    );
                    println!(
                        "  {} open a new terminal or run: {}",
                        style("→").cyan(),
                        style(format!("source {}", p.display())).yellow()
                    );
                }
                Err(e) => {
                    eprintln!("  {} could not update rc file: {}", style("!").yellow(), e);
                }
            }
        }
    }

    println!();

    Ok(())
}

fn shell_basename_from_path(path: &str) -> String {
    path.rsplit('/').next().unwrap_or("bash").to_string()
}

fn detect_shell() -> String {
    shell_basename_from_path(&std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()))
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

#[cfg(test)]
mod tests {
    use super::shell_basename_from_path;

    #[test]
    fn shell_basename_from_login_paths() {
        assert_eq!(shell_basename_from_path("/usr/bin/zsh"), "zsh");
        assert_eq!(shell_basename_from_path("/bin/bash"), "bash");
    }

    #[test]
    fn shell_basename_without_slash_is_whole_string() {
        assert_eq!(shell_basename_from_path("fish"), "fish");
    }

    #[test]
    fn shell_basename_empty_path() {
        assert_eq!(shell_basename_from_path(""), "");
    }
}
