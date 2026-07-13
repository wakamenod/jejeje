mod cli;
mod commands;
mod config;
mod error;
mod judge;
mod meta;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Prepare { url } => commands::prepare::run(url).await?,
        Commands::Test {
            command,
            tle,
            epsilon,
        } => commands::test::run(command, tle, epsilon).await?,
        Commands::Contest => commands::contest::run().await?,
        Commands::Tasks => commands::tasks::run().await?,
        Commands::Config { key, value } => commands::config::run(key, value).await?,
    }

    Ok(())
}
