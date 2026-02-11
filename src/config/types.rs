//! Configuration type definitions for loadout.toml

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Complete configuration loaded from loadout.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Source directories containing skills
    pub sources: Sources,

    /// Global skill activation
    pub global: Global,

    /// Per-project overrides (keyed by project path)
    #[serde(default)]
    pub projects: HashMap<PathBuf, Project>,
}

/// Source directories configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sources {
    /// List of directories to search for skills (in priority order)
    pub skills: Vec<PathBuf>,
}

/// Global skill configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Global {
    /// Target directories where skills will be symlinked
    pub targets: Vec<PathBuf>,

    /// Skills to enable globally
    pub skills: Vec<String>,
}

/// Project-specific skill configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Skills to enable for this project
    pub skills: Vec<String>,

    /// Whether to include global skills (default: true)
    #[serde(default = "default_inherit")]
    pub inherit: bool,
}

fn default_inherit() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_deserialize_minimal_config() {
        // Given
        let toml = r#"
            [sources]
            skills = ["/home/user/.config/loadout/skills"]

            [global]
            targets = ["/home/user/.claude/skills"]
            skills = ["my-skill"]
        "#;

        // When
        let config: Config = toml::from_str(toml).unwrap();

        // Then
        assert_eq!(config.sources.skills.len(), 1);
        assert_eq!(config.global.targets.len(), 1);
        assert_eq!(config.global.skills.len(), 1);
        assert_eq!(config.global.skills[0], "my-skill");
        assert!(config.projects.is_empty());
    }

    #[test]
    fn should_deserialize_config_with_projects() {
        // Given
        let toml = r#"
            [sources]
            skills = ["/home/user/.config/loadout/skills"]

            [global]
            targets = ["/home/user/.claude/skills"]
            skills = ["global-skill"]

            [projects."/home/user/my-project"]
            skills = ["project-skill"]
            inherit = false
        "#;

        // When
        let config: Config = toml::from_str(toml).unwrap();

        // Then
        let project_path = PathBuf::from("/home/user/my-project");
        assert!(config.projects.contains_key(&project_path));
        let project = &config.projects[&project_path];
        assert_eq!(project.skills.len(), 1);
        assert_eq!(project.skills[0], "project-skill");
        assert!(!project.inherit);
    }

    #[test]
    fn should_default_project_inherit_to_true() {
        // Given
        let toml = r#"
            [sources]
            skills = ["/home/user/.config/loadout/skills"]

            [global]
            targets = ["/home/user/.claude/skills"]
            skills = []

            [projects."/home/user/my-project"]
            skills = []
        "#;

        // When
        let config: Config = toml::from_str(toml).unwrap();

        // Then
        let project_path = PathBuf::from("/home/user/my-project");
        let project = &config.projects[&project_path];
        assert!(project.inherit);
    }

    #[test]
    fn should_handle_multiple_sources() {
        // Given
        let toml = r#"
            [sources]
            skills = [
                "/home/user/.config/loadout/skills",
                "/opt/shared-skills",
                "/home/user/projects/team-skills"
            ]

            [global]
            targets = []
            skills = []
        "#;

        // When
        let config: Config = toml::from_str(toml).unwrap();

        // Then
        assert_eq!(config.sources.skills.len(), 3);
        assert_eq!(
            config.sources.skills[0],
            PathBuf::from("/home/user/.config/loadout/skills")
        );
        assert_eq!(
            config.sources.skills[1],
            PathBuf::from("/opt/shared-skills")
        );
        assert_eq!(
            config.sources.skills[2],
            PathBuf::from("/home/user/projects/team-skills")
        );
    }

    #[test]
    fn should_handle_multiple_targets() {
        // Given
        let toml = r#"
            [sources]
            skills = []

            [global]
            targets = [
                "/home/user/.claude/skills",
                "/home/user/.config/opencode/skills",
                "/home/user/.agents/skills"
            ]
            skills = []
        "#;

        // When
        let config: Config = toml::from_str(toml).unwrap();

        // Then
        assert_eq!(config.global.targets.len(), 3);
        assert_eq!(
            config.global.targets[0],
            PathBuf::from("/home/user/.claude/skills")
        );
        assert_eq!(
            config.global.targets[1],
            PathBuf::from("/home/user/.config/opencode/skills")
        );
        assert_eq!(
            config.global.targets[2],
            PathBuf::from("/home/user/.agents/skills")
        );
    }
}
