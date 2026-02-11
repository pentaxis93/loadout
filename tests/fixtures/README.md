# Test Fixtures

Sample data for integration tests.

## Structure

```
fixtures/
├── loadout.toml          # Minimal config pointing to fixtures/skills/
├── skills/               # Sample skill source directory
│   ├── test-skill/       # Basic skill at root level
│   ├── another-skill/    # Second skill for multi-skill tests
│   └── category/
│       └── nested-skill/ # Skill in subdirectory (tests recursive discovery)
└── targets/              # Empty target directories created during tests
    └── global/
```

## Usage

Integration tests should:
1. Create a `TempDir` for the test
2. Copy or reference these fixtures as needed
3. Run `loadout` commands against the fixture config
4. Verify expected symlinks, errors, or output

Example:

```rust
use tempfile::TempDir;
use std::fs;

#[test]
fn should_install_skills_from_fixture_config() {
    // Given
    let tmp = TempDir::new().unwrap();
    let fixture_config = "tests/fixtures/loadout.toml";
    
    // When
    let result = install_from_config(fixture_config, tmp.path());
    
    // Then
    assert!(result.is_ok());
    let symlink = tmp.path().join("test-skill");
    assert!(symlink.exists());
}
```
