//! Symlink creation, removal, and marker management

use std::fs;
use std::path::{Component, Path, PathBuf};

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
    let canonical_target_dir = fs::canonicalize(target_dir).context(format!(
        "Failed to canonicalize target directory: {}",
        target_dir.display()
    ))?;
    let canonical_skill_path = fs::canonicalize(skill_path).context(format!(
        "Failed to canonicalize skill path: {}",
        skill_path.display()
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
            if let Ok(resolved_target) = resolve_symlink_destination(&link_path, &current_target) {
                if resolved_target == canonical_skill_path {
                    // Symlink already correct, nothing to do
                    return Ok(());
                }
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
    let link_target =
        relative_path(&canonical_target_dir, &canonical_skill_path).unwrap_or(canonical_skill_path);
    create_symlink(&link_target, &link_path)?;

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

/// List managed symlinks in a target directory that would be pruned.
pub fn preview_prune_target(
    target_dir: &Path,
    keep_skill_names: &[String],
) -> Result<Vec<PathBuf>> {
    prune_target_impl(target_dir, keep_skill_names, true)
}

/// Remove managed symlinks from a target directory except the provided skill names.
pub fn prune_target_except(target_dir: &Path, keep_skill_names: &[String]) -> Result<Vec<PathBuf>> {
    prune_target_impl(target_dir, keep_skill_names, false)
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

fn prune_target_impl(
    target_dir: &Path,
    keep_skill_names: &[String],
    dry_run: bool,
) -> Result<Vec<PathBuf>> {
    if !is_managed(target_dir) {
        return Ok(Vec::new());
    }

    let mut removed = Vec::new();

    if target_dir.exists() && target_dir.is_dir() {
        for entry in fs::read_dir(target_dir).context(format!(
            "Failed to read directory: {}",
            target_dir.display()
        ))? {
            let entry = entry?;
            let path = entry.path();

            if path.file_name().and_then(|n| n.to_str()) == Some(MARKER_FILE_NAME) {
                continue;
            }

            if !path.is_symlink() {
                continue;
            }

            let skill_name = path.file_name().and_then(|n| n.to_str());
            if skill_name.is_some_and(|name| keep_skill_names.iter().any(|keep| keep == name)) {
                continue;
            }

            if !dry_run {
                remove_symlink(&path)?;
            }
            removed.push(path);
        }
    }

    if !dry_run && has_only_marker_or_is_empty(target_dir)? {
        remove_marker(target_dir)?;
        if is_directory_empty(target_dir)? {
            fs::remove_dir(target_dir).context(format!(
                "Failed to remove empty directory: {}",
                target_dir.display()
            ))?;
        }
    }

    Ok(removed)
}

fn resolve_symlink_destination(link_path: &Path, current_target: &Path) -> Result<PathBuf> {
    let absolute_target = if current_target.is_absolute() {
        current_target.to_path_buf()
    } else {
        let parent = link_path.parent().context(format!(
            "Symlink has no parent directory: {}",
            link_path.display()
        ))?;
        parent.join(current_target)
    };
    fs::canonicalize(&absolute_target).context(format!(
        "Failed to resolve symlink destination: {}",
        link_path.display()
    ))
}

fn relative_path(from: &Path, to: &Path) -> Option<PathBuf> {
    let from_components: Vec<Component<'_>> = from.components().collect();
    let to_components: Vec<Component<'_>> = to.components().collect();

    if from_components.is_empty() || to_components.is_empty() {
        return None;
    }

    // If roots/prefixes differ, no safe relative path exists.
    if from_components[0] != to_components[0] {
        return None;
    }

    let mut common_len = 0;
    while common_len < from_components.len()
        && common_len < to_components.len()
        && from_components[common_len] == to_components[common_len]
    {
        common_len += 1;
    }

    let mut result = PathBuf::new();

    for component in &from_components[common_len..] {
        if matches!(component, Component::Normal(_)) {
            result.push("..");
        }
    }

    for component in &to_components[common_len..] {
        match component {
            Component::Normal(part) => result.push(part),
            Component::ParentDir => result.push(".."),
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    Some(result)
}

#[cfg(unix)]
fn create_symlink(source: &Path, link_path: &Path) -> Result<()> {
    std::os::unix::fs::symlink(source, link_path)
        .context(format!("Failed to create symlink: {}", link_path.display()))?;
    Ok(())
}

#[cfg(windows)]
fn create_symlink(source: &Path, link_path: &Path) -> Result<()> {
    std::os::windows::fs::symlink_dir(source, link_path)
        .context(format!("Failed to create symlink: {}", link_path.display()))?;
    Ok(())
}

/// Check if a directory is empty (or only contains the marker file)
fn is_directory_empty(dir: &Path) -> Result<bool> {
    let entries: Vec<_> = fs::read_dir(dir)?.collect();
    Ok(entries.is_empty())
}

fn has_only_marker_or_is_empty(dir: &Path) -> Result<bool> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(false);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_name().to_str() != Some(MARKER_FILE_NAME) {
            return Ok(false);
        }
    }

    Ok(true)
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
        assert!(link_target.is_relative());
        let resolved = fs::canonicalize(target_dir.join(link_target)).unwrap();
        assert_eq!(resolved, fs::canonicalize(skill_dir).unwrap());
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
        let resolved = fs::canonicalize(target_dir.join(link_target)).unwrap();
        assert_eq!(resolved, fs::canonicalize(skill_dir_2).unwrap());
    }

    #[test]
    fn should_keep_existing_absolute_symlink_when_destination_matches() {
        // Given
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("skill-source");
        let target_dir = temp.path().join("target");
        let link_path = target_dir.join("my-skill");

        fs::create_dir(&skill_dir).unwrap();
        fs::create_dir(&target_dir).unwrap();
        create_marker(&target_dir).unwrap();
        create_symlink(&skill_dir, &link_path).unwrap();
        let original_target = fs::read_link(&link_path).unwrap();
        assert!(original_target.is_absolute());

        // When
        link_skill("my-skill", &skill_dir, &target_dir).unwrap();

        // Then
        assert_eq!(fs::read_link(&link_path).unwrap(), original_target);
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

    #[test]
    fn should_prune_stale_symlinks_and_keep_requested_skills() {
        // Given
        let temp = TempDir::new().unwrap();
        let source_a = temp.path().join("source-a");
        let source_b = temp.path().join("source-b");
        let target_dir = temp.path().join("target");
        fs::create_dir_all(&source_a).unwrap();
        fs::create_dir_all(&source_b).unwrap();
        link_skill("keep-skill", &source_a, &target_dir).unwrap();
        link_skill("stale-skill", &source_b, &target_dir).unwrap();
        let keep = vec!["keep-skill".to_string()];

        // When
        let removed = prune_target_except(&target_dir, &keep).unwrap();

        // Then
        assert_eq!(removed.len(), 1);
        assert!(target_dir.join("keep-skill").exists());
        assert!(!target_dir.join("stale-skill").exists());
    }

    #[test]
    fn should_remove_marker_and_directory_when_prune_empties_target() {
        // Given
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let target_dir = temp.path().join("target");
        fs::create_dir_all(&source).unwrap();
        link_skill("stale-skill", &source, &target_dir).unwrap();
        let keep = Vec::new();

        // When
        prune_target_except(&target_dir, &keep).unwrap();

        // Then
        assert!(!target_dir.exists());
    }

    #[test]
    fn should_preview_prune_without_removing_symlinks() {
        // Given
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let target_dir = temp.path().join("target");
        fs::create_dir_all(&source).unwrap();
        link_skill("stale-skill", &source, &target_dir).unwrap();
        let keep = Vec::new();

        // When
        let removed = preview_prune_target(&target_dir, &keep).unwrap();

        // Then
        assert_eq!(removed.len(), 1);
        assert!(target_dir.join("stale-skill").exists());
        assert!(is_managed(&target_dir));
    }
}
