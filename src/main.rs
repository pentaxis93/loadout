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
    List {
        /// Show skills organized by detected clusters
        #[arg(long)]
        groups: bool,
        /// Show references for a specific skill
        #[arg(long)]
        refs: Option<String>,
        /// Show only missing skills (dangling references)
        #[arg(long)]
        missing: bool,
        /// Show all tags with skill counts
        #[arg(long)]
        tags: bool,
        /// Show skills with a specific tag
        #[arg(long)]
        tag: Option<String>,
        /// Show all pipelines with skill counts
        #[arg(long)]
        pipelines: bool,
        /// Show a specific pipeline in stage order
        #[arg(long)]
        pipeline: Option<String>,
    },
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
        Commands::List {
            groups,
            refs,
            missing,
            tags,
            tag,
            pipelines,
            pipeline,
        } => {
            let mode = if groups {
                commands::list::ListMode::Groups
            } else if let Some(skill_name) = refs {
                commands::list::ListMode::Refs(skill_name)
            } else if missing {
                commands::list::ListMode::Missing
            } else if tags {
                commands::list::ListMode::Tags
            } else if let Some(tag_name) = tag {
                commands::list::ListMode::Tag(tag_name)
            } else if pipelines {
                commands::list::ListMode::Pipelines
            } else if let Some(pipeline_name) = pipeline {
                commands::list::ListMode::Pipeline(pipeline_name)
            } else {
                commands::list::ListMode::Default
            };

            commands::list(&config, mode)?;
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
