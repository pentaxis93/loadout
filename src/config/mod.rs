//! Configuration loading and path resolution

mod types;

pub use types::{Config, Global, Project, Sources};

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

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

    // Expand ~ in all path fields
    expand_paths(&mut config)?;

    Ok(config)
}

/// Resolve the config file path using environment variables and XDG conventions
fn resolve_config_path() -> Result<PathBuf> {
    let loadout_config = env::var("LOADOUT_CONFIG").ok();
    let xdg_home = env::var("XDG_CONFIG_HOME").ok();
    let home = env::var("HOME").ok();

    resolve_config_path_from_env(loadout_config.as_deref(), xdg_home.as_deref(), home.as_deref())
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

/// Expand ~ in all path fields within the config
fn expand_paths(config: &mut Config) -> Result<()> {
    // Expand source paths
    for source in &mut config.sources.skills {
        let path_str = source
            .to_str()
            .ok_or_else(|| anyhow!("sources.skills contains non-UTF-8 path"))?;
        *source = expand_tilde(path_str)?;
    }

    // Expand global target paths
    for target in &mut config.global.targets {
        let path_str = target
            .to_str()
            .ok_or_else(|| anyhow!("global.targets contains non-UTF-8 path"))?;
        *target = expand_tilde(path_str)?;
    }

    // Expand project paths (both keys and target paths if they exist)
    let project_keys: Vec<PathBuf> = config.projects.keys().cloned().collect();
    for old_key in project_keys {
        let key_str = old_key
            .to_str()
            .ok_or_else(|| anyhow!("projects contains non-UTF-8 path key"))?;
        let new_key = expand_tilde(key_str)?;
        if new_key != old_key {
            if let Some(project) = config.projects.remove(&old_key) {
                config.projects.insert(new_key, project);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let resolved = resolve_config_path_from_env(None, Some("/xdg/config"), Some("/home/test"))
            .unwrap();

        // Then
        assert_eq!(
            resolved,
            PathBuf::from("/xdg/config").join("loadout").join("loadout.toml")
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
            targets = ["~/.claude/skills"]
            skills = []

            [projects."~/my-project"]
            skills = []
        "#;

        // When
        let mut config: Config = toml::from_str(toml).unwrap();
        expand_paths(&mut config).unwrap();

        // Then
        assert_eq!(
            config.sources.skills[0],
            PathBuf::from(&home).join(".config/loadout/skills")
        );
        assert_eq!(config.sources.skills[1], PathBuf::from("/opt/skills"));
        assert_eq!(
            config.global.targets[0],
            PathBuf::from(&home).join(".claude/skills")
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
        writeln!(temp_file, "invalid toml content [[[").unwrap();
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
                targets: vec![],
                skills: vec![],
            },
            projects: Default::default(),
            check: Default::default(),
        };

        // When
        let result = expand_paths(&mut config);

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-UTF-8 path"));
    }
}
