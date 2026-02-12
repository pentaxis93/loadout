# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.3.5] — 2026-02-12

Phases 2, 3, and 3.5: Rust CLI, analysis commands, and metadata.

### Added

#### Phase 2 — Rust CLI
- `loadout install` and `loadout clean` commands with `--dry-run` support
- `loadout validate` for SKILL.md frontmatter validation (all, by name, or by directory)
- `loadout new` for creating skills from template
- `loadout list` with default scope view and skill paths
- `loadout check` for skill system health diagnostics
- Config loading with XDG resolution, tilde expansion, and `$LOADOUT_CONFIG` override
- Symlink creation with `.managed-by-loadout` marker tracking

#### Phase 3 — Analysis
- `loadout graph` with DOT, text, JSON, and Mermaid output formats
- Cross-reference extraction from skill content (XML, backtick, natural language, related tables)
- Cluster detection and root/leaf skill identification
- `loadout list --groups` for cluster-organized display
- `loadout list --refs <skill>` for incoming/outgoing references
- `loadout list --missing` for dangling reference detection

#### Phase 3.5 — Metadata & Actionable Output
- `tags` field in SKILL.md frontmatter for classification (kebab-case, validated)
- `pipeline` field in SKILL.md frontmatter for workflow stage ordering
  with `stage`, `order`, `after`, and `before` declarations
- `loadout list --tags` to show all tags with skill counts
- `loadout list --tag <tag>` to filter skills by tag
- `loadout list --pipelines` to show all pipelines with stage summaries
- `loadout list --pipeline <name>` to show a pipeline in stage order
- `loadout graph --pipeline <name>` and `--tag <tag>` for filtered graphs
- Graph edge deduplication and `EdgeKind` (CrossRef vs Pipeline) distinction
- Pipeline-aware checks: missing dependencies, asymmetric after/before declarations
- Metadata coverage check for skills with no tags and no pipeline
  (only fires when library is partially annotated)
- Fix suggestions on every `loadout check` finding with `↳` prefix
- `[check]` config section with `ignore` patterns for suppression
- `loadout check --verbose` to show suppressed findings alongside active ones
- JSON schema updated for tags and pipeline fields
- Skill template updated with commented `tags` field

## [0.1.0] — 2026-02-10

Phase 1: Bash Scripts Implementation.

### Added
- `install.sh` — config parsing, skill resolution, symlink management
  with `--dry-run`, `--clean`, and `--list` modes
- `validate.sh` — SKILL.md frontmatter validation (single skill or
  directory mode)
- `new.sh` — skill directory scaffolding with SKILL.md template
- XDG-compliant config resolution with `$LOADOUT_CONFIG` override
- Multiple source directories with first-match-wins resolution
- Global and per-project target directories
- Managed symlink tracking via `.managed-by-loadout` marker files
- `DESIGN.md` documenting config format, skill structure, and
  resolution rules

[Unreleased]: https://github.com/pentaxis93/loadout/compare/v0.3.5...HEAD
[0.3.5]: https://github.com/pentaxis93/loadout/compare/v0.1.0...v0.3.5
[0.1.0]: https://github.com/pentaxis93/loadout/releases/tag/v0.1.0
