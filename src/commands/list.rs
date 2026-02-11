//! List command implementation

use anyhow::Result;
use colored::Colorize;

use crate::config::Config;
use crate::skill;

/// List enabled skills per scope
pub fn list(config: &Config) -> Result<()> {
    // Discover all available skills
    let skills = skill::discover_all(&config.sources.skills)?;
    let skill_map = skill::build_skill_map(skills);

    // List global skills
    println!("{}", "--- Global scope ---".cyan().bold());
    println!("Skills: {}", config.global.skills.len());
    for skill_name in &config.global.skills {
        if let Some(skill) = skill_map.get(skill_name) {
            println!(
                "  {} {} ({})",
                "✓".green(),
                skill_name,
                skill.path.display().to_string().dimmed()
            );
        } else {
            println!("  {} {} {}", "✗".red(), skill_name, "(not found)".red());
        }
    }

    // List project skills
    for (project_path, project_config) in &config.projects {
        println!();
        println!(
            "{} {}",
            "--- Project:".cyan().bold(),
            project_path.display()
        );

        let mut all_skills = Vec::new();

        // Add global skills if inherited
        if project_config.inherit {
            all_skills.extend(config.global.skills.clone());
        }

        // Add project-specific skills
        all_skills.extend(project_config.skills.clone());

        // Deduplicate
        all_skills.sort();
        all_skills.dedup();

        println!(
            "Skills: {} (inherit: {})",
            all_skills.len(),
            if project_config.inherit {
                "true"
            } else {
                "false"
            }
        );

        for skill_name in &all_skills {
            if let Some(skill) = skill_map.get(skill_name) {
                let source = if config.global.skills.contains(skill_name) {
                    "global".dimmed()
                } else {
                    "project".dimmed()
                };
                println!(
                    "  {} {} ({}, {})",
                    "✓".green(),
                    skill_name,
                    source,
                    skill.path.display().to_string().dimmed()
                );
            } else {
                println!("  {} {} {}", "✗".red(), skill_name, "(not found)".red());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Global, Project, Sources};
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_skills(temp: &TempDir) {
        let skills_dir = temp.path().join("skills");

        let test_skill_dir = skills_dir.join("test-skill");
        fs::create_dir_all(&test_skill_dir).unwrap();
        fs::write(
            test_skill_dir.join("SKILL.md"),
            "---\nname: test-skill\ndescription: Test skill\n---\n",
        )
        .unwrap();

        let another_skill_dir = skills_dir.join("another-skill");
        fs::create_dir_all(&another_skill_dir).unwrap();
        fs::write(
            another_skill_dir.join("SKILL.md"),
            "---\nname: another-skill\ndescription: Another test skill\n---\n",
        )
        .unwrap();
    }

    #[test]
    fn should_list_global_skills() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec!["test-skill".to_string()],
            },
            projects: HashMap::new(),
        };

        // When - should not error
        let result = list(&config);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_list_project_skills_with_inheritance() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let mut projects = HashMap::new();
        projects.insert(
            temp.path().join("project"),
            Project {
                skills: vec!["another-skill".to_string()],
                inherit: true,
            },
        );

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec!["test-skill".to_string()],
            },
            projects,
        };

        // When - should not error
        let result = list(&config);

        // Then
        assert!(result.is_ok());
    }
}
