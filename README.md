# loadout

Skill management for AI development tools.

Loadout is the tool that links your skills into the discovery paths
that [OpenCode](https://opencode.ai) and
[Claude Code](https://docs.anthropic.com/en/docs/claude-code) scan.
Your skills live wherever you want. Loadout wires them up.

## How it works

```
~/.config/loadout/
├── loadout.toml              what's enabled and where
└── skills/                   your skill definitions
    └── <name>/SKILL.md

         ↓ install.sh

~/.claude/skills/             → Claude Code discovers them
~/.config/opencode/skills/    → OpenCode discovers them
~/.agents/skills/             → Any compatible tool
```

The repo contains the scripts and schema. Your personal config and
skills live in `~/.config/loadout/` (XDG-compliant) and never touch
version control.

## Quick start

```bash
# Clone and build
git clone https://github.com/pentaxis93/loadout.git
cd loadout
cargo install --path .

# Set up your config
mkdir -p ~/.config/loadout/skills
cp loadout.example.toml ~/.config/loadout/loadout.toml

# Create a skill
loadout new git-commit --description "Create conventional commits with scope and body"

# Edit it
$EDITOR ~/.config/loadout/skills/git-commit/SKILL.md

# Enable it — add "git-commit" to [global] skills in loadout.toml
$EDITOR ~/.config/loadout/loadout.toml

# Validate and install
loadout validate
loadout install
```

## Configuration

`~/.config/loadout/loadout.toml` controls everything.

Override the config path with `$LOADOUT_CONFIG` or `$XDG_CONFIG_HOME`:

```bash
# Use an alternate config
LOADOUT_CONFIG=~/work-loadout.toml loadout install

# Respects XDG
XDG_CONFIG_HOME=~/.local/config loadout install
```

```toml
[sources]
skills = [
  "~/.config/loadout/skills",         # your personal skills
  # "/path/to/team-skills/skills",    # shared/team skills
]

[global]
targets = [
  "~/.claude/skills",
  "~/.config/opencode/skills",
  "~/.agents/skills",
]
skills = ["git-commit", "code-review"]

[projects."/home/user/my-app"]
skills = ["deploy-staging"]
inherit = true  # also include global skills (default)
```

**Sources** are directories containing skill folders. Listed in priority
order — first match wins for duplicate names. This lets you layer team
skills under personal overrides.

See [`loadout.example.toml`](loadout.example.toml) for the full
annotated config.

## Commands

| Command | Purpose |
|---------|---------|
| `loadout install` | Link enabled skills into discovery paths |
| `loadout install --dry-run` | Show what would happen without changes |
| `loadout clean` | Remove all managed symlinks |
| `loadout clean --dry-run` | Preview what would be cleaned |
| `loadout check` | Check skill system health and report diagnostics |
| `loadout check --severity <level>` | Filter diagnostics by severity (error, warning, info) |
| `loadout graph --format dot` | Visualize dependency graph as Graphviz DOT |
| `loadout graph --format text` | Show dependency graph as text adjacency list |
| `loadout graph --format json` | Export dependency graph as JSON |
| `loadout graph --format mermaid` | Render dependency graph as Mermaid diagram |
| `loadout list` | Show enabled skills per scope with paths |
| `loadout list --tags` | Show all tags with skill counts |
| `loadout list --tag <tag>` | Show skills with a specific tag |
| `loadout list --pipelines` | Show all pipelines with stage summaries |
| `loadout list --pipeline <name>` | Show a pipeline in stage order with dependencies |
| `loadout list --groups` | Organize skills by detected cluster |
| `loadout list --refs <skill>` | Show incoming and outgoing references for a skill |
| `loadout list --missing` | Show only missing skills (dangling references) |
| `loadout validate` | Check all skills across all sources |
| `loadout validate <name>` | Check a specific skill by name |
| `loadout validate <dir>` | Check all skills in a directory |
| `loadout new <name>` | Create a new skill from template |
| `loadout new <name> -d "desc"` | Create skill with description |

All commands respect `$LOADOUT_CONFIG` to locate your config file.

Use `loadout --help` or `loadout <command> --help` for detailed usage.

## Compatibility

The install script symlinks into all paths that OpenCode and Claude
Code scan:

| Path | Scope | Tool |
|------|-------|------|
| `~/.claude/skills/` | Global | Both |
| `~/.config/opencode/skills/` | Global | OpenCode |
| `~/.agents/skills/` | Global | Both |
| `.claude/skills/` | Project | Both |
| `.opencode/skills/` | Project | OpenCode |
| `.agents/skills/` | Project | Both |

Claude Code frontmatter extensions (`disable-model-invocation`,
`context`, `allowed-tools`) are silently ignored by OpenCode.

## SKILL.md format

Each skill is a directory containing a `SKILL.md` file with YAML
frontmatter and markdown instructions:

```yaml
---
name: my-skill
description: >-
  What this skill does and when the agent should use it.
---

Instructions the agent receives when it loads this skill.
```

### Required fields

| Field | Constraint |
|-------|-----------|
| `name` | Lowercase alphanumeric, single-hyphen separators, 1-64 chars, must match directory name |
| `description` | 1-1024 characters |

### Optional fields

**Claude Code** (ignored by OpenCode):

| Field | Effect |
|-------|--------|
| `disable-model-invocation: true` | Only user can invoke via `/name` |
| `user-invocable: false` | Only the model can invoke; hidden from `/` menu |
| `allowed-tools: Read, Grep` | Tools permitted without per-use approval |
| `context: fork` | Run in an isolated subagent |
| `agent: Explore` | Subagent type for `context: fork` |
| `argument-hint: [issue-number]` | Autocomplete hint |

**Loadout metadata** (used by loadout commands, ignored by agents):

| Field | Effect |
|-------|--------|
| `tags: [blog, writing]` | Classification tags for filtering and grouping |
| `pipeline:` | Workflow participation with stage ordering (see below) |

Pipeline fields declare how a skill fits into a workflow:

```yaml
pipeline:
  blog-production:
    stage: compile       # human label for this skill's role
    order: 3             # numeric position (1-based)
    after: [story-spine] # skills that run before this one
    before: [blog-edit]  # skills that run after this one
```

A skill can participate in multiple pipelines. Use `loadout list --pipelines`
to see all defined pipelines, and `loadout list --pipeline <name>` for detail.

**OpenCode** (ignored by Claude Code):

| Field | Effect |
|-------|--------|
| `license: MIT` | License identifier |
| `compatibility: opencode` | Tool compatibility hint |
| `metadata: {}` | Arbitrary string-to-string map |

## Design

For architecture details, rationale, and the full compatibility matrix,
see [DESIGN.md](DESIGN.md).

## License

[MIT](LICENSE)
