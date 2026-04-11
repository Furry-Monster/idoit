//! Binary crate: Tokio runtime, parse argv, dispatch to `commands`.

mod ai;
mod cli;
mod commands;
mod config;
mod macros;
mod parser;
mod session;
mod shell;
mod tui;

use anyhow::Result;
use clap::Parser;

use parser::Args;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        cli::output::print_error(&format!("{e:#}"));
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = Args::parse();
    commands::dispatch::run(args).await
}
