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
    /// Check skill system health and report diagnostics
    Check {
        /// Filter by minimum severity (error, warning, info)
        #[arg(long)]
        severity: Option<String>,
    },
    /// Visualize skill dependency graph
    #[cfg(feature = "graph")]
    Graph {
        /// Output format: dot, text, json, mermaid
        #[arg(long, default_value = "text")]
        format: String,
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
        Commands::Check { severity } => {
            let filter = severity
                .as_deref()
                .and_then(|s| match s.to_lowercase().as_str() {
                    "error" => Some(commands::check::Severity::Error),
                    "warning" => Some(commands::check::Severity::Warning),
                    "info" => Some(commands::check::Severity::Info),
                    _ => {
                        eprintln!(
                            "Invalid severity: {}. Valid values: error, warning, info",
                            s
                        );
                        std::process::exit(1);
                    }
                });

            let findings = commands::check(&config, filter)?;
            commands::print_check_findings(&findings);
            std::process::exit(commands::check_exit_code(&findings));
        }
        #[cfg(feature = "graph")]
        Commands::Graph { format } => {
            let output_format =
                commands::graph::OutputFormat::from_str(&format).unwrap_or_else(|| {
                    eprintln!(
                        "Invalid format: {}. Valid values: dot, text, json, mermaid",
                        format
                    );
                    std::process::exit(1);
                });

            commands::graph(&config, output_format)?;
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
