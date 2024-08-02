//! Module for the CLI.

use clap::Parser;
use color_eyre::eyre::{eyre, Result};
use tracing::Level;

pub mod blobs;
pub mod fixtures;
pub mod from_l1;
pub mod from_l2;
pub mod info;
pub use fixtures::build_fixture_blocks;

/// Main opd8n CLI
#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Subcommands for the CLI
    #[command(subcommand)]
    pub command: Commands,
}

/// Subcommands for the CLI
#[derive(Parser, Clone, Debug)]
pub enum Commands {
    /// Creates the derivation fixture with L1 Block content for a range of L2 Blocks.
    FromL2(from_l2::FromL2),
    /// Creates the derivation fixture for a given L1 Block.
    FromL1(from_l1::FromL1),
    /// Gets the L2 block info including the l1 origin for the l2 block number.
    Info(info::Info),
}

impl Cli {
    /// Returns the verbosity level for the CLI
    pub fn v(&self) -> u8 {
        match &self.command {
            Commands::FromL2(cmd) => cmd.v,
            Commands::FromL1(cmd) => cmd.v,
            Commands::Info(cmd) => cmd.v,
        }
    }

    /// Initializes telemtry for the application.
    pub fn init_telemetry(self) -> Result<Self> {
        color_eyre::install()?;
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(match self.v() {
                0 => Level::ERROR,
                1 => Level::WARN,
                2 => Level::INFO,
                3 => Level::DEBUG,
                _ => Level::TRACE,
            })
            .finish();
        tracing::subscriber::set_global_default(subscriber).map_err(|e| eyre!(e))?;
        Ok(self)
    }

    /// Parse the CLI arguments and run the command
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::FromL2(cmd) => cmd.run().await,
            Commands::FromL1(cmd) => cmd.run().await,
            Commands::Info(cmd) => cmd.run().await,
        }
    }
}
