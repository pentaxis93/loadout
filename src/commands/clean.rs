//! Clean command implementation

use anyhow::Result;
use colored::Colorize;

use crate::config::Config;
use crate::linker;

const PROJECT_SUBDIRS: &[&str] = &[".claude/skills", ".opencode/skills", ".agents/skills"];

/// Remove all managed symlinks from target directories
pub fn clean(config: &Config, dry_run: bool) -> Result<()> {
    if dry_run {
        println!("{}", "[DRY RUN MODE]".yellow().bold());
        println!();
    }

    let mut total_removed = 0;

    // Clean global targets
    println!("{}", "--- Global scope ---".cyan().bold());
    for target in &config.global.targets {
        if dry_run {
            if linker::is_managed(target) {
                println!(
                    "  {} would clean: {}",
                    "[dry-run]".yellow(),
                    target.display()
                );
            }
        } else {
            let removed = linker::clean_target(target)?;
            if !removed.is_empty() {
                println!(
                    "  {} {} (removed {} symlinks)",
                    "cleaned:".green(),
                    target.display(),
                    removed.len()
                );
                total_removed += removed.len();
            }
        }
    }

    // Clean project targets
    for project_path in config.projects.keys() {
        println!();
        println!(
            "{} {}",
            "--- Project:".cyan().bold(),
            project_path.display()
        );

        for subdir in PROJECT_SUBDIRS {
            let target = project_path.join(subdir);
            if dry_run {
                if linker::is_managed(&target) {
                    println!(
                        "  {} would clean: {}",
                        "[dry-run]".yellow(),
                        target.display()
                    );
                }
            } else {
                let removed = linker::clean_target(&target)?;
                if !removed.is_empty() {
                    println!(
                        "  {} {} (removed {} symlinks)",
                        "cleaned:".green(),
                        target.display(),
                        removed.len()
                    );
                    total_removed += removed.len();
                }
            }
        }
    }

    if !dry_run {
        println!();
        println!(
            "{} {}",
            "Done.".green().bold(),
            format!("Removed {} symlinks", total_removed).dimmed()
        );
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
                skills: vec![],
            },
            projects: {
                let mut projects = HashMap::new();
                projects.insert(
                    project_path,
                    Project {
                        skills: vec![],
                        inherit: false,
                    },
                );
                projects
            },
        }
    }

    fn create_managed_target(target: &std::path::Path, skill_name: &str) {
        let skill_dir = target.parent().unwrap().join("skill-source");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::create_dir_all(target).unwrap();

        linker::link_skill(skill_name, &skill_dir, target).unwrap();
    }

    #[test]
    fn should_clean_global_targets() {
        // Given
        let temp = TempDir::new().unwrap();
        let config = create_test_config(&temp);
        let global_target = temp.path().join("global");

        create_managed_target(&global_target, "test-skill");

        // When
        clean(&config, false).unwrap();

        // Then
        assert!(!global_target.join("test-skill").exists());
        assert!(!linker::is_managed(&global_target));
    }

    #[test]
    fn should_clean_project_targets() {
        // Given
        let temp = TempDir::new().unwrap();
        let config = create_test_config(&temp);
        let project_target = temp.path().join("project/.claude/skills");

        create_managed_target(&project_target, "test-skill");

        // When
        clean(&config, false).unwrap();

        // Then
        assert!(!project_target.join("test-skill").exists());
        assert!(!linker::is_managed(&project_target));
    }

    #[test]
    fn should_not_clean_in_dry_run_mode() {
        // Given
        let temp = TempDir::new().unwrap();
        let config = create_test_config(&temp);
        let global_target = temp.path().join("global");

        create_managed_target(&global_target, "test-skill");

        // When
        clean(&config, true).unwrap();

        // Then - symlink still exists
        assert!(global_target.join("test-skill").exists());
        assert!(linker::is_managed(&global_target));
    }

    #[test]
    fn should_skip_unmanaged_directories() {
        // Given
        let temp = TempDir::new().unwrap();
        let config = create_test_config(&temp);
        let global_target = temp.path().join("global");

        // Create unmanaged directory (no marker)
        fs::create_dir_all(&global_target).unwrap();
        fs::write(global_target.join("some-file.txt"), "content").unwrap();

        // When
        clean(&config, false).unwrap();

        // Then - file still exists
        assert!(global_target.join("some-file.txt").exists());
    }
}
