use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};

fn aliases_path() -> PathBuf {
    crate::config::config_dir().join("aliases.toml")
}

pub fn load() -> BTreeMap<String, String> {
    let path = aliases_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return BTreeMap::new(),
    };
    toml::from_str(&content).unwrap_or_default()
}

pub fn save(name: &str, description: &str) -> Result<()> {
    let mut aliases = load();
    aliases.insert(name.to_string(), description.to_string());

    let dir = crate::config::config_dir();
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create {}", dir.display()))?;

    let content = toml::to_string_pretty(&aliases)
        .with_context(|| "failed to serialize aliases")?;
    std::fs::write(aliases_path(), content)
        .with_context(|| "failed to write aliases.toml")?;

    Ok(())
}

pub fn resolve(name: &str) -> Option<String> {
    load().get(name).cloned()
}

#[allow(dead_code)]
pub fn list() -> BTreeMap<String, String> {
    load()
}
