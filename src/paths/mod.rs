//! Shared path policies and helpers.

use std::path::{Path, PathBuf};

/// Tool discovery subdirectories under a project root.
pub const PROJECT_DISCOVERY_SUBDIRS: &[&str] =
    &[".claude/skills", ".opencode/skills", ".agents/skills"];

/// Build all project-local discovery targets for a project path.
pub fn project_targets(project_path: &Path) -> Vec<PathBuf> {
    PROJECT_DISCOVERY_SUBDIRS
        .iter()
        .map(|subdir| project_path.join(subdir))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_project_targets_in_expected_order() {
        // Given
        let project_path = PathBuf::from("project-root");

        // When
        let targets = project_targets(&project_path);

        // Then
        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0], project_path.join(".claude/skills"));
        assert_eq!(targets[1], project_path.join(".opencode/skills"));
        assert_eq!(targets[2], project_path.join(".agents/skills"));
    }
}
