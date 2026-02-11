# Loadout — Roadmap

Development plan for evolving loadout from bash scripts into a Rust CLI,
then extending it from a symlink manager into a skill lifecycle tool.

For current architecture, see [DESIGN.md](../DESIGN.md).

## Overview

Loadout began as three bash scripts (`install.sh`, `validate.sh`, `new.sh`)
that parse TOML with Python and manage symlinks. The roadmap covers the
rewrite into a single compiled binary, then adds analysis, a TUI, and
composition features in later phases.

Each phase produces an independently useful tool. No phase depends on a
later phase being complete.

| Version | Phase | Summary |
|---------|-------|---------|
| 0.1.0 | [Phase 1 — Bash Scripts](#phase-1--bash-scripts) | Original bash implementation (complete) |
| 0.2.0 | [Phase 2 — Rust Parity](#phase-2--rust-parity) | Replace scripts with `loadout` binary |
| 0.3.0 | [Phase 3 — Analysis & Intelligence](#phase-3--analysis--intelligence) | Cross-references, health checks, dependency graphs |
| 0.4.0 | [Phase 4 — TUI](#phase-4--tui) | Interactive terminal interface |
| 0.5.0 | [Phase 5 — Composition & Evolution](#phase-5--composition--evolution) | Chains, tags, templates, gap analysis |

## Source layout

```
src/
├── main.rs              # Entry point, clap CLI dispatch
├── cli/
│   ├── mod.rs           # Clap command definitions
│   ├── install.rs       # loadout install
│   ├── clean.rs         # loadout clean
│   ├── list.rs          # loadout list
│   ├── new.rs           # loadout new
│   ├── validate.rs      # loadout validate
│   ├── check.rs         # loadout check          (Phase 3)
│   ├── graph.rs         # loadout graph           (Phase 3)
│   └── tui.rs           # loadout tui             (Phase 4)
├── config/
│   ├── mod.rs           # Config loading + path resolution
│   └── types.rs         # Serde structs for loadout.toml
├── skill/
│   ├── mod.rs           # Skill resolution, discovery
│   ├── frontmatter.rs   # YAML frontmatter parsing + validation
│   └── crossref.rs      # Cross-reference extraction (Phase 3)
├── linker/
│   ├── mod.rs           # Symlink creation, marker management
│   └── clean.rs         # Symlink removal
├── graph/               # Phase 3, behind `graph` feature
│   └── mod.rs           # Dependency graph construction + analysis
└── tui/                 # Phase 4, behind `tui` feature
    ├── mod.rs           # Ratatui app loop
    ├── skill_browser.rs # Browse/filter skills
    ├── graph_view.rs    # Visual dependency graph
    └── installer.rs     # Interactive install/toggle
```

Design decisions:

- Each bash script becomes a subcommand (`loadout install`, not `loadout --install`).
- `config`, `skill`, and `linker` are independent library modules. The CLI is
  a thin dispatch layer over them.
- Feature gates (`tui`, `graph`) keep optional dependencies out of the default
  binary.
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

Replace all three bash scripts with a single Rust binary installed via
`cargo install --path .`.

### Commands

| Command | Replaces | Behavior |
|---------|----------|----------|
| `loadout install` | `install.sh` | Parse config, resolve skills, create symlinks + markers |
| `loadout install --dry-run` | `install.sh --dry-run` | Print what would happen |
| `loadout clean` | `install.sh --clean` | Remove managed symlinks + markers |
| `loadout list` | `install.sh --list` | Show sources, targets, skills, resolution paths |
| `loadout validate [name]` | `validate.sh [name]` | Validate skill frontmatter |
| `loadout validate --dir <path>` | `validate.sh /path` | Validate all skills in directory |
| `loadout new <name> [--desc "..."]` | `new.sh <name> [desc]` | Scaffold new skill |
| `loadout new <name> --dir <path>` | `new.sh --dir <path>` | Scaffold into specific directory |

### Implementation order

1. **`config/`** — TOML parsing with serde. Path expansion (`~` to `$HOME`).
   XDG resolution. `$LOADOUT_CONFIG` override.
2. **`skill/frontmatter.rs`** — Extract YAML between `---` delimiters. Parse
   with `serde_yaml`. Validate name regex, length, directory match, description
   constraints.
3. **`skill/mod.rs`** — Walk source directories. Resolve skill by name (first
   match wins). Enumerate all available skills.
4. **`linker/`** — Create/remove symlinks. Marker file management
   (`.managed-by-loadout`). Conflict detection for existing non-managed
   directories.
5. **`cli/install.rs`** — Wire config + skill resolution + linker. Validate
   all skills before linking. Support `--dry-run`.
6. **`cli/clean.rs`** — Walk target directories, remove managed symlinks +
   markers, remove empty directories.
7. **`cli/list.rs`** — Enumerate sources with skill counts, global targets,
   skill-to-source resolution, project entries.
8. **`cli/validate.rs`** — Single skill, all skills, or directory mode. Report
   pass/fail with description preview.
9. **`cli/new.rs`** — Scaffold skill directory + SKILL.md from embedded template.

### Acceptance criteria

- [ ] `loadout install` produces identical symlink layout to `install.sh`
- [ ] `loadout clean` removes exactly what `install.sh --clean` removes
- [ ] `loadout list` output covers same information as `install.sh --list`
- [ ] `loadout validate` catches same errors as `validate.sh`
- [ ] `loadout new` produces same SKILL.md structure as `new.sh`
- [ ] No Python dependency at runtime
- [ ] `cargo install --path .` places binary in `~/.cargo/bin/loadout`

### Transition

Bash scripts remain in `scripts/` during Phase 2 for behavioral reference.
Once acceptance criteria pass, move them to `scripts/legacy/` or remove them.
Update README to show `cargo install --path .` as the install method.

---

## Phase 3 — Analysis & Intelligence

Move beyond install tooling into skill system analysis. This is where loadout
becomes more than a symlink manager.

### 3a. Cross-reference extraction

`skill/crossref.rs` — Parse SKILL.md body content (not just frontmatter) to
extract references to other skills. Detection heuristics:

- Explicit mentions in "Related skills" or "Integration" tables
- Backtick-quoted names matching the skill name pattern
- Phrases like "invoke the X skill", "load X first", "use X" adjacent to
  known skill names
- Frontmatter references (if skills declare dependencies explicitly)

Builds an in-memory dependency graph of skill relationships.

### 3b. `loadout check`

A diagnostic command reporting:

| Check | Severity |
|-------|----------|
| Dangling references — skills referenced but not in any source | error |
| Orphaned skills — in source but not in any config section | warning |
| Name/directory mismatch | error |
| Missing required frontmatter fields | error |
| Broken symlinks in target directories | error |
| Unmanaged conflicts in target directories | warning |
| Empty or placeholder descriptions | warning |
| Circular references | info |

Output grouped by severity with actionable messages.

### 3c. `loadout graph`

Behind the `graph` feature flag. Uses petgraph to build the skill dependency
graph with multiple output formats:

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
- **Bridge skills** — removal would disconnect clusters

### 3d. Enhanced `loadout list`

- `loadout list --groups` — skills organized by detected cluster
- `loadout list --refs <skill>` — incoming and outgoing references
- `loadout list --missing` — dangling references only

### Acceptance criteria

- [ ] `loadout check` identifies dangling references in the current skill set
- [ ] `loadout graph --format dot` produces valid Graphviz output
- [ ] Detected clusters match natural groupings (content pipeline, design
      system, foundational, elicitation, QA)
- [ ] All checks complete in under 1 second for 14 skills

---

## Phase 4 — TUI

Interactive terminal interface using ratatui. Behind the `tui` feature flag.

### Views

**Skill Browser**
- Filterable list of all skills with status indicators (installed, orphaned,
  broken)
- Preview pane showing description + frontmatter
- Toggle skills on/off per scope (global, per-project)
- Search/filter by name, description, group

**Graph View**
- Box-drawing dependency graph
- Navigate between connected skills
- Highlight clusters with color
- Show dangling references in red

**Install Dashboard**
- Current state of all target directories
- One-key install, clean, reinstall
- Diff view: what would change on next install

**Health Panel**
- Live results from `check` analysis
- Navigate directly to problem skills

### Interaction model

- Vim-style navigation (hjkl, /, ?)
- Tab to switch panels
- Enter to drill into skill detail
- Space to toggle selection
- `i` to install, `c` to clean
- `q` to quit

### Acceptance criteria

- [ ] TUI launches with `loadout tui`
- [ ] All four views are navigable
- [ ] Install/clean operations work from within TUI
- [ ] Binary without `tui` feature has no ratatui/crossterm dependency

---

## Phase 5 — Composition & Evolution

Features that help the skill system itself evolve.

### 5a. Skill chains

Named sequences for common workflows, defined in config:

```toml
[chains]
publish = ["seed-craft", "story-spine", "story-compiler", "strangers-eye"]
design  = ["web-design", "screenshot", "design-loop"]
```

`loadout chain <name>` lists the skills in order with descriptions, verifying
all are present and installed. Informational — it documents workflows and
validates completeness, it does not invoke skills.

### 5b. Skill groups / tags

Optional `tags` field in frontmatter:

```yaml
name: story-compiler
description: ...
tags: [content-pipeline, writing]
```

- `loadout list --tag <tag>` — filter by tag
- `loadout list --tags` — show all tags with counts
- Untagged skills fall back to auto-detected graph clusters from Phase 3

### 5c. Skill templates

Extend `loadout new` with `--from <template>`:

- `loadout new my-skill --from minimal` — frontmatter + heading only
- `loadout new my-skill --from full` — all standard sections (current default)
- `loadout new my-skill --from <existing-skill>` — copy structure from
  another skill

### 5d. Gap analysis

`loadout gaps` — combines graph analysis with cross-reference data to report:

- Skills referenced but not present (create candidates)
- Clusters with single points of failure (bridge nodes at risk)
- Skills with no references in or out (isolated — still useful?)
- Workflow chains with missing steps

### Acceptance criteria

- [ ] Chains validate that all referenced skills exist and are installed
- [ ] Tags are optional — untagged skills work everywhere
- [ ] Gap analysis identifies referenced-but-missing skills as creation
      candidates

---

## Open questions

These are recorded for future consideration. None block current work.

**Config evolution.** Phase 5 adds `[chains]` and skill-level `tags`. Tags
belong in SKILL.md frontmatter (skill metadata, portable). Chains belong in
`loadout.toml` (user configuration, personal). This split keeps skills
portable and chains personal.

**Drop-in config fragments.** Should loadout support `loadout.d/*.toml` for
composing config from multiple files? Useful for separating global from
project overrides. Not critical now but worth considering in the config
module design.

**Remote sources.** Should `[sources].skills` eventually support git URLs
for team/community skill sharing? Significant scope increase — probably a
Phase 6 concern if it ever becomes one.
