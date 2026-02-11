//! Symlink creation, removal, and marker management

use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use thiserror::Error;

const MARKER_FILE_NAME: &str = ".managed-by-loadout";

/// Errors that can occur during linking operations
#[derive(Error, Debug)]
pub enum LinkerError {
    #[error("Target {0} exists and is not managed by loadout")]
    UnmanagedTarget(PathBuf),

    #[error("Symlink already exists: {0}")]
    SymlinkExists(PathBuf),
}

/// Create a symlink from source skill directory to target location
///
/// This function:
/// - Creates the target directory if it doesn't exist
/// - Creates a marker file to indicate the directory is managed
/// - Creates the symlink if it doesn't already exist
/// - Skips if the symlink already exists and points to the correct source
pub fn link_skill(skill_name: &str, skill_path: &Path, target_dir: &Path) -> Result<()> {
    // Create target directory if it doesn't exist
    fs::create_dir_all(target_dir).context(format!(
        "Failed to create target directory: {}",
        target_dir.display()
    ))?;

    // Create marker file
    create_marker(target_dir)?;

    // Create symlink
    let link_path = target_dir.join(skill_name);

    // Check if symlink already exists
    if link_path.exists() || link_path.is_symlink() {
        // Check if it's a symlink pointing to the correct location
        if link_path.is_symlink() {
            let current_target = fs::read_link(&link_path)
                .context(format!("Failed to read symlink: {}", link_path.display()))?;
            if current_target == skill_path {
                // Symlink already correct, nothing to do
                return Ok(());
            }
        }

        // If it's a directory or file that we didn't create, error
        if !is_managed(target_dir) {
            return Err(LinkerError::UnmanagedTarget(link_path).into());
        }

        // Remove and recreate the symlink
        remove_symlink(&link_path)?;
    }

    // Create the symlink
    unix_fs::symlink(skill_path, &link_path)
        .context(format!("Failed to create symlink: {}", link_path.display()))?;

    Ok(())
}

/// Remove all managed symlinks from a target directory
pub fn clean_target(target_dir: &Path) -> Result<Vec<PathBuf>> {
    if !is_managed(target_dir) {
        // Not a managed directory, nothing to do
        return Ok(Vec::new());
    }

    let mut removed = Vec::new();

    // Read all entries in the target directory
    if target_dir.exists() && target_dir.is_dir() {
        for entry in fs::read_dir(target_dir).context(format!(
            "Failed to read directory: {}",
            target_dir.display()
        ))? {
            let entry = entry?;
            let path = entry.path();

            // Skip the marker file itself
            if path.file_name().and_then(|n| n.to_str()) == Some(MARKER_FILE_NAME) {
                continue;
            }

            // Remove symlinks
            if path.is_symlink() {
                remove_symlink(&path)?;
                removed.push(path);
            }
        }
    }

    // Remove marker file
    remove_marker(target_dir)?;

    // Remove directory if it's empty
    if is_directory_empty(target_dir)? {
        fs::remove_dir(target_dir).context(format!(
            "Failed to remove empty directory: {}",
            target_dir.display()
        ))?;
    }

    Ok(removed)
}

/// Create a marker file in the target directory
fn create_marker(target_dir: &Path) -> Result<()> {
    let marker_path = target_dir.join(MARKER_FILE_NAME);

    if !marker_path.exists() {
        fs::write(&marker_path, "").context(format!(
            "Failed to create marker file: {}",
            marker_path.display()
        ))?;
    }

    Ok(())
}

/// Remove the marker file from a target directory
fn remove_marker(target_dir: &Path) -> Result<()> {
    let marker_path = target_dir.join(MARKER_FILE_NAME);

    if marker_path.exists() {
        fs::remove_file(&marker_path).context(format!(
            "Failed to remove marker file: {}",
            marker_path.display()
        ))?;
    }

    Ok(())
}

/// Check if a target directory is managed by loadout
pub fn is_managed(target_dir: &Path) -> bool {
    target_dir.join(MARKER_FILE_NAME).exists()
}

/// Remove a symlink
fn remove_symlink(path: &Path) -> Result<()> {
    fs::remove_file(path).context(format!("Failed to remove symlink: {}", path.display()))?;
    Ok(())
}

/// Check if a directory is empty (or only contains the marker file)
fn is_directory_empty(dir: &Path) -> Result<bool> {
    let entries: Vec<_> = fs::read_dir(dir)?.collect();
    Ok(entries.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn should_create_symlink_to_skill() {
        // Given
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("skill-source");
        let target_dir = temp.path().join("target");

        fs::create_dir(&skill_dir).unwrap();

        // When
        link_skill("my-skill", &skill_dir, &target_dir).unwrap();

        // Then
        let link_path = target_dir.join("my-skill");
        assert!(link_path.exists());
        assert!(link_path.is_symlink());
        let link_target = fs::read_link(&link_path).unwrap();
        assert_eq!(link_target, skill_dir);
    }

    #[test]
    fn should_create_marker_file() {
        // Given
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("skill-source");
        let target_dir = temp.path().join("target");

        fs::create_dir(&skill_dir).unwrap();

        // When
        link_skill("my-skill", &skill_dir, &target_dir).unwrap();

        // Then
        let marker_path = target_dir.join(MARKER_FILE_NAME);
        assert!(marker_path.exists());
        assert!(is_managed(&target_dir));
    }

    #[test]
    fn should_not_recreate_existing_correct_symlink() {
        // Given
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("skill-source");
        let target_dir = temp.path().join("target");

        fs::create_dir(&skill_dir).unwrap();
        link_skill("my-skill", &skill_dir, &target_dir).unwrap();

        let link_path = target_dir.join("my-skill");
        let original_link = fs::read_link(&link_path).unwrap();

        // When - link again
        link_skill("my-skill", &skill_dir, &target_dir).unwrap();

        // Then - symlink unchanged
        assert_eq!(fs::read_link(&link_path).unwrap(), original_link);
    }

    #[test]
    fn should_update_symlink_when_target_changes() {
        // Given
        let temp = TempDir::new().unwrap();
        let skill_dir_1 = temp.path().join("skill-source-1");
        let skill_dir_2 = temp.path().join("skill-source-2");
        let target_dir = temp.path().join("target");

        fs::create_dir(&skill_dir_1).unwrap();
        fs::create_dir(&skill_dir_2).unwrap();

        link_skill("my-skill", &skill_dir_1, &target_dir).unwrap();

        // When - link to different source
        link_skill("my-skill", &skill_dir_2, &target_dir).unwrap();

        // Then - symlink points to new location
        let link_path = target_dir.join("my-skill");
        let link_target = fs::read_link(&link_path).unwrap();
        assert_eq!(link_target, skill_dir_2);
    }

    #[test]
    fn should_clean_all_symlinks_from_managed_directory() {
        // Given
        let temp = TempDir::new().unwrap();
        let skill_dir_1 = temp.path().join("skill-1");
        let skill_dir_2 = temp.path().join("skill-2");
        let target_dir = temp.path().join("target");

        fs::create_dir(&skill_dir_1).unwrap();
        fs::create_dir(&skill_dir_2).unwrap();

        link_skill("skill-1", &skill_dir_1, &target_dir).unwrap();
        link_skill("skill-2", &skill_dir_2, &target_dir).unwrap();

        // When
        let removed = clean_target(&target_dir).unwrap();

        // Then
        assert_eq!(removed.len(), 2);
        assert!(!target_dir.join("skill-1").exists());
        assert!(!target_dir.join("skill-2").exists());
        assert!(!is_managed(&target_dir));
    }

    #[test]
    fn should_remove_empty_directory_after_cleaning() {
        // Given
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("skill-source");
        let target_dir = temp.path().join("target");

        fs::create_dir(&skill_dir).unwrap();
        link_skill("my-skill", &skill_dir, &target_dir).unwrap();

        // When
        clean_target(&target_dir).unwrap();

        // Then
        assert!(!target_dir.exists());
    }

    #[test]
    fn should_not_remove_non_empty_directory_after_cleaning() {
        // Given
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("skill-source");
        let target_dir = temp.path().join("target");

        fs::create_dir(&skill_dir).unwrap();
        link_skill("my-skill", &skill_dir, &target_dir).unwrap();

        // Add a regular file
        fs::write(target_dir.join("other-file.txt"), "content").unwrap();

        // When
        clean_target(&target_dir).unwrap();

        // Then - directory still exists because of the other file
        assert!(target_dir.exists());
        assert!(target_dir.join("other-file.txt").exists());
        assert!(!target_dir.join("my-skill").exists());
    }

    #[test]
    fn should_return_empty_vec_when_cleaning_unmanaged_directory() {
        // Given
        let temp = TempDir::new().unwrap();
        let target_dir = temp.path().join("unmanaged");
        fs::create_dir(&target_dir).unwrap();

        // When
        let removed = clean_target(&target_dir).unwrap();

        // Then
        assert_eq!(removed.len(), 0);
        assert!(target_dir.exists());
    }

    #[test]
    fn should_detect_managed_directory() {
        // Given
        let temp = TempDir::new().unwrap();
        let target_dir = temp.path().join("target");
        fs::create_dir(&target_dir).unwrap();

        // When - before marker
        assert!(!is_managed(&target_dir));

        // When - after marker
        create_marker(&target_dir).unwrap();
        assert!(is_managed(&target_dir));
    }
}
