# AGENTS.md

Development guide for coding agents working in the loadout repository.

## Project Overview

**loadout** is a skill lifecycle management system for AI agents (OpenCode and Claude Code). It manages SKILL.md files by symlinking them from user-controlled directories into tool discovery paths.

- **Language**: Rust (Edition 2021), currently in Phase 2 development (v0.2.0-dev)
- **Phase 1**: Bash scripts implementation (complete, in `scripts/`)
- **Phase 2**: Rust CLI rewrite (in progress, replacing bash scripts)
- **Current State**: Minimal Rust scaffolding; bash scripts are the working implementation

## Available Skills

This project has skills loaded that provide guidance for common development tasks:

- **bdd** — Behaviour-driven development. Use when writing tests or deciding what to test next.
- **dev-workflow** — Conventional commits, changelog maintenance, issue management.
- **issue-craft** — Create, decompose, refine, triage, and close GitHub issues optimized for autonomous execution.

## Build, Test, and Lint Commands

### Build
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run the binary (currently just prints "Hello, world!")
cargo run

# Install to ~/.cargo/bin/
cargo install --path .
```

### Test
```bash
# Run all tests (currently none exist)
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a single test
cargo test test_name

# Run tests in a specific module
cargo test module_name::
```

### Lint and Format
```bash
# Format code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check

# Run Clippy linter
cargo clippy

# Clippy with all warnings as errors
cargo clippy -- -D warnings
```

### Current Working Scripts (Phase 1)
```bash
# Install skills (main operation)
./scripts/install.sh
./scripts/install.sh --dry-run
./scripts/install.sh --clean
./scripts/install.sh --list

# Validate SKILL.md files
./scripts/validate.sh                    # all skills
./scripts/validate.sh skill-name         # single skill by name
./scripts/validate.sh /path/to/dir       # all skills in directory

# Create new skill
./scripts/new.sh <name> [description]
./scripts/new.sh --dir <path> <name> [description]
```

## Code Style Guidelines

### File Organization
```
src/
├── main.rs              # Entry point, clap CLI dispatch
├── commands/            # CLI command implementations (Phase 2)
├── config/              # Config loading, TOML parsing
├── linker/              # Symlink creation and marker management
└── skill/               # Skill resolution, validation, discovery
```

### Imports
- Group imports: std library, external crates, internal modules
- Use `use` statements, avoid wildcard imports except for preludes
- Sort alphabetically within groups

```rust
// Standard library
use std::fs;
use std::path::{Path, PathBuf};

// External crates
use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};

// Internal modules
use crate::config::Config;
use crate::skill::Skill;
```

### Formatting
- **Indentation**: 4 spaces (Rust standard)
- **Line length**: 100 characters max (default rustfmt)
- **Use rustfmt defaults**: Do not create custom `.rustfmt.toml` without discussion

### Naming Conventions
- **Types/Traits**: `PascalCase` (e.g., `SkillResolver`, `ConfigLoader`)
- **Functions/Variables**: `snake_case` (e.g., `resolve_skill`, `skill_path`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_CONFIG_PATH`)
- **Modules**: `snake_case` (e.g., `skill`, `config`, `linker`)
- **Lifetimes**: Single lowercase letter or descriptive (e.g., `'a`, `'config`)

### Types
- Use strong types; avoid stringly-typed code
- Leverage `Path`/`PathBuf` for filesystem paths
- Use `anyhow::Result<T>` for CLI error propagation
- Use `thiserror` for typed library errors

```rust
// Good: Strong types
pub struct SkillPath(PathBuf);

// Avoid: Stringly-typed
pub type SkillPath = String;
```

### Error Handling
- **CLI layer**: Use `anyhow::Result` for error propagation, add context
- **Library layer**: Use `thiserror` for typed error enums
- Always add context when wrapping errors

```rust
use anyhow::{Context, Result};

// Add context to errors
fs::read_to_string(path)
    .context(format!("Failed to read skill file: {}", path.display()))?;

// Library error types with thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(String),
    #[error("Invalid frontmatter: {0}")]
    InvalidFrontmatter(String),
}
```

### Structs and Data
- Derive `Debug` for all types
- Use `#[derive(Serialize, Deserialize)]` for config/TOML types
- Document public APIs with `///` doc comments
- Use `#[serde(rename = "...")]` for TOML keys with hyphens

```rust
/// Configuration loaded from loadout.toml
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Source directories containing skills
    pub sources: Sources,
    
    /// Global skill activation
    pub global: Global,
    
    /// Per-project overrides
    #[serde(default)]
    pub projects: HashMap<PathBuf, Project>,
}
```

### Pattern Matching
- Use exhaustive pattern matching where possible
- Prefer `if let` for single-variant checks
- Use `match` for multiple branches

```rust
// Prefer match for clarity
match result {
    Ok(value) => process(value),
    Err(e) => log_error(e),
}

// Use if let for single variant
if let Some(skill) = find_skill(name) {
    install(skill);
}
```

### Comments
- Use `//` for inline comments
- Use `///` for public API documentation
- Use `//!` for module-level documentation
- Keep comments concise and meaningful
- Avoid obvious comments; code should be self-documenting

### Testing (BDD)

This project uses behaviour-driven development. Tests describe **what the
system should do**, not how the code works internally. See the `bdd` skill
for the full thinking pattern.

**Core rules:**
- Test names are sentences: `should_<behaviour>` or `should_<behaviour>_when_<context>`
- Every test body has three phases: **Given** (setup), **When** (action), **Then** (assertion)
- One behaviour per test — if the name contains "and", split it
- Assert on observable outcomes, not implementation details

**Structure:**
- Unit tests in same file using `#[cfg(test)]` module
- Integration tests in `tests/` directory
- Use `tempfile::TempDir` for filesystem tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_resolve_skill_from_first_matching_source() {
        // Given
        let config = test_config_with_sources(&["/source-a", "/source-b"]);
        create_skill("/source-a/my-skill/SKILL.md");

        // When
        let result = resolve_skill(&config, "my-skill");

        // Then
        assert!(result.is_ok());
        assert_eq!(result.unwrap().source, PathBuf::from("/source-a"));
    }

    #[test]
    fn should_return_error_when_skill_not_found() {
        // Given
        let config = test_config_with_sources(&["/empty-source"]);

        // When
        let result = resolve_skill(&config, "nonexistent");

        // Then
        assert!(result.is_err());
    }
}
```

Reading test names as a specification:

```
resolve_skill
  - should resolve skill from first matching source
  - should return error when skill not found
```

## Configuration and Paths

### Config Resolution
1. `$LOADOUT_CONFIG` environment variable (if set)
2. `$XDG_CONFIG_HOME/loadout/loadout.toml` (if `XDG_CONFIG_HOME` set)
3. `~/.config/loadout/loadout.toml` (default)

### Key Paths
- **User config**: `~/.config/loadout/loadout.toml`
- **User skills**: `~/.config/loadout/skills/`
- **Template**: `skills/_template/`
- **Schema**: `schema/skill-frontmatter.json`

### Discovery Paths (Target Directories)
- `~/.claude/skills/` (global, both tools)
- `~/.config/opencode/skills/` (global, OpenCode)
- `~/.agents/skills/` (global, both tools)
- `.claude/skills/` (project-local, both tools)
- `.opencode/skills/` (project-local, OpenCode)
- `.agents/skills/` (project-local, both tools)

## SKILL.md Format

### Name Validation
Pattern: `^[a-z0-9]+(-[a-z0-9]+)*$`
- Lowercase alphanumeric with single-hyphen separators
- 1-64 characters
- Must match containing directory name

### Required Frontmatter
```yaml
---
name: skill-name          # Must match directory name
description: >-           # 1-1024 chars, what it does and when to use it
  Description text here
---
```

### Optional Fields
- **Claude Code**: `disable-model-invocation`, `user-invocable`, `allowed-tools`, `context`, `agent`, `model`, `argument-hint`
- **OpenCode**: `license`, `compatibility`, `metadata`

## Development Workflow

### Git Workflow

This project uses **direct commits to main** (no PRs required for solo development):

- Work directly on `main` or feature branches: `feat/#5-config-module`, `fix/#10-symlink-cleanup`
- Use **conventional commits**: `feat(config): add XDG resolution`
- Reference issues in commit messages: `Closes #5` or `Refs #1`
- Update `CHANGELOG.md` under `[Unreleased]` section for all user-facing changes
- Squash locally if needed before pushing (keep history clean)
- Delete feature branches after merging to main

**Commit format:**
```
<type>(<scope>): <description>

[optional body explaining why, not what]

Closes #N
```

**Types:** `feat`, `fix`, `docs`, `refactor`, `test`, `chore`  
**Scopes:** Module names like `config`, `skill`, `linker`, `cli`

### Phase 2 Implementation Order
Per docs/ROADMAP.md:
1. `config/` — TOML parsing, path expansion, XDG resolution
2. `skill/frontmatter.rs` — YAML extraction and validation
3. `skill/mod.rs` — Skill discovery and resolution
4. `linker/` — Symlink creation, marker management
5. `commands/` — CLI subcommands (install, clean, list, validate, new)

### Acceptance Criteria (Phase 2)
- `loadout install` produces identical symlink layout to `install.sh`
- `loadout clean` removes exactly what `install.sh --clean` removes
- No Python dependency at runtime
- All bash script functionality replicated in Rust

## Dependencies

### Core
- `clap` (4.5) — CLI argument parsing with derive macros
- `toml` (0.8) — Config file parsing
- `serde` (1.0) + `serde_yaml` (0.9) — Serialization
- `walkdir` (2.5) — Directory traversal

### Error Handling
- `anyhow` (1.0) — CLI-level error propagation
- `thiserror` (1.0) — Typed library errors

### Optional Features
- `tui` feature: `ratatui` (0.29), `crossterm` (0.28) — Phase 4
- `graph` feature: `petgraph` (0.6) — Phase 3

## References

- **DESIGN.md**: Architecture and rationale
- **docs/ROADMAP.md**: 5-phase development plan
- **README.md**: User guide and quick start
- **schema/skill-frontmatter.json**: Frontmatter validation schema
