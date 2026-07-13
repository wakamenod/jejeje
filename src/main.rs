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
        Commands::Info => commands::info::run().await?,
        Commands::Contests { judge, limit } => commands::contests::run(judge, limit).await?,
        Commands::Config { key, value } => commands::config::run(key, value).await?,
    }

    Ok(())
}
