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

    /// Target path templates keyed by runner alias
    #[serde(default = "default_target_aliases")]
    pub target_aliases: HashMap<String, TargetAliasPaths>,

    /// Per-project overrides (keyed by project path)
    #[serde(default)]
    pub projects: HashMap<PathBuf, Project>,

    /// Check command configuration
    #[serde(default)]
    pub check: CheckConfig,
}

/// Configuration for the check command
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckConfig {
    /// Patterns to suppress: "check-type:source:detail"
    /// e.g., "dangling:skill-format:related-skill"
    #[serde(default)]
    pub ignore: Vec<String>,
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
    /// Target aliases where skills will be symlinked
    pub targets: Vec<String>,

    /// Skills to enable globally
    pub skills: Vec<String>,
}

/// Target paths for a runner alias
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetAliasPaths {
    /// Global-scope target path for this alias
    pub global: PathBuf,

    /// Project-scope target path for this alias
    pub project: PathBuf,
}

/// Project-specific skill configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Skills to enable for this project
    pub skills: Vec<String>,

    /// Whether to include global skills (default: true)
    #[serde(default = "default_inherit")]
    pub inherit: bool,

    /// Optional target alias override for this project
    #[serde(default)]
    pub targets: Option<Vec<String>>,
}

pub fn default_target_aliases() -> HashMap<String, TargetAliasPaths> {
    HashMap::from([
        (
            "claude_code".to_string(),
            TargetAliasPaths {
                global: PathBuf::from("~/.claude/skills"),
                project: PathBuf::from(".claude/skills"),
            },
        ),
        (
            "opencode".to_string(),
            TargetAliasPaths {
                global: PathBuf::from("~/.config/opencode/skills"),
                project: PathBuf::from(".opencode/skills"),
            },
        ),
        (
            "codex".to_string(),
            TargetAliasPaths {
                global: PathBuf::from("~/.agents/skills"),
                project: PathBuf::from(".agents/skills"),
            },
        ),
    ])
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
            targets = ["claude_code"]
            skills = ["my-skill"]
        "#;

        // When
        let config: Config = toml::from_str(toml).unwrap();

        // Then
        assert_eq!(config.sources.skills.len(), 1);
        assert_eq!(config.global.targets.len(), 1);
        assert_eq!(config.global.targets[0], "claude_code");
        assert_eq!(config.global.skills.len(), 1);
        assert_eq!(config.global.skills[0], "my-skill");
        assert!(config.projects.is_empty());
        assert_eq!(config.target_aliases.len(), 3);
    }

    #[test]
    fn should_deserialize_config_with_projects() {
        // Given
        let toml = r#"
            [sources]
            skills = ["/home/user/.config/loadout/skills"]

            [global]
            targets = ["claude_code"]
            skills = ["global-skill"]

            [projects."/home/user/my-project"]
            skills = ["project-skill"]
            inherit = false
            targets = ["opencode", "codex"]
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
        assert_eq!(
            project.targets.as_ref().unwrap(),
            &vec!["opencode".to_string(), "codex".to_string()]
        );
    }

    #[test]
    fn should_default_project_inherit_to_true() {
        // Given
        let toml = r#"
            [sources]
            skills = ["/home/user/.config/loadout/skills"]

            [global]
            targets = ["claude_code"]
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
        assert!(project.targets.is_none());
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
            targets = ["claude_code", "opencode", "codex"]
            skills = []
        "#;

        // When
        let config: Config = toml::from_str(toml).unwrap();

        // Then
        assert_eq!(config.global.targets.len(), 3);
        assert_eq!(config.global.targets[0], "claude_code");
        assert_eq!(config.global.targets[1], "opencode");
        assert_eq!(config.global.targets[2], "codex");
    }

    #[test]
    fn should_allow_custom_target_aliases() {
        // Given
        let toml = r#"
            [sources]
            skills = []

            [global]
            targets = ["my_runner"]
            skills = []

            [target_aliases.my_runner]
            global = "~/.my-runner/skills"
            project = ".my-runner/skills"
        "#;

        // When
        let config: Config = toml::from_str(toml).unwrap();

        // Then
        assert!(config.target_aliases.contains_key("my_runner"));
        assert_eq!(config.global.targets, vec!["my_runner".to_string()]);
    }
}
