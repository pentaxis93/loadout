# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Rust project scaffolding (Cargo.toml, clap, serde dependencies)
- Development roadmap for Phases 2–5

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

[Unreleased]: https://github.com/pentaxis93/loadout/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/pentaxis93/loadout/releases/tag/v0.1.0
