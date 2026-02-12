//! New command implementation

use std::fs;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::Config;

const TEMPLATE_CONTENT: &str = r#"---
name: {name}
description: >-
  {description}
# tags: []
---

# {name}

TODO: Add your skill content here.

## Instructions

Write your instructions for the AI agent. You can use:
- Plain text
- Markdown formatting
- Code blocks
- XML structures (optional)

## Example

```xml
<skill>
  <goal>Describe what this skill helps accomplish.</goal>
  
  <constraints>
    <constraint name="example">A rule that must always be followed</constraint>
  </constraints>
  
  <procedures>
    <procedure name="main-workflow">
      // Pseudocode or natural language describing the workflow
      def handle_request(input):
        // Process the input
        // Return result
    </procedure>
  </procedures>
</skill>
```
"#;

/// Create a new skill from template
pub fn new(config: &Config, name: String, description: Option<String>) -> Result<()> {
    // Validate skill name format
    validate_skill_name(&name)?;

    // Use first source directory as target
    let source_dir = config
        .sources
        .skills
        .first()
        .context("No source directories configured")?;

    let skill_dir = source_dir.join(&name);

    // Check if skill already exists
    if skill_dir.exists() {
        return Err(anyhow::anyhow!(
            "Skill directory already exists: {}",
            skill_dir.display()
        ));
    }

    // Create skill directory
    fs::create_dir_all(&skill_dir).context(format!(
        "Failed to create skill directory: {}",
        skill_dir.display()
    ))?;

    // Generate SKILL.md content
    let desc = description.unwrap_or_else(|| format!("Description for {}", name));
    let content = TEMPLATE_CONTENT
        .replace("{name}", &name)
        .replace("{description}", &desc);

    // Write SKILL.md file
    let skill_file = skill_dir.join("SKILL.md");
    fs::write(&skill_file, content).context(format!(
        "Failed to write SKILL.md: {}",
        skill_file.display()
    ))?;

    println!("{} {}", "Created skill:".green().bold(), name);
    println!("  Path: {}", skill_dir.display());
    println!("  File: {}", skill_file.display());
    println!();
    println!("Next steps:");
    println!("  1. Edit {}", skill_file.display().to_string().cyan());
    println!("  2. Add '{}' to loadout.toml [global] skills", name.cyan());
    println!("  3. Run {} to link it", "loadout install".cyan());

    Ok(())
}

/// Validate skill name follows the pattern: ^[a-z0-9]+(-[a-z0-9]+)*$
fn validate_skill_name(name: &str) -> Result<()> {
    let re = regex::Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap();

    if !re.is_match(name) {
        return Err(anyhow::anyhow!(
            "Invalid skill name '{}'. Must be lowercase alphanumeric with hyphens only (e.g., my-skill-name)",
            name
        ));
    }

    if name.is_empty() || name.len() > 64 {
        return Err(anyhow::anyhow!(
            "Invalid skill name length: {}. Must be 1-64 characters",
            name.len()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Global, Sources};
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_config(temp: &TempDir) -> Config {
        Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        }
    }

    #[test]
    fn should_create_new_skill() {
        // Given
        let temp = TempDir::new().unwrap();
        let config = create_test_config(&temp);

        // When
        new(
            &config,
            "my-skill".to_string(),
            Some("Test skill".to_string()),
        )
        .unwrap();

        // Then
        let skill_dir = temp.path().join("skills/my-skill");
        assert!(skill_dir.exists());
        assert!(skill_dir.join("SKILL.md").exists());

        let content = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        assert!(content.contains("name: my-skill"));
        assert!(content.contains("Test skill"));
    }

    #[test]
    fn should_create_skill_with_default_description() {
        // Given
        let temp = TempDir::new().unwrap();
        let config = create_test_config(&temp);

        // When
        new(&config, "my-skill".to_string(), None).unwrap();

        // Then
        let skill_file = temp.path().join("skills/my-skill/SKILL.md");
        let content = fs::read_to_string(skill_file).unwrap();
        assert!(content.contains("Description for my-skill"));
    }

    #[test]
    fn should_return_error_when_skill_exists() {
        // Given
        let temp = TempDir::new().unwrap();
        let config = create_test_config(&temp);

        fs::create_dir_all(temp.path().join("skills/my-skill")).unwrap();

        // When
        let result = new(&config, "my-skill".to_string(), None);

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn should_validate_skill_name_pattern() {
        // Given - valid names
        assert!(validate_skill_name("my-skill").is_ok());
        assert!(validate_skill_name("skill").is_ok());
        assert!(validate_skill_name("skill-123").is_ok());

        // Given - invalid names
        assert!(validate_skill_name("My-Skill").is_err());
        assert!(validate_skill_name("my_skill").is_err());
        assert!(validate_skill_name("my--skill").is_err());
        assert!(validate_skill_name("-my-skill").is_err());
        assert!(validate_skill_name("my-skill-").is_err());
    }

    #[test]
    fn should_validate_skill_name_length() {
        // Given - name too long
        let long_name = "a".repeat(65);

        // When
        let result = validate_skill_name(&long_name);

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("length"));
    }
}
