//! Skill discovery, resolution, and frontmatter validation

pub mod frontmatter;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

pub use frontmatter::Frontmatter;

const SKILL_FILE_NAME: &str = "SKILL.md";

/// Errors that can occur during skill resolution
#[derive(Error, Debug)]
pub enum SkillError {
    #[error("Skill '{0}' not found in any source directory")]
    NotFound(String),

    #[error("No SKILL.md found in skill directory: {0}")]
    MissingSkillFile(PathBuf),

    #[error("Failed to walk directory {path}: {source}")]
    WalkError {
        path: PathBuf,
        source: walkdir::Error,
    },
}

/// A discovered skill with its metadata
#[derive(Debug, Clone)]
pub struct Skill {
    /// The skill name (from frontmatter)
    pub name: String,

    /// Path to the skill directory (containing SKILL.md)
    pub path: PathBuf,

    /// Path to the SKILL.md file
    pub skill_file: PathBuf,

    /// Parsed frontmatter
    pub frontmatter: Frontmatter,
}

impl Skill {
    /// Load a skill from a directory containing SKILL.md
    pub fn from_directory(path: &Path) -> Result<Self> {
        let skill_file = path.join(SKILL_FILE_NAME);

        if !skill_file.exists() {
            return Err(SkillError::MissingSkillFile(path.to_path_buf()).into());
        }

        let frontmatter = Frontmatter::from_file(&skill_file)?;

        // Validate that the directory name matches the skill name
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            frontmatter.validate_directory_name(dir_name)?;
        }

        Ok(Skill {
            name: frontmatter.name.clone(),
            path: path.to_path_buf(),
            skill_file,
            frontmatter,
        })
    }
}

/// Walk source directories to discover all skills
///
/// Skills are discovered by recursively walking each source directory
/// looking for directories containing SKILL.md files.
pub fn discover_all(sources: &[PathBuf]) -> Result<Vec<Skill>> {
    let mut skills = Vec::new();

    for source in sources {
        let discovered = discover_in_directory(source)?;
        skills.extend(discovered);
    }

    Ok(skills)
}

/// Discover skills within a single source directory
fn discover_in_directory(source: &Path) -> Result<Vec<Skill>> {
    if !source.exists() {
        // Silently skip non-existent sources
        return Ok(Vec::new());
    }

    let mut skills = Vec::new();

    let walker = WalkDir::new(source)
        .follow_links(true)
        .into_iter()
        .filter_entry(is_not_hidden);

    for entry in walker {
        let entry = entry.map_err(|e| SkillError::WalkError {
            path: source.to_path_buf(),
            source: e,
        })?;

        if is_skill_file(&entry) {
            if let Some(skill_dir) = entry.path().parent() {
                match Skill::from_directory(skill_dir) {
                    Ok(skill) => skills.push(skill),
                    Err(e) => {
                        // Log error but continue discovering other skills
                        eprintln!(
                            "Warning: Failed to load skill from {}: {}",
                            skill_dir.display(),
                            e
                        );
                    }
                }
            }
        }
    }

    Ok(skills)
}

/// Resolve a skill by name from source directories
///
/// Searches sources in order and returns the first match.
pub fn resolve(sources: &[PathBuf], name: &str) -> Result<Skill> {
    for source in sources {
        if let Some(skill) = find_in_directory(source, name)? {
            return Ok(skill);
        }
    }

    Err(SkillError::NotFound(name.to_string()).into())
}

/// Find a skill by name within a single source directory
fn find_in_directory(source: &Path, name: &str) -> Result<Option<Skill>> {
    if !source.exists() {
        return Ok(None);
    }

    let walker = WalkDir::new(source)
        .follow_links(true)
        .into_iter()
        .filter_entry(is_not_hidden);

    for entry in walker {
        let entry = entry.map_err(|e| SkillError::WalkError {
            path: source.to_path_buf(),
            source: e,
        })?;

        if is_skill_file(&entry) {
            if let Some(skill_dir) = entry.path().parent() {
                if let Some(dir_name) = skill_dir.file_name().and_then(|n| n.to_str()) {
                    if dir_name == name {
                        return Ok(Some(Skill::from_directory(skill_dir)?));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Build a map of skill names to Skill objects
pub fn build_skill_map(skills: Vec<Skill>) -> HashMap<String, Skill> {
    skills.into_iter().map(|s| (s.name.clone(), s)).collect()
}

/// Check if a directory entry is a SKILL.md file
fn is_skill_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
        && entry
            .file_name()
            .to_str()
            .map(|s| s == SKILL_FILE_NAME)
            .unwrap_or(false)
}

/// Filter out hidden directories (starting with .)
fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| !s.starts_with('.'))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn should_load_skill_from_directory() {
        // Given
        let skill_dir = PathBuf::from("tests/fixtures/skills/test-skill");

        // When
        let skill = Skill::from_directory(&skill_dir).unwrap();

        // Then
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.path, skill_dir);
        assert!(skill.skill_file.ends_with("SKILL.md"));
    }

    #[test]
    fn should_return_error_when_directory_missing_skill_file() {
        // Given
        let skill_dir = PathBuf::from("tests/fixtures/skills/category");

        // When
        let result = Skill::from_directory(&skill_dir);

        // Then
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No SKILL.md found"));
    }

    #[test]
    fn should_discover_all_skills_in_directory() {
        // Given
        let source = PathBuf::from("tests/fixtures/skills");

        // When
        let skills = discover_in_directory(&source).unwrap();

        // Then
        assert_eq!(skills.len(), 3);
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"test-skill"));
        assert!(names.contains(&"another-skill"));
        assert!(names.contains(&"nested-skill"));
    }

    #[test]
    fn should_discover_nested_skills() {
        // Given
        let source = PathBuf::from("tests/fixtures/skills");

        // When
        let skills = discover_in_directory(&source).unwrap();

        // Then
        let nested = skills.iter().find(|s| s.name == "nested-skill");
        assert!(nested.is_some());
        let nested = nested.unwrap();
        assert!(nested.path.ends_with("category/nested-skill"));
    }

    #[test]
    fn should_resolve_skill_from_first_matching_source() {
        // Given
        let sources = vec![
            PathBuf::from("tests/fixtures/skills"),
            PathBuf::from("tests/fixtures/other-skills"),
        ];

        // When
        let skill = resolve(&sources, "test-skill").unwrap();

        // Then
        assert_eq!(skill.name, "test-skill");
        assert!(skill.path.starts_with("tests/fixtures/skills"));
    }

    #[test]
    fn should_return_error_when_skill_not_found() {
        // Given
        let sources = vec![PathBuf::from("tests/fixtures/skills")];

        // When
        let result = resolve(&sources, "nonexistent-skill");

        // Then
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
        assert!(err.to_string().contains("nonexistent-skill"));
    }

    #[test]
    fn should_handle_nonexistent_source_directory() {
        // Given
        let source = PathBuf::from("/nonexistent/directory");

        // When
        let skills = discover_in_directory(&source).unwrap();

        // Then
        assert_eq!(skills.len(), 0);
    }

    #[test]
    fn should_discover_all_skills_from_multiple_sources() {
        // Given
        let sources = vec![
            PathBuf::from("tests/fixtures/skills"),
            PathBuf::from("/nonexistent/source"),
        ];

        // When
        let skills = discover_all(&sources).unwrap();

        // Then
        assert!(skills.len() >= 3);
    }

    #[test]
    fn should_build_skill_map() {
        // Given
        let source = PathBuf::from("tests/fixtures/skills");
        let skills = discover_in_directory(&source).unwrap();

        // When
        let skill_map = build_skill_map(skills);

        // Then
        assert!(skill_map.contains_key("test-skill"));
        assert!(skill_map.contains_key("another-skill"));
        assert!(skill_map.contains_key("nested-skill"));
        assert_eq!(skill_map.len(), 3);
    }

    #[test]
    fn should_find_skill_by_name_in_directory() {
        // Given
        let source = PathBuf::from("tests/fixtures/skills");

        // When
        let result = find_in_directory(&source, "nested-skill").unwrap();

        // Then
        assert!(result.is_some());
        let skill = result.unwrap();
        assert_eq!(skill.name, "nested-skill");
    }

    #[test]
    fn should_return_none_when_skill_not_in_directory() {
        // Given
        let source = PathBuf::from("tests/fixtures/skills");

        // When
        let result = find_in_directory(&source, "nonexistent").unwrap();

        // Then
        assert!(result.is_none());
    }
}
