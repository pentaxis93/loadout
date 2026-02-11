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
    /// Remove all managed symlinks from target directories
    Clean {
        /// Show what would happen without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// List enabled skills per scope
    List,
    /// Validate SKILL.md files
    Validate {
        /// Skill name or directory path (validates all if not specified)
        target: Option<String>,
    },
    /// Create a new skill from template
    New {
        /// Skill name (lowercase-with-hyphens)
        name: String,
        /// Skill description
        #[arg(short, long)]
        description: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config = config::load()?;

    match cli.command {
        Commands::Install { dry_run } => {
            commands::install(&config, dry_run)?;
        }
        Commands::Clean { dry_run } => {
            commands::clean(&config, dry_run)?;
        }
        Commands::List => {
            commands::list(&config)?;
        }
        Commands::Validate { target } => {
            commands::validate(&config, target)?;
        }
        Commands::New { name, description } => {
            commands::new(&config, name, description)?;
        }
    }

    Ok(())
}
