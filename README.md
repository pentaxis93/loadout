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
# Clone the tool
git clone https://github.com/pentaxis93/loadout.git
cd loadout

# Set up your config
mkdir -p ~/.config/loadout/skills
cp loadout.example.toml ~/.config/loadout/loadout.toml

# Create a skill
./scripts/new.sh git-commit "Create conventional commits with scope and body"

# Edit it
$EDITOR ~/.config/loadout/skills/git-commit/SKILL.md

# Enable it — add "git-commit" to [global] skills in loadout.toml
$EDITOR ~/.config/loadout/loadout.toml

# Validate and install
./scripts/validate.sh
./scripts/install.sh
```

## Configuration

`~/.config/loadout/loadout.toml` controls everything.

Override the config path with `$LOADOUT_CONFIG` or `$XDG_CONFIG_HOME`:

```bash
# Use an alternate config
LOADOUT_CONFIG=~/work-loadout.toml ./scripts/install.sh

# Respects XDG
XDG_CONFIG_HOME=~/.local/config ./scripts/install.sh
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

## Scripts

| Script | Purpose |
|--------|---------|
| `scripts/install.sh` | Link enabled skills into discovery paths |
| `scripts/install.sh --dry-run` | Show what would happen without changes |
| `scripts/install.sh --clean` | Remove all managed symlinks |
| `scripts/install.sh --list` | Show sources, skills, and resolution |
| `scripts/validate.sh` | Check all skills across all sources |
| `scripts/validate.sh <name>` | Check a single skill by name |
| `scripts/new.sh <name> [desc]` | Scaffold a new personal skill |
| `scripts/new.sh --dir <path> <name> [desc]` | Scaffold into a specific directory |

All scripts respect `$LOADOUT_CONFIG` to locate your config file.

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
