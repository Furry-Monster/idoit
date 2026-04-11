//! `idoit run …` — explicit natural-language → shell command (same pipeline as bare prompt).

use anyhow::Result;

use crate::config::settings::Settings;
use crate::parser::{Args, GlobalOpts};

use super::translate;

pub async fn run(g: &GlobalOpts, settings: &Settings, prompt: &[String]) -> Result<()> {
    let prompt = Args::join_prompt(prompt);
    if prompt.is_empty() {
        anyhow::bail!("run needs a prompt. Example: idoit run find all TODO in src");
    }
    translate::run_from_cli(g, settings, &prompt).await
}
