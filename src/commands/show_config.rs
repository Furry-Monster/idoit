//! `idoit config` — print current effective settings (TOML).

use anyhow::Result;

use crate::config;
use crate::config::settings::Settings;

pub fn run(settings: &Settings) -> Result<()> {
    config::show_config(settings)
}
