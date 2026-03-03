//! Shared path policies and helpers.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::{Config, Project};

/// Resolve global target directories from configured target aliases.
pub fn global_targets(config: &Config) -> Result<Vec<PathBuf>> {
    config
        .global
        .targets
        .iter()
        .map(|alias| {
            config
                .target_aliases
                .get(alias)
                .map(|paths| paths.global.clone())
                .with_context(|| format!("Unknown target alias '{alias}' in global.targets"))
        })
        .collect()
}

/// Resolve project-local target directories for one project.
pub fn project_targets(
    config: &Config,
    project_path: &Path,
    project: &Project,
) -> Result<Vec<PathBuf>> {
    let aliases = project.targets.as_ref().unwrap_or(&config.global.targets);

    let mut resolved = Vec::with_capacity(aliases.len());
    for alias in aliases {
        let paths = config.target_aliases.get(alias).with_context(|| {
            format!(
                "Unknown target alias '{alias}' in projects.\"{}\".targets",
                project_path.display()
            )
        })?;

        let target = if paths.project.is_relative() {
            project_path.join(&paths.project)
        } else {
            paths.project.clone()
        };

        resolved.push(target);
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CheckConfig, Global, Sources, TargetAliasPaths};
    use std::collections::HashMap;

    fn test_config() -> Config {
        Config {
            sources: Sources { skills: vec![] },
            global: Global {
                targets: vec![
                    "claude_code".to_string(),
                    "opencode".to_string(),
                    "codex".to_string(),
                ],
                skills: vec![],
            },
            target_aliases: HashMap::from([
                (
                    "claude_code".to_string(),
                    TargetAliasPaths {
                        global: PathBuf::from("/home/user/.claude/skills"),
                        project: PathBuf::from(".claude/skills"),
                    },
                ),
                (
                    "opencode".to_string(),
                    TargetAliasPaths {
                        global: PathBuf::from("/home/user/.config/opencode/skills"),
                        project: PathBuf::from(".opencode/skills"),
                    },
                ),
                (
                    "codex".to_string(),
                    TargetAliasPaths {
                        global: PathBuf::from("/home/user/.agents/skills"),
                        project: PathBuf::from(".agents/skills"),
                    },
                ),
            ]),
            projects: HashMap::new(),
            check: CheckConfig::default(),
        }
    }

    #[test]
    fn should_resolve_global_targets_in_configured_order() {
        // Given
        let config = test_config();

        // When
        let targets = global_targets(&config).unwrap();

        // Then
        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0], PathBuf::from("/home/user/.claude/skills"));
        assert_eq!(
            targets[1],
            PathBuf::from("/home/user/.config/opencode/skills")
        );
        assert_eq!(targets[2], PathBuf::from("/home/user/.agents/skills"));
    }

    #[test]
    fn should_resolve_project_targets_from_global_aliases_when_project_targets_omitted() {
        // Given
        let config = test_config();
        let project_path = PathBuf::from("/repo");
        let project = Project {
            skills: vec![],
            inherit: true,
            targets: None,
        };

        // When
        let targets = project_targets(&config, &project_path, &project).unwrap();

        // Then
        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0], project_path.join(".claude/skills"));
        assert_eq!(targets[1], project_path.join(".opencode/skills"));
        assert_eq!(targets[2], project_path.join(".agents/skills"));
    }

    #[test]
    fn should_resolve_subset_of_project_targets() {
        // Given
        let config = test_config();
        let project_path = PathBuf::from("/repo");
        let project = Project {
            skills: vec![],
            inherit: true,
            targets: Some(vec!["claude_code".to_string(), "codex".to_string()]),
        };

        // When
        let targets = project_targets(&config, &project_path, &project).unwrap();

        // Then
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0], project_path.join(".claude/skills"));
        assert_eq!(targets[1], project_path.join(".agents/skills"));
    }

    #[test]
    fn should_return_empty_project_targets_when_project_targets_is_empty() {
        // Given
        let config = test_config();
        let project_path = PathBuf::from("/repo");
        let project = Project {
            skills: vec![],
            inherit: true,
            targets: Some(vec![]),
        };

        // When
        let targets = project_targets(&config, &project_path, &project).unwrap();

        // Then
        assert!(targets.is_empty());
    }

    #[test]
    fn should_support_absolute_project_alias_paths() {
        // Given
        let mut config = test_config();
        config.target_aliases.insert(
            "custom".to_string(),
            TargetAliasPaths {
                global: PathBuf::from("/global/custom"),
                project: PathBuf::from("/absolute/project/custom"),
            },
        );
        let project_path = PathBuf::from("/repo");
        let project = Project {
            skills: vec![],
            inherit: true,
            targets: Some(vec!["custom".to_string()]),
        };

        // When
        let targets = project_targets(&config, &project_path, &project).unwrap();

        // Then
        assert_eq!(targets, vec![PathBuf::from("/absolute/project/custom")]);
    }
}
