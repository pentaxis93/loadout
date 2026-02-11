//! YAML frontmatter extraction and validation for SKILL.md files

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const NAME_PATTERN: &str = r"^[a-z0-9]+(-[a-z0-9]+)*$";
const MIN_NAME_LENGTH: usize = 1;
const MAX_NAME_LENGTH: usize = 64;
const MIN_DESCRIPTION_LENGTH: usize = 1;
const MAX_DESCRIPTION_LENGTH: usize = 1024;

/// Errors that can occur during frontmatter parsing and validation
#[derive(Error, Debug)]
pub enum FrontmatterError {
    #[error("SKILL.md does not contain YAML frontmatter delimiters (---)")]
    MissingDelimiters,

    #[error("Invalid YAML frontmatter: {0}")]
    InvalidYaml(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid skill name '{0}': must match pattern {NAME_PATTERN}")]
    InvalidNamePattern(String),

    #[error("Invalid skill name length: {0} (must be {MIN_NAME_LENGTH}-{MAX_NAME_LENGTH} chars)")]
    InvalidNameLength(usize),

    #[error("Invalid description length: {0} (must be {MIN_DESCRIPTION_LENGTH}-{MAX_DESCRIPTION_LENGTH} chars)")]
    InvalidDescriptionLength(usize),

    #[error("Skill name '{found}' does not match directory name '{expected}'")]
    NameMismatch { expected: String, found: String },
}

/// SKILL.md frontmatter
///
/// This struct represents the union of all supported frontmatter fields
/// across Claude Code and OpenCode. Only `name` and `description` are required.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    /// Skill identifier (must match directory name)
    pub name: String,

    /// What the skill does and when to use it
    pub description: String,

    // Claude Code fields (optional)
    #[serde(rename = "disable-model-invocation")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_model_invocation: Option<bool>,

    #[serde(rename = "user-invocable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_invocable: Option<bool>,

    #[serde(rename = "allowed-tools")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(rename = "argument-hint")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argument_hint: Option<String>,

    // OpenCode fields (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl Frontmatter {
    /// Parse frontmatter from a SKILL.md file
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .context(format!("Failed to read SKILL.md: {}", path.display()))?;

        Self::parse(&contents)
    }

    /// Parse frontmatter from SKILL.md content string
    pub fn parse(content: &str) -> Result<Self> {
        let yaml_content = extract_yaml(content)?;

        let frontmatter: Frontmatter = serde_yaml::from_str(&yaml_content)
            .map_err(|e| FrontmatterError::InvalidYaml(e.to_string()))?;

        frontmatter.validate()?;

        Ok(frontmatter)
    }

    /// Validate frontmatter fields
    pub fn validate(&self) -> Result<()> {
        self.validate_name()?;
        self.validate_description()?;
        Ok(())
    }

    /// Validate the skill name
    fn validate_name(&self) -> Result<()> {
        let name_len = self.name.len();
        if !(MIN_NAME_LENGTH..=MAX_NAME_LENGTH).contains(&name_len) {
            return Err(FrontmatterError::InvalidNameLength(name_len).into());
        }

        let re = Regex::new(NAME_PATTERN).unwrap();
        if !re.is_match(&self.name) {
            return Err(FrontmatterError::InvalidNamePattern(self.name.clone()).into());
        }

        Ok(())
    }

    /// Validate the description
    fn validate_description(&self) -> Result<()> {
        let desc_len = self.description.trim().len();
        if !(MIN_DESCRIPTION_LENGTH..=MAX_DESCRIPTION_LENGTH).contains(&desc_len) {
            return Err(FrontmatterError::InvalidDescriptionLength(desc_len).into());
        }

        Ok(())
    }

    /// Validate that frontmatter name matches the expected directory name
    pub fn validate_directory_name(&self, dir_name: &str) -> Result<()> {
        if self.name != dir_name {
            return Err(FrontmatterError::NameMismatch {
                expected: dir_name.to_string(),
                found: self.name.clone(),
            }
            .into());
        }
        Ok(())
    }
}

/// Extract YAML content between --- delimiters
fn extract_yaml(content: &str) -> Result<String> {
    let lines: Vec<&str> = content.lines().collect();

    // Find the first --- delimiter
    let start = lines
        .iter()
        .position(|line| line.trim() == "---")
        .ok_or(FrontmatterError::MissingDelimiters)?;

    // Find the second --- delimiter
    let end = lines
        .iter()
        .skip(start + 1)
        .position(|line| line.trim() == "---")
        .ok_or(FrontmatterError::MissingDelimiters)?;

    // Extract YAML between delimiters
    let yaml_lines = &lines[start + 1..start + 1 + end];
    Ok(yaml_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_minimal_frontmatter() {
        // Given
        let content = r#"---
name: my-skill
description: A test skill
---

# My Skill
Content here."#;

        // When
        let frontmatter = Frontmatter::parse(content).unwrap();

        // Then
        assert_eq!(frontmatter.name, "my-skill");
        assert_eq!(frontmatter.description, "A test skill");
        assert!(frontmatter.disable_model_invocation.is_none());
    }

    #[test]
    fn should_parse_frontmatter_with_optional_fields() {
        // Given
        let content = r#"---
name: my-skill
description: A test skill
disable-model-invocation: true
allowed-tools: Read, Write
license: MIT
---"#;

        // When
        let frontmatter = Frontmatter::parse(content).unwrap();

        // Then
        assert_eq!(frontmatter.name, "my-skill");
        assert_eq!(frontmatter.disable_model_invocation, Some(true));
        assert_eq!(frontmatter.allowed_tools, Some("Read, Write".to_string()));
        assert_eq!(frontmatter.license, Some("MIT".to_string()));
    }

    #[test]
    fn should_parse_multiline_description() {
        // Given
        let content = r#"---
name: my-skill
description: >-
  This is a longer description
  that spans multiple lines
  in the YAML.
---"#;

        // When
        let frontmatter = Frontmatter::parse(content).unwrap();

        // Then
        assert_eq!(frontmatter.name, "my-skill");
        assert!(frontmatter.description.contains("longer description"));
    }

    #[test]
    fn should_return_error_when_missing_delimiters() {
        // Given
        let content = "name: my-skill\ndescription: No delimiters";

        // When
        let result = Frontmatter::parse(content);

        // Then
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("delimiters"));
    }

    #[test]
    fn should_return_error_when_missing_name() {
        // Given
        let content = r#"---
description: Missing name field
---"#;

        // When
        let result = Frontmatter::parse(content);

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn should_return_error_when_missing_description() {
        // Given
        let content = r#"---
name: my-skill
---"#;

        // When
        let result = Frontmatter::parse(content);

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn should_validate_name_pattern() {
        // Given
        let valid_names = vec!["skill", "my-skill", "test-skill-123", "a", "skill-1-2-3"];
        let invalid_names = vec![
            "My-Skill",  // uppercase
            "my_skill",  // underscore
            "my--skill", // double hyphen
            "-my-skill", // starts with hyphen
            "my-skill-", // ends with hyphen
            "my skill",  // space
            "my.skill",  // dot
            "",          // empty
        ];

        // When/Then
        for name in valid_names {
            let content = format!("---\nname: {}\ndescription: test\n---", name);
            let result = Frontmatter::parse(&content);
            assert!(result.is_ok(), "Expected {} to be valid", name);
        }

        for name in invalid_names {
            let content = format!("---\nname: {}\ndescription: test\n---", name);
            let result = Frontmatter::parse(&content);
            assert!(result.is_err(), "Expected {} to be invalid", name);
        }
    }

    #[test]
    fn should_validate_name_length() {
        // Given - name too long (65 chars)
        let long_name = "a".repeat(65);
        let content = format!("---\nname: {}\ndescription: test\n---", long_name);

        // When
        let result = Frontmatter::parse(&content);

        // Then
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("length"));
    }

    #[test]
    fn should_validate_description_length() {
        // Given - description too long (1025 chars)
        let long_desc = "a".repeat(1025);
        let content = format!("---\nname: test\ndescription: {}\n---", long_desc);

        // When
        let result = Frontmatter::parse(&content);

        // Then
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("description length"));
    }

    #[test]
    fn should_detect_directory_name_mismatch() {
        // Given
        let content = r#"---
name: my-skill
description: test
---"#;
        let frontmatter = Frontmatter::parse(content).unwrap();

        // When
        let result = frontmatter.validate_directory_name("wrong-name");

        // Then
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("does not match directory name"));
        assert!(err.to_string().contains("my-skill"));
        assert!(err.to_string().contains("wrong-name"));
    }

    #[test]
    fn should_validate_matching_directory_name() {
        // Given
        let content = r#"---
name: my-skill
description: test
---"#;
        let frontmatter = Frontmatter::parse(content).unwrap();

        // When
        let result = frontmatter.validate_directory_name("my-skill");

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_extract_yaml_between_delimiters() {
        // Given
        let content = r#"---
name: test
description: value
---
Content after frontmatter"#;

        // When
        let yaml = extract_yaml(content).unwrap();

        // Then
        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("description: value"));
        assert!(!yaml.contains("Content after"));
    }

    #[test]
    fn should_parse_fixture_skill() {
        // Given
        let fixture_path = Path::new("tests/fixtures/skills/test-skill/SKILL.md");

        // When
        let frontmatter = Frontmatter::from_file(fixture_path).unwrap();

        // Then
        assert_eq!(frontmatter.name, "test-skill");
        assert!(frontmatter
            .description
            .contains("minimal test skill for integration tests"));
    }
}
