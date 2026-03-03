//! Configuration loading and path resolution

mod types;

pub use types::{
    default_target_aliases, CheckConfig, Config, Global, Project, Sources, TargetAliasPaths,
};

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

/// Load configuration from the standard location
///
/// Resolution order:
/// 1. $LOADOUT_CONFIG (if set)
/// 2. $XDG_CONFIG_HOME/loadout/loadout.toml (if set)
/// 3. ~/.config/loadout/loadout.toml (default)
pub fn load() -> Result<Config> {
    let path = resolve_config_path()?;
    load_from(&path)
}

/// Load configuration from a specific path
pub fn load_from(path: &Path) -> Result<Config> {
    let contents = fs::read_to_string(path)
        .context(format!("Failed to read config file: {}", path.display()))?;

    let mut config: Config = toml::from_str(&contents)
        .context(format!("Failed to parse config file: {}", path.display()))?;

    merge_default_target_aliases(&mut config);

    let config_dir = path.parent().context(format!(
        "Config file has no parent directory: {}",
        path.display()
    ))?;
    let config_dir = if config_dir.is_absolute() {
        config_dir.to_path_buf()
    } else {
        env::current_dir()
            .context("Failed to resolve current working directory")?
            .join(config_dir)
    };

    // Expand path fields
    expand_paths(&mut config, &config_dir)?;

    // Validate aliases and references after expansion.
    validate_aliases(&config)?;

    Ok(config)
}

/// Resolve the config file path using environment variables and XDG conventions
fn resolve_config_path() -> Result<PathBuf> {
    let loadout_config = env::var("LOADOUT_CONFIG").ok();
    let xdg_home = env::var("XDG_CONFIG_HOME").ok();
    let home = env::var("HOME").ok();

    resolve_config_path_from_env(
        loadout_config.as_deref(),
        xdg_home.as_deref(),
        home.as_deref(),
    )
}

fn resolve_config_path_from_env(
    loadout_config: Option<&str>,
    xdg_home: Option<&str>,
    home: Option<&str>,
) -> Result<PathBuf> {
    if let Some(path) = loadout_config {
        return expand_tilde_with_home(path, home);
    }

    if let Some(xdg_home) = xdg_home {
        return Ok(PathBuf::from(xdg_home).join("loadout").join("loadout.toml"));
    }

    let home = home.context("HOME environment variable not set")?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("loadout")
        .join("loadout.toml"))
}

/// Expand ~ and ~/ to $HOME in a path string
fn expand_tilde(path: &str) -> Result<PathBuf> {
    let home = env::var("HOME").ok();
    expand_tilde_with_home(path, home.as_deref())
}

fn expand_tilde_with_home(path: &str, home: Option<&str>) -> Result<PathBuf> {
    if let Some(stripped) = path.strip_prefix("~/") {
        let home = home.context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(stripped))
    } else if path == "~" {
        let home = home.context("HOME environment variable not set")?;
        Ok(PathBuf::from(home))
    } else {
        Ok(PathBuf::from(path))
    }
}

/// Expand path fields within the config.
fn expand_paths(config: &mut Config, config_dir: &Path) -> Result<()> {
    // Expand source paths
    for source in &mut config.sources.skills {
        *source = expand_config_path(source, config_dir, "sources.skills")?;
    }

    // Expand target alias paths
    for (alias, paths) in &mut config.target_aliases {
        let global_field = format!("target_aliases.{alias}.global");
        let project_field = format!("target_aliases.{alias}.project");
        paths.global = expand_config_path(&paths.global, config_dir, &global_field)?;
        paths.project = expand_tilde_path(&paths.project, &project_field)?;
    }

    // Expand project paths (keys)
    let project_keys: Vec<PathBuf> = config.projects.keys().cloned().collect();
    for old_key in project_keys {
        let new_key = expand_config_path(&old_key, config_dir, "projects path key")?;
        if new_key != old_key {
            if let Some(project) = config.projects.remove(&old_key) {
                config.projects.insert(new_key, project);
            }
        }
    }

    Ok(())
}

fn expand_config_path(path: &Path, config_dir: &Path, field_name: &str) -> Result<PathBuf> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow!("{field_name} contains non-UTF-8 path"))?;
    let expanded = expand_tilde(path_str)?;
    if expanded.is_relative() {
        Ok(config_dir.join(expanded))
    } else {
        Ok(expanded)
    }
}

fn expand_tilde_path(path: &Path, field_name: &str) -> Result<PathBuf> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow!("{field_name} contains non-UTF-8 path"))?;
    expand_tilde(path_str)
}

fn merge_default_target_aliases(config: &mut Config) {
    for (alias, paths) in default_target_aliases() {
        config.target_aliases.entry(alias).or_insert(paths);
    }
}

fn validate_aliases(config: &Config) -> Result<()> {
    for alias in config.target_aliases.keys() {
        if !is_valid_alias_name(alias) {
            anyhow::bail!("Invalid target alias '{alias}'. Alias names must match ^[a-z0-9_]+$");
        }
    }

    for alias in &config.global.targets {
        ensure_alias_exists(config, alias, "global.targets")?;
    }

    for (project_path, project) in &config.projects {
        if let Some(targets) = &project.targets {
            for alias in targets {
                ensure_alias_exists(
                    config,
                    alias,
                    &format!("projects.\"{}\".targets", project_path.display()),
                )?;
            }
        }
    }

    Ok(())
}

fn ensure_alias_exists(config: &Config, alias: &str, field_name: &str) -> Result<()> {
    if config.target_aliases.contains_key(alias) {
        return Ok(());
    }

    anyhow::bail!(
        "Unknown target alias '{alias}' in {field_name}. Define it under [target_aliases.{alias}]"
    )
}

fn is_valid_alias_name(alias: &str) -> bool {
    !alias.is_empty()
        && alias
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::env;

    #[test]
    fn should_expand_tilde_to_home() {
        // Given
        let home = env::var("HOME").unwrap();

        // When
        let expanded = expand_tilde("~/test/path").unwrap();

        // Then
        assert_eq!(expanded, PathBuf::from(home).join("test/path"));
    }

    #[test]
    fn should_expand_bare_tilde_to_home() {
        // Given
        let home = env::var("HOME").unwrap();

        // When
        let expanded = expand_tilde("~").unwrap();

        // Then
        assert_eq!(expanded, PathBuf::from(home));
    }

    #[test]
    fn should_not_expand_non_tilde_paths() {
        // Given
        let path = "/absolute/path";

        // When
        let expanded = expand_tilde(path).unwrap();

        // Then
        assert_eq!(expanded, PathBuf::from(path));
    }

    #[test]
    fn should_prefer_loadout_config_over_xdg_and_home() {
        // Given
        let home = "/home/test-user";

        // When
        let resolved = resolve_config_path_from_env(
            Some("~/custom/loadout.toml"),
            Some("/xdg/config"),
            Some(home),
        )
        .unwrap();

        // Then
        assert_eq!(resolved, PathBuf::from(home).join("custom/loadout.toml"));
    }

    #[test]
    fn should_use_xdg_path_when_set() {
        // When
        let resolved =
            resolve_config_path_from_env(None, Some("/xdg/config"), Some("/home/test")).unwrap();

        // Then
        assert_eq!(
            resolved,
            PathBuf::from("/xdg/config")
                .join("loadout")
                .join("loadout.toml")
        );
    }

    #[test]
    fn should_fallback_to_home_config_when_xdg_not_set() {
        // When
        let resolved = resolve_config_path_from_env(None, None, Some("/home/test")).unwrap();

        // Then
        assert_eq!(
            resolved,
            PathBuf::from("/home/test")
                .join(".config")
                .join("loadout")
                .join("loadout.toml")
        );
    }

    #[test]
    fn should_return_error_when_home_is_missing_for_default_resolution() {
        // When
        let result = resolve_config_path_from_env(None, None, None);

        // Then
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("HOME environment variable not set"));
    }

    #[test]
    fn should_expand_paths_in_config() {
        // Given
        let home = env::var("HOME").unwrap();
        let toml = r#"
            [sources]
            skills = ["~/.config/loadout/skills", "/opt/skills"]

            [global]
            targets = ["claude_code"]
            skills = []

            [projects."~/my-project"]
            skills = []

            [target_aliases.claude_code]
            global = "~/.claude/skills"
            project = ".claude/skills"
        "#;

        // When
        let mut config: Config = toml::from_str(toml).unwrap();
        expand_paths(&mut config, Path::new("/tmp")).unwrap();

        // Then
        assert_eq!(
            config.sources.skills[0],
            PathBuf::from(&home).join(".config/loadout/skills")
        );
        assert_eq!(config.sources.skills[1], PathBuf::from("/opt/skills"));
        assert_eq!(
            config.target_aliases["claude_code"].global,
            PathBuf::from(&home).join(".claude/skills")
        );
        assert_eq!(
            config.target_aliases["claude_code"].project,
            PathBuf::from(".claude/skills")
        );
        assert!(config
            .projects
            .contains_key(&PathBuf::from(&home).join("my-project")));
    }

    #[test]
    fn should_load_fixture_config() {
        // Given
        let fixture_path = PathBuf::from("tests/fixtures/loadout.toml");

        // When
        let config = load_from(&fixture_path).unwrap();

        // Then
        assert_eq!(config.sources.skills.len(), 1);
        assert_eq!(config.global.targets.len(), 1);
        assert_eq!(config.global.skills.len(), 1);
        assert_eq!(config.global.skills[0], "test-skill");
    }

    #[test]
    fn should_resolve_relative_global_alias_paths_against_config_directory() {
        // Given
        let toml = r#"
            [sources]
            skills = ["skills"]

            [global]
            targets = ["custom"]
            skills = []

            [projects."."]
            skills = []

            [target_aliases.custom]
            global = "targets/global"
            project = "targets/project"
        "#;
        let mut config: Config = toml::from_str(toml).unwrap();
        let config_dir = PathBuf::from("/tmp/loadout-config");

        // When
        expand_paths(&mut config, &config_dir).unwrap();

        // Then
        assert_eq!(config.sources.skills[0], config_dir.join("skills"));
        assert_eq!(
            config.target_aliases["custom"].global,
            config_dir.join("targets/global")
        );
        assert_eq!(
            config.target_aliases["custom"].project,
            PathBuf::from("targets/project")
        );
        assert!(config.projects.contains_key(&config_dir));
    }

    #[test]
    fn should_return_error_for_unknown_global_target_alias() {
        // Given
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[sources]
skills = []

[global]
targets = ["missing_alias"]
skills = []
"#
        )
        .unwrap();

        // When
        let result = load_from(temp_file.path());

        // Then
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown target alias 'missing_alias'"));
    }

    #[test]
    fn should_return_error_for_invalid_alias_name() {
        // Given
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[sources]
skills = []

[global]
targets = ["bad-alias"]
skills = []

[target_aliases."bad-alias"]
global = "~/.bad/skills"
project = ".bad/skills"
"#
        )
        .unwrap();

        // When
        let result = load_from(temp_file.path());

        // Then
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid target alias"));
    }

    #[test]
    fn should_merge_builtin_aliases_when_custom_aliases_defined() {
        // Given
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[sources]
skills = []

[global]
targets = ["my_runner", "codex"]
skills = []

[target_aliases.my_runner]
global = "~/.my-runner/skills"
project = ".my-runner/skills"
"#
        )
        .unwrap();

        // When
        let config = load_from(temp_file.path()).unwrap();

        // Then
        assert!(config.target_aliases.contains_key("my_runner"));
        assert!(config.target_aliases.contains_key("claude_code"));
        assert!(config.target_aliases.contains_key("opencode"));
        assert!(config.target_aliases.contains_key("codex"));
    }

    #[test]
    fn should_return_error_when_config_file_missing() {
        // Given
        let nonexistent = PathBuf::from("/nonexistent/config.toml");

        // When
        let result = load_from(&nonexistent);

        // Then
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to read config file"));
    }

    #[test]
    fn should_return_error_when_config_has_invalid_toml() {
        // Given
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "invalid toml content [[[ ").unwrap();
        let path = temp_file.path();

        // When
        let result = load_from(path);

        // Then
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse config file"));
    }

    #[cfg(unix)]
    #[test]
    fn should_return_error_when_config_contains_non_utf8_paths() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        // Given
        let mut config = Config {
            sources: Sources {
                skills: vec![PathBuf::from(OsString::from_vec(vec![
                    0x66, 0x6f, 0x80, 0x6f,
                ]))],
            },
            global: Global {
                targets: vec!["claude_code".to_string()],
                skills: vec![],
            },
            target_aliases: HashMap::from([(
                "claude_code".to_string(),
                TargetAliasPaths {
                    global: PathBuf::from("~/.claude/skills"),
                    project: PathBuf::from(".claude/skills"),
                },
            )]),
            projects: Default::default(),
            check: Default::default(),
        };

        // When
        let result = expand_paths(&mut config, Path::new("/tmp"));

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-UTF-8 path"));
    }
}
