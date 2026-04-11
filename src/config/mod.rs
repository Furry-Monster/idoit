//! Config directory (`~/.config/idoit`), `config.toml` load/save, and [`settings`] types.

pub mod settings;

use std::path::PathBuf;

use anyhow::{Context, Result};

use self::settings::Settings;

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("idoit")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn exists() -> bool {
    config_path().exists()
}

pub fn load() -> Result<Settings> {
    let path = config_path();
    if !path.exists() {
        return Ok(Settings::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;
    let settings: Settings =
        toml::from_str(&content).with_context(|| "failed to parse config.toml")?;
    Ok(settings)
}

#[allow(dead_code)]
pub fn save(settings: &Settings) -> Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create config directory {}", dir.display()))?;
    let content =
        toml::to_string_pretty(settings).with_context(|| "failed to serialize settings")?;
    std::fs::write(config_path(), content).with_context(|| "failed to write config.toml")?;
    Ok(())
}

pub fn show_config(settings: &Settings) -> Result<()> {
    let path = config_path();
    println!("Config file: {}", path.display());
    if !path.exists() {
        println!("(not yet created — using defaults)");
    }
    println!();
    print!(
        "{}",
        toml::to_string_pretty(settings).with_context(|| "failed to serialize settings")?
    );
    Ok(())
}

#[allow(dead_code)]
pub fn ensure_default_config() -> Result<()> {
    let path = config_path();
    if !path.exists() {
        save(&Settings::default())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_is_under_config_dir() {
        assert_eq!(config_path(), config_dir().join("config.toml"));
    }

    #[test]
    fn config_dir_name_is_idoit() {
        assert_eq!(
            config_dir().file_name().and_then(|s| s.to_str()),
            Some("idoit")
        );
    }
}
