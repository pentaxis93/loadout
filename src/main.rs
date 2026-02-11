use anyhow::Result;
use clap::{Parser, Subcommand};
use loadout::{commands, config};

/// Loadout: Skill lifecycle management for AI agents
#[derive(Parser, Debug)]
#[command(name = "loadout")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Install skills by creating symlinks in target directories
    Install {
        /// Show what would happen without making changes
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install { dry_run } => {
            let config = config::load()?;
            commands::install(&config, dry_run)?;
        }
    }

    Ok(())
}
