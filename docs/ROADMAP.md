# Loadout — Roadmap

Development plan for evolving loadout from bash scripts into a Rust CLI,
then extending it from a symlink manager into a skill lifecycle tool.

For current architecture, see [DESIGN.md](../DESIGN.md).

## Overview

Loadout began as three bash scripts (`install.sh`, `validate.sh`, `new.sh`)
that parse TOML with Python and manage symlinks. The roadmap covers the
rewrite into a single compiled binary, then adds analysis and metadata
capabilities for ongoing skill library maintenance.

Each phase produces an independently useful tool. No phase depends on a
later phase being complete.

| Version | Phase | Summary |
|---------|-------|---------|
| 0.1.0 | [Phase 1 — Bash Scripts](#phase-1--bash-scripts) | Original bash implementation (complete) |
| 0.2.0 | [Phase 2 — Rust Parity](#phase-2--rust-parity) | Replace scripts with `loadout` binary (complete) |
| 0.3.0 | [Phase 3 — Analysis & Intelligence](#phase-3--analysis--intelligence) | Cross-references, health checks, dependency graphs (complete) |
| 0.3.5 | [Phase 3.5 — Metadata & Actionable Output](#phase-35--metadata--actionable-output) | Tags, pipelines, actionable diagnostics (complete) |
| 0.3.6 | Maintenance Release | Path-resolution fixes, roadmap simplification, and feature-surface cleanup |

## Source layout

```
src/
├── main.rs              # Entry point, clap CLI dispatch
├── commands/
│   ├── mod.rs           # Re-exports
│   ├── install.rs       # loadout install
│   ├── clean.rs         # loadout clean
│   ├── list.rs          # loadout list (all modes)
│   ├── new.rs           # loadout new
│   ├── validate.rs      # loadout validate
│   ├── check.rs         # loadout check
│   └── graph.rs         # loadout graph
├── config/
│   ├── mod.rs           # Config loading + path resolution
│   └── types.rs         # Serde structs for loadout.toml
├── skill/
│   ├── mod.rs           # Skill resolution, discovery
│   ├── frontmatter.rs   # YAML frontmatter parsing + validation
│   └── crossref.rs      # Cross-reference extraction
├── linker/
│   └── mod.rs           # Symlink creation, marker management, cleanup
├── graph/
│   └── mod.rs           # Dependency graph construction + analysis
```

Design decisions:

- Each bash script becomes a subcommand (`loadout install`, not `loadout --install`).
- `config`, `skill`, and `linker` are independent library modules. The CLI is
  a thin dispatch layer over them.
- The `graph` feature is enabled by default and keeps graph-specific
  dependencies optional.
- `thiserror` for typed library errors, `anyhow` for CLI-level propagation.

---

## Phase 1 — Bash Scripts

**Status: Complete (v0.1.0)**

The original implementation using bash scripts with Python TOML parsing.
Provides core symlink management functionality.

### Scripts

- `install.sh` — Parse config, resolve skills, create symlinks + markers
- `install.sh --dry-run` — Print what would happen
- `install.sh --clean` — Remove managed symlinks + markers
- `install.sh --list` — Show sources, targets, skills, resolution paths
- `validate.sh [name]` — Validate skill frontmatter
- `validate.sh /path` — Validate all skills in directory
- `new.sh <name> [desc]` — Scaffold new skill

### Limitations

- Requires Python for TOML parsing
- No built-in dependency analysis
- Limited error handling
- Three separate scripts instead of unified CLI

---

## Phase 2 — Rust Parity

**Status: Complete**

Replaced all three bash scripts with a single Rust binary installed via
`cargo install --path .`.

### Commands

| Command | Replaces | Behavior |
|---------|----------|----------|
| `loadout install` | `install.sh` | Parse config, resolve skills, create symlinks + markers |
| `loadout install --dry-run` | `install.sh --dry-run` | Print what would happen |
| `loadout clean` | `install.sh --clean` | Remove managed symlinks + markers |
| `loadout list` | `install.sh --list` | Show sources, targets, skills, resolution paths |
| `loadout validate [name]` | `validate.sh [name]` | Validate skill frontmatter |
| `loadout validate <dir>` | `validate.sh /path` | Validate all skills in directory |
| `loadout new <name> [-d "..."]` | `new.sh <name> [desc]` | Scaffold new skill |

### Acceptance criteria

- [x] `loadout install` produces identical symlink layout to `install.sh`
- [x] `loadout clean` removes exactly what `install.sh --clean` removes
- [x] `loadout list` output covers same information as `install.sh --list`
- [x] `loadout validate` catches same errors as `validate.sh`
- [x] `loadout new` produces same SKILL.md structure as `new.sh`
- [x] No Python dependency at runtime
- [x] `cargo install --path .` places binary in `~/.cargo/bin/loadout`

---

## Phase 3 — Analysis & Intelligence

**Status: Complete**

Moved beyond install tooling into skill system analysis. This is where
loadout became more than a symlink manager.

### 3a. Cross-reference extraction

`skill/crossref.rs` — Parses SKILL.md body content (not just frontmatter)
to extract references to other skills. Detection heuristics:

- Explicit mentions in "Related skills" or "Integration" tables
- Backtick-quoted names matching the skill name pattern
- Phrases like "invoke the X skill", "load X first", "use X" adjacent to
  known skill names
- XML `<crossref>` elements

Builds an in-memory dependency graph of skill relationships.

### 3b. `loadout check`

A diagnostic command reporting health issues with actionable fix suggestions:

| Check | Severity |
|-------|----------|
| Dangling references — skills referenced but not in any source | error |
| Orphaned skills — in source but not in any config section | warning |
| Name/directory mismatch | error |
| Missing required frontmatter fields | error |
| Broken symlinks in target directories | error |
| Unmanaged conflicts in target directories | warning |
| Empty or placeholder descriptions | warning |

Output grouped by severity with fix suggestions on every finding.

### 3c. `loadout graph`

Uses petgraph to build the skill dependency graph with multiple output
formats:

| Format | Flag | Use case |
|--------|------|----------|
| DOT (Graphviz) | `--format dot` | Pipe to `dot -Tsvg` |
| Adjacency list | `--format text` | Terminal-friendly |
| JSON | `--format json` | Machine-readable |
| Mermaid | `--format mermaid` | Embed in markdown |

Additional analysis:

- **Clusters** — groups of tightly connected skills
- **Root skills** — no incoming references (entry points)
- **Leaf skills** — no outgoing references (pure utilities)

### 3d. Enhanced `loadout list`

- `loadout list --groups` — skills organized by detected cluster
- `loadout list --refs <skill>` — incoming and outgoing references
- `loadout list --missing` — dangling references only

### Acceptance criteria

- [x] `loadout check` identifies dangling references in the current skill set
- [x] `loadout graph --format dot` produces valid Graphviz output
- [x] Detected clusters match natural groupings (content pipeline, design
      system, foundational, elicitation, QA)
- [x] All checks complete in under 1 second for 23 skills

---

## Phase 3.5 — Metadata & Actionable Output

**Status: Complete**

Extended the skill format with metadata fields and made all analysis
commands produce actionable output. Pulled forward from earlier roadmap drafts because
tags and workflow ordering proved essential for meaningful analysis.

### 3.5a. Tags

Optional `tags` field in SKILL.md frontmatter:

```yaml
tags: [blog, writing, meta-skill]
```

- Kebab-case validated, stored as `Option<Vec<String>>`
- `loadout list --tags` — all tags with skill counts
- `loadout list --tag <tag>` — filter skills by tag
- `loadout graph --tag <tag>` — filter graph to tagged skills

### 3.5b. Pipelines

Optional `pipeline` field declaring workflow stage ordering:

```yaml
pipeline:
  blog-production:
    stage: compile
    order: 3
    after: [story-spine]
    before: [blog-edit]
```

- Skills can participate in multiple pipelines
- `after`/`before` are cross-validated for consistency
- `loadout list --pipelines` — all pipelines with stage summaries
- `loadout list --pipeline <name>` — pipeline in stage order
- `loadout graph --pipeline <name>` — filter graph to pipeline skills

### 3.5c. Actionable check output

Every `loadout check` finding now includes a concrete fix suggestion:

| Finding | Fix suggestion |
|---------|---------------|
| Dangling reference | `loadout new <name>`, or remove the reference |
| Orphaned skill | Add to `[global].skills` in loadout.toml |
| Pipeline gap (asymmetric after/before) | Add missing reciprocal declaration |
| Pipeline references non-existent skill | Create or remove from after/before |
| No tags and no pipeline | Add metadata (only when library is partially annotated) |

Suppression via `[check.ignore]` in loadout.toml:

```toml
[check]
ignore = ["dangling:skill-format:related-skill"]
```

`--verbose` flag reveals suppressed findings.

### 3.5d. Graph enhancements

- Edge deduplication (same pair, different detection methods → single edge)
- `EdgeKind` distinguishes CrossRef (content-detected) from Pipeline (declared)
- Pipeline edges rendered distinctly (dashed/blue in DOT, dotted in Mermaid)

### Acceptance criteria

- [x] Tags validated and parsed from frontmatter
- [x] Pipeline stages validated with order and dependency references
- [x] Every finding type has a non-empty fix suggestion
- [x] Pipeline integrity checks detect missing deps and asymmetric declarations
- [x] No-metadata check only fires when library is partially annotated
- [x] Suppression via `[check.ignore]` works; `--verbose` reveals suppressed
- [x] Graph filtering by `--pipeline` and `--tag` works for all output formats
- [x] 111 tests passing, clippy clean

---

## Open questions

These are recorded for future consideration. None block current work.

**Frontmatter rewriting.** Some future metadata mutation commands may require
modifying YAML inside markdown files. Options: regex-based surgery (fragile), roundtrip YAML parser
(complex), or a template-based approach that rewrites the entire frontmatter
block. Worth prototyping before committing to an approach.

**Drop-in config fragments.** Should loadout support `loadout.d/*.toml` for
composing config from multiple files? Useful for separating global from
project overrides. Not critical now but worth considering in the config
module design.

**Remote sources.** Should `[sources].skills` eventually support git URLs
for team/community skill sharing? Significant scope increase — probably a
future-phase concern if it ever becomes one.

## Resolved questions

**Tags in frontmatter vs config.** Resolved: tags belong in SKILL.md
frontmatter (portable with the skill). Delivered in Phase 3.5.

**Chains vs pipelines.** Earlier roadmap drafts proposed `[chains]` in
loadout.toml — named sequences of skills for common workflows. This was
superseded by the `pipeline` frontmatter field, which is more expressive
(stages, ordering, dependency cross-validation) and portable (travels with
the skill, not locked to one user's config). The config-based chain concept
is retired.
