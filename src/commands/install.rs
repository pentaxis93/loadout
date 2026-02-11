//! Install command implementation

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::Config;
use crate::linker;
use crate::skill;

const PROJECT_SUBDIRS: &[&str] = &[".claude/skills", ".opencode/skills", ".agents/skills"];

/// Install skills by creating symlinks in target directories
///
/// This function:
/// - Discovers all skills from configured source directories
/// - Links global skills to global target directories
/// - Links project skills to project-local target directories
/// - Respects project `inherit` setting for global skills
pub fn install(config: &Config, dry_run: bool) -> Result<()> {
    // Discover all available skills
    let skills = skill::discover_all(&config.sources.skills)
        .context("Failed to discover skills from source directories")?;

    let skill_map = skill::build_skill_map(skills);

    if dry_run {
        println!("{}", "[DRY RUN MODE]".yellow().bold());
        println!();
    }

    // Link global skills
    install_global_skills(config, &skill_map, dry_run)?;

    // Link project skills
    install_project_skills(config, &skill_map, dry_run)?;

    if !dry_run {
        println!();
        println!("{}", "Done.".green().bold());
    }

    Ok(())
}

/// Install global skills to global target directories
fn install_global_skills(
    config: &Config,
    skill_map: &HashMap<String, skill::Skill>,
    dry_run: bool,
) -> Result<()> {
    println!("{}", "--- Global scope ---".cyan().bold());

    for target in &config.global.targets {
        println!("Target: {}", target.display());

        for skill_name in &config.global.skills {
            install_skill(skill_name, skill_map, target, dry_run)?;
        }
    }

    Ok(())
}

/// Install project-specific skills to project-local target directories
fn install_project_skills(
    config: &Config,
    skill_map: &HashMap<String, skill::Skill>,
    dry_run: bool,
) -> Result<()> {
    for (project_path, project_config) in &config.projects {
        println!();
        println!(
            "{} {}",
            "--- Project:".cyan().bold(),
            project_path.display()
        );

        for subdir in PROJECT_SUBDIRS {
            let target = project_path.join(subdir);
            println!("Target: {}", target.display());

            // Link global skills if inherit is true
            if project_config.inherit {
                for skill_name in &config.global.skills {
                    install_skill(skill_name, skill_map, &target, dry_run)?;
                }
            }

            // Link project-specific skills
            for skill_name in &project_config.skills {
                install_skill(skill_name, skill_map, &target, dry_run)?;
            }
        }
    }

    Ok(())
}

/// Install a single skill to a target directory
fn install_skill(
    skill_name: &str,
    skill_map: &HashMap<String, skill::Skill>,
    target: &Path,
    dry_run: bool,
) -> Result<()> {
    let skill = skill_map.get(skill_name).context(format!(
        "Skill '{}' not found in source directories",
        skill_name
    ))?;

    if dry_run {
        println!(
            "  {} {} -> {}",
            "[dry-run]".yellow(),
            skill.path.display(),
            target.join(skill_name).display()
        );
    } else {
        linker::link_skill(skill_name, &skill.path, target).context(format!(
            "Failed to link skill '{}' to {}",
            skill_name,
            target.display()
        ))?;

        println!(
            "  {} {} -> {}",
            "linked:".green(),
            skill_name,
            target.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Global, Project, Sources};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config(temp: &TempDir) -> Config {
        let skill_source = temp.path().join("skills");
        let global_target = temp.path().join("global");
        let project_path = temp.path().join("project");

        Config {
            sources: Sources {
                skills: vec![skill_source],
            },
            global: Global {
                targets: vec![global_target],
                skills: vec!["test-skill".to_string()],
            },
            projects: {
                let mut projects = HashMap::new();
                projects.insert(
                    project_path,
                    Project {
                        skills: vec!["another-skill".to_string()],
                        inherit: true,
                    },
                );
                projects
            },
        }
    }

    fn create_test_skills(temp: &TempDir) {
        let skills_dir = temp.path().join("skills");

        // Create test-skill
        let test_skill_dir = skills_dir.join("test-skill");
        fs::create_dir_all(&test_skill_dir).unwrap();
        fs::write(
            test_skill_dir.join("SKILL.md"),
            "---\nname: test-skill\ndescription: Test skill\n---\n",
        )
        .unwrap();

        // Create another-skill
        let another_skill_dir = skills_dir.join("another-skill");
        fs::create_dir_all(&another_skill_dir).unwrap();
        fs::write(
            another_skill_dir.join("SKILL.md"),
            "---\nname: another-skill\ndescription: Another test skill\n---\n",
        )
        .unwrap();
    }

    #[test]
    fn should_install_global_skills() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);
        let config = create_test_config(&temp);

        // When
        install(&config, false).unwrap();

        // Then
        let global_target = temp.path().join("global");
        assert!(global_target.join("test-skill").exists());
        assert!(global_target.join("test-skill").is_symlink());
    }

    #[test]
    fn should_install_project_skills_with_inheritance() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);
        let config = create_test_config(&temp);

        // When
        install(&config, false).unwrap();

        // Then
        let project_target = temp.path().join("project/.claude/skills");
        assert!(project_target.join("test-skill").exists()); // inherited
        assert!(project_target.join("another-skill").exists()); // project-specific
    }

    #[test]
    fn should_respect_project_inherit_false() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);
        let mut config = create_test_config(&temp);

        // Set inherit to false
        let project_path = temp.path().join("project");
        config.projects.get_mut(&project_path).unwrap().inherit = false;

        // When
        install(&config, false).unwrap();

        // Then
        let project_target = temp.path().join("project/.claude/skills");
        assert!(!project_target.join("test-skill").exists()); // NOT inherited
        assert!(project_target.join("another-skill").exists()); // project-specific
    }

    #[test]
    fn should_create_symlinks_in_all_project_subdirs() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);
        let config = create_test_config(&temp);

        // When
        install(&config, false).unwrap();

        // Then
        for subdir in PROJECT_SUBDIRS {
            let target = temp.path().join("project").join(subdir);
            assert!(target.join("test-skill").exists());
            assert!(target.join("another-skill").exists());
        }
    }

    #[test]
    fn should_not_create_symlinks_in_dry_run_mode() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);
        let config = create_test_config(&temp);

        // When
        install(&config, true).unwrap();

        // Then
        let global_target = temp.path().join("global");
        assert!(!global_target.exists());
    }

    #[test]
    fn should_return_error_when_skill_not_found() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);
        let mut config = create_test_config(&temp);

        // Add non-existent skill
        config.global.skills.push("nonexistent".to_string());

        // When
        let result = install(&config, false);

        // Then
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
        assert!(err.to_string().contains("nonexistent"));
    }
}
