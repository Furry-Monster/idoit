mod ai;
mod cli;
mod commands;
mod config;
mod macros;
mod session;
mod shell;
mod tui;
mod ui;

use anyhow::Result;
use clap::Parser;

use cli::Cli;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        ui::output::print_error(&format!("{e:#}"));
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    commands::dispatch::run(cli).await
}
