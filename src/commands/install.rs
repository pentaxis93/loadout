//! Install command implementation

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::Config;
use crate::linker;
use crate::skill;

#[derive(Debug)]
struct TargetPlan {
    target: PathBuf,
    skills: Vec<String>,
}

#[derive(Debug)]
struct ProjectPlan {
    project_path: PathBuf,
    targets: Vec<TargetPlan>,
}

#[derive(Debug)]
struct InstallPlan {
    global_targets: Vec<TargetPlan>,
    project_targets: Vec<ProjectPlan>,
}

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
    let install_plan = build_install_plan(config)?;

    if dry_run {
        println!("{}", "[DRY RUN MODE]".yellow().bold());
        println!();
    }

    // Reconcile + link global skills
    install_global_skills(&install_plan, &skill_map, dry_run)?;

    // Reconcile + link project skills
    install_project_skills(&install_plan, &skill_map, dry_run)?;

    if !dry_run {
        println!();
        println!("{}", "Done.".green().bold());
    }

    Ok(())
}

fn build_install_plan(config: &Config) -> Result<InstallPlan> {
    let mut aliases: Vec<_> = config.target_aliases.keys().cloned().collect();
    aliases.sort();
    validate_global_aliases(config)?;

    let selected_global: HashSet<_> = config.global.targets.iter().cloned().collect();
    let mut global_targets = Vec::new();

    for alias in &aliases {
        let alias_paths = config
            .target_aliases
            .get(alias)
            .context(format!("Unknown target alias '{alias}' in global.targets"))?;

        let skills = if selected_global.contains(alias) {
            unique_skills(config.global.skills.iter().cloned())
        } else {
            Vec::new()
        };

        global_targets.push(TargetPlan {
            target: alias_paths.global.clone(),
            skills,
        });
    }

    let mut project_entries: Vec<_> = config.projects.iter().collect();
    project_entries.sort_by(|(left, _), (right, _)| left.cmp(right));

    let mut project_targets = Vec::new();
    for (project_path, project_config) in project_entries {
        validate_project_aliases(config, project_path, project_config)?;

        let selected_aliases: HashSet<String> = project_config
            .targets
            .as_ref()
            .unwrap_or(&config.global.targets)
            .iter()
            .cloned()
            .collect();

        let mut targets = Vec::new();
        for alias in &aliases {
            let alias_paths = config.target_aliases.get(alias).context(format!(
                "Unknown target alias '{alias}' in projects.\"{}\".targets",
                project_path.display()
            ))?;

            let target = if alias_paths.project.is_relative() {
                project_path.join(&alias_paths.project)
            } else {
                alias_paths.project.clone()
            };

            let mut skills = Vec::new();
            if selected_aliases.contains(alias) {
                if project_config.inherit {
                    skills.extend(config.global.skills.iter().cloned());
                }
                skills.extend(project_config.skills.iter().cloned());
            }

            targets.push(TargetPlan {
                target,
                skills: unique_skills(skills),
            });
        }

        project_targets.push(ProjectPlan {
            project_path: project_path.clone(),
            targets,
        });
    }

    Ok(InstallPlan {
        global_targets,
        project_targets,
    })
}

fn validate_global_aliases(config: &Config) -> Result<()> {
    for alias in &config.global.targets {
        if !config.target_aliases.contains_key(alias) {
            anyhow::bail!("Unknown target alias '{alias}' in global.targets");
        }
    }
    Ok(())
}

fn validate_project_aliases(
    config: &Config,
    project_path: &Path,
    project_config: &crate::config::Project,
) -> Result<()> {
    let aliases = project_config
        .targets
        .as_ref()
        .unwrap_or(&config.global.targets);
    for alias in aliases {
        if !config.target_aliases.contains_key(alias) {
            anyhow::bail!(
                "Unknown target alias '{alias}' in projects.\"{}\".targets",
                project_path.display()
            );
        }
    }
    Ok(())
}

fn unique_skills(skills: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut deduped = BTreeSet::new();
    for skill in skills {
        deduped.insert(skill);
    }
    deduped.into_iter().collect()
}

/// Reconcile + install global skills to global target directories.
fn install_global_skills(
    plan: &InstallPlan,
    skill_map: &HashMap<String, skill::Skill>,
    dry_run: bool,
) -> Result<()> {
    println!("{}", "--- Global scope ---".cyan().bold());

    for target_plan in &plan.global_targets {
        println!("Target: {}", target_plan.target.display());
        prune_stale_links(&target_plan.target, &target_plan.skills, dry_run)?;

        for skill_name in &target_plan.skills {
            install_skill(skill_name, skill_map, &target_plan.target, dry_run)?;
        }
    }

    Ok(())
}

/// Reconcile + install project-specific skills to project-local target directories.
fn install_project_skills(
    plan: &InstallPlan,
    skill_map: &HashMap<String, skill::Skill>,
    dry_run: bool,
) -> Result<()> {
    for project_plan in &plan.project_targets {
        println!();
        println!(
            "{} {}",
            "--- Project:".cyan().bold(),
            project_plan.project_path.display()
        );

        for target_plan in &project_plan.targets {
            println!("Target: {}", target_plan.target.display());
            prune_stale_links(&target_plan.target, &target_plan.skills, dry_run)?;

            for skill_name in &target_plan.skills {
                install_skill(skill_name, skill_map, &target_plan.target, dry_run)?;
            }
        }
    }

    Ok(())
}

fn prune_stale_links(target: &Path, desired_skills: &[String], dry_run: bool) -> Result<()> {
    let removed = if dry_run {
        linker::preview_prune_target(target, desired_skills)?
    } else {
        linker::prune_target_except(target, desired_skills)?
    };

    for stale_path in removed {
        if dry_run {
            println!(
                "  {} would prune: {}",
                "[dry-run]".yellow(),
                stale_path.display()
            );
        } else {
            println!("  {} {}", "pruned:".green(), stale_path.display());
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
    use crate::config::{Global, Project, Sources, TargetAliasPaths};
    use crate::paths;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config(temp: &TempDir) -> Config {
        let skill_source = temp.path().join("skills");
        let global_target = temp.path().join("global");
        let project_path = temp.path().join("project");

        let mut target_aliases = HashMap::new();
        target_aliases.insert(
            "test_runner".to_string(),
            TargetAliasPaths {
                global: global_target,
                project: std::path::PathBuf::from(".test-runner/skills"),
            },
        );
        target_aliases.insert(
            "codex".to_string(),
            TargetAliasPaths {
                global: temp.path().join("codex-global"),
                project: std::path::PathBuf::from(".agents/skills"),
            },
        );
        target_aliases.insert(
            "claude_code".to_string(),
            TargetAliasPaths {
                global: temp.path().join("claude-global"),
                project: std::path::PathBuf::from(".claude/skills"),
            },
        );
        target_aliases.insert(
            "opencode".to_string(),
            TargetAliasPaths {
                global: temp.path().join("opencode-global"),
                project: std::path::PathBuf::from(".opencode/skills"),
            },
        );

        Config {
            sources: Sources {
                skills: vec![skill_source],
            },
            global: Global {
                targets: vec!["test_runner".to_string()],
                skills: vec!["test-skill".to_string()],
            },
            target_aliases,
            projects: {
                let mut projects = HashMap::new();
                projects.insert(
                    project_path,
                    Project {
                        skills: vec!["another-skill".to_string()],
                        inherit: true,
                        targets: None,
                    },
                );
                projects
            },
            check: Default::default(),
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
        let project_target = temp.path().join("project/.test-runner/skills");
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
        let project_target = temp.path().join("project/.test-runner/skills");
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
        let project_path = temp.path().join("project");
        let project = config.projects.get(&project_path).unwrap();
        for target in paths::project_targets(&config, &project_path, project).unwrap() {
            assert!(target.join("test-skill").exists());
            assert!(target.join("another-skill").exists());
        }
    }

    #[test]
    fn should_install_project_skills_into_codex_directory() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);
        let mut config = create_test_config(&temp);
        let project_path = temp.path().join("project");
        config.projects.get_mut(&project_path).unwrap().targets = Some(vec!["codex".to_string()]);

        // When
        install(&config, false).unwrap();

        // Then
        let project_target = temp.path().join("project/.agents/skills");
        assert!(project_target.join("test-skill").exists()); // inherited
        assert!(project_target.join("another-skill").exists()); // project-specific
    }

    #[test]
    fn should_install_project_skills_to_selected_targets() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);
        let mut config = create_test_config(&temp);
        let project_path = temp.path().join("project");
        config.projects.get_mut(&project_path).unwrap().targets =
            Some(vec!["codex".to_string(), "claude_code".to_string()]);

        // When
        install(&config, false).unwrap();

        // Then
        assert!(temp
            .path()
            .join("project/.agents/skills/test-skill")
            .exists());
        assert!(temp
            .path()
            .join("project/.claude/skills/test-skill")
            .exists());
        assert!(!temp
            .path()
            .join("project/.opencode/skills/test-skill")
            .exists());
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

    #[test]
    fn should_install_project_skill_from_relative_source_with_project_dot_key() {
        // Given
        let temp = TempDir::new().unwrap();
        let project_root = temp.path();

        let demo_skill_dir = project_root.join("skills").join("demo");
        fs::create_dir_all(&demo_skill_dir).unwrap();
        fs::write(
            demo_skill_dir.join("SKILL.md"),
            "---\nname: demo\ndescription: Demo skill\n---\n",
        )
        .unwrap();

        let config_path = project_root.join("loadout.toml");
        fs::write(
            &config_path,
            r#"
[sources]
skills = ["skills"]

[global]
targets = ["codex", "claude_code", "opencode"]
skills = []

[projects."."]
skills = ["demo"]
inherit = false
"#,
        )
        .unwrap();
        let config = crate::config::load_from(&config_path).unwrap();

        // When
        install(&config, false).unwrap();

        // Then
        let expected = fs::canonicalize(project_root.join("skills/demo")).unwrap();
        for link_path in [
            project_root.join(".claude/skills/demo"),
            project_root.join(".opencode/skills/demo"),
            project_root.join(".agents/skills/demo"),
        ] {
            assert!(link_path.is_symlink());
            assert_eq!(fs::canonicalize(link_path).unwrap(), expected);
        }
    }

    #[test]
    fn should_prune_removed_global_skill_when_reinstalling() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);
        let mut config = create_test_config(&temp);
        install(&config, false).unwrap();
        let global_target = temp.path().join("global");
        assert!(global_target.join("test-skill").exists());

        config.global.skills.clear();

        // When
        install(&config, false).unwrap();

        // Then
        assert!(!global_target.join("test-skill").exists());
    }

    #[test]
    fn should_prune_removed_project_skill_when_reinstalling() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);
        let mut config = create_test_config(&temp);
        install(&config, false).unwrap();
        let project_target = temp.path().join("project/.test-runner/skills");
        assert!(project_target.join("another-skill").exists());

        let project_path = temp.path().join("project");
        config
            .projects
            .get_mut(&project_path)
            .unwrap()
            .skills
            .clear();
        config.projects.get_mut(&project_path).unwrap().inherit = false;

        // When
        install(&config, false).unwrap();

        // Then
        assert!(!project_target.join("another-skill").exists());
    }

    #[test]
    fn should_prune_links_in_deselected_alias_targets() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);
        let mut config = create_test_config(&temp);
        config.global.targets = vec!["claude_code".to_string(), "codex".to_string()];
        let project_path = temp.path().join("project");
        config.projects.get_mut(&project_path).unwrap().targets =
            Some(vec!["claude_code".to_string(), "codex".to_string()]);
        install(&config, false).unwrap();
        assert!(temp.path().join("claude-global/test-skill").exists());
        assert!(temp
            .path()
            .join("project/.claude/skills/test-skill")
            .exists());

        config.global.targets = vec!["codex".to_string()];
        config.projects.get_mut(&project_path).unwrap().targets = Some(vec!["codex".to_string()]);

        // When
        install(&config, false).unwrap();

        // Then
        assert!(!temp.path().join("claude-global/test-skill").exists());
        assert!(!temp
            .path()
            .join("project/.claude/skills/test-skill")
            .exists());
        assert!(temp.path().join("codex-global/test-skill").exists());
        assert!(temp
            .path()
            .join("project/.agents/skills/test-skill")
            .exists());
    }

    #[test]
    fn should_not_prune_in_dry_run_mode() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);
        let mut config = create_test_config(&temp);
        install(&config, false).unwrap();
        let global_target = temp.path().join("global");
        assert!(global_target.join("test-skill").exists());
        config.global.skills.clear();

        // When
        install(&config, true).unwrap();

        // Then
        assert!(global_target.join("test-skill").exists());
    }
}
