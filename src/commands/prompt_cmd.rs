//! Bare `idoit <words…>` — NL → command when words are not a reserved subcommand.

use std::ffi::OsString;

use anyhow::Result;
use clap::CommandFactory;

use crate::config::settings::Settings;
use crate::parser::{Args, GlobalOpts};

use super::translate;

pub async fn run(g: &GlobalOpts, settings: &Settings, parts: &[OsString]) -> Result<()> {
    let prompt = Args::join_prompt_os(parts);
    if prompt.is_empty() {
        Args::command().print_help()?;
        println!();
        return Ok(());
    }
    translate::run_from_cli(g, settings, &prompt).await
}
