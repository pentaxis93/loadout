//! Validate command implementation

use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;

use crate::config::Config;
use crate::skill;

/// Validate SKILL.md files in source directories
///
/// Can validate:
/// - All skills from config sources (no arguments)
/// - A specific skill by name
/// - All skills in a specific directory
pub fn validate(config: &Config, target: Option<String>) -> Result<()> {
    let mut errors = 0;
    let mut validated = 0;

    match target {
        None => {
            // Validate all skills from configured sources
            println!(
                "{}",
                "Validating all skills from configured sources..."
                    .cyan()
                    .bold()
            );
            println!();

            for source in &config.sources.skills {
                println!("Source: {}", source.display());
                let skills = skill::discover_in_directory(source)?;

                for skill_result in skills {
                    validated += 1;
                    match validate_skill(&skill_result) {
                        Ok(_) => {
                            println!("  {} {}", "✓".green(), skill_result.name);
                        }
                        Err(e) => {
                            println!("  {} {} - {}", "✗".red(), skill_result.name, e);
                            errors += 1;
                        }
                    }
                }
            }
        }
        Some(target_str) => {
            let target_path = PathBuf::from(&target_str);

            if target_path.exists() && target_path.is_dir() {
                // Validate all skills in a directory
                println!(
                    "{} {}",
                    "Validating skills in:".cyan().bold(),
                    target_path.display()
                );
                println!();

                let skills = skill::discover_in_directory(&target_path)?;

                for skill_result in skills {
                    validated += 1;
                    match validate_skill(&skill_result) {
                        Ok(_) => {
                            println!("  {} {}", "✓".green(), skill_result.name);
                        }
                        Err(e) => {
                            println!("  {} {} - {}", "✗".red(), skill_result.name, e);
                            errors += 1;
                        }
                    }
                }
            } else {
                // Validate a specific skill by name
                println!("{} {}", "Validating skill:".cyan().bold(), target_str);
                println!();

                let skill_result = skill::resolve(&config.sources.skills, &target_str)?;
                validated += 1;

                match validate_skill(&skill_result) {
                    Ok(_) => {
                        println!("  {} {}", "✓".green(), skill_result.name);
                        println!(
                            "  Path: {}",
                            skill_result.path.display().to_string().dimmed()
                        );
                    }
                    Err(e) => {
                        println!("  {} {} - {}", "✗".red(), skill_result.name, e);
                        errors += 1;
                    }
                }
            }
        }
    }

    println!();
    if errors == 0 {
        println!("{} {} skills validated", "✓".green().bold(), validated);
        Ok(())
    } else {
        println!(
            "{} {} errors in {} skills",
            "✗".red().bold(),
            errors,
            validated
        );
        Err(anyhow::anyhow!("Validation failed"))
    }
}

/// Validate a single skill
fn validate_skill(skill: &skill::Skill) -> Result<()> {
    // Frontmatter is already validated during discovery
    // but we can do additional checks here if needed

    // Re-validate frontmatter
    skill.frontmatter.validate()?;

    // Validate directory name matches
    if let Some(dir_name) = skill.path.file_name().and_then(|n| n.to_str()) {
        skill.frontmatter.validate_directory_name(dir_name)?;
    }

    // Could add more validations here:
    // - Check for required content
    // - Validate XML structure
    // - Check for dead links
    // etc.

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Global, Sources};
    use std::collections::HashMap;

    #[test]
    fn should_validate_all_skills_from_config() {
        // Given
        let config = Config {
            sources: Sources {
                skills: vec![PathBuf::from("tests/fixtures/skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = validate(&config, None);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_validate_specific_skill_by_name() {
        // Given
        let config = Config {
            sources: Sources {
                skills: vec![PathBuf::from("tests/fixtures/skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = validate(&config, Some("test-skill".to_string()));

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_validate_skills_in_directory() {
        // Given
        let config = Config {
            sources: Sources { skills: vec![] },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = validate(&config, Some("tests/fixtures/skills".to_string()));

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_return_error_for_nonexistent_skill() {
        // Given
        let config = Config {
            sources: Sources {
                skills: vec![PathBuf::from("tests/fixtures/skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = validate(&config, Some("nonexistent-skill".to_string()));

        // Then
        assert!(result.is_err());
    }
}
