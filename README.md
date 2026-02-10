# agent-skills

Manage reusable skills for AI development tools. One repo, multiple tools.

Skills are markdown files with instructions that agents load on demand.
This repo provides the storage, activation config, and delivery scripts
to make them discoverable by [OpenCode](https://opencode.ai) and
[Claude Code](https://docs.anthropic.com/en/docs/claude-code) from a
single source of truth.

## How it works

```
skills/<name>/SKILL.md    skill definitions (version-controlled)
skills.toml               which skills are enabled where
scripts/install.sh        symlinks enabled skills into tool discovery paths
```

The install script reads `skills.toml` and creates symlinks in the
directories that OpenCode and Claude Code scan for skills. You manage
skills in one place; both tools find them automatically.

## Quick start

```bash
# Clone
git clone https://github.com/pentaxis93/agent-skills.git
cd agent-skills

# Create a skill
./scripts/new.sh git-commit "Create conventional commits with scope and body"

# Edit the generated SKILL.md
$EDITOR skills/git-commit/SKILL.md

# Enable it globally
# Edit skills.toml, add "git-commit" to [global] skills list

# Validate and install
./scripts/validate.sh
./scripts/install.sh
```

## Compatibility

Both tools discover skills from `<name>/SKILL.md` directories. The
install script symlinks into all supported paths:

| Path | Scope | Tool |
|------|-------|------|
| `~/.claude/skills/` | Global | Both |
| `~/.config/opencode/skills/` | Global | OpenCode |
| `~/.agents/skills/` | Global | Both |
| `.claude/skills/` | Project | Both |
| `.opencode/skills/` | Project | OpenCode |
| `.agents/skills/` | Project | Both |

Claude Code extensions in frontmatter (like `disable-model-invocation`,
`context`, `allowed-tools`) are silently ignored by OpenCode.

## Scripts

| Script | Purpose |
|--------|---------|
| `scripts/install.sh` | Link enabled skills into discovery paths |
| `scripts/install.sh --dry-run` | Show what would happen without changes |
| `scripts/install.sh --clean` | Remove all managed symlinks |
| `scripts/install.sh --list` | Show enabled skills per scope |
| `scripts/validate.sh` | Check all SKILL.md files for correctness |
| `scripts/validate.sh <name>` | Check a single skill |
| `scripts/new.sh <name> [desc]` | Scaffold a new skill directory |

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

## Configuration

`skills.toml` controls activation. Skills listed under `[global]` are
symlinked to global discovery paths. Per-project overrides add skills
to specific project directories.

```toml
[global]
skills = ["git-commit", "code-review"]

[projects."/home/user/my-app"]
skills = ["deploy-staging"]
inherit = true  # also include global skills (default)
```

See [`skills.toml`](skills.toml) for the full annotated config.

## Design

For architecture details, rationale, and the full compatibility matrix,
see [DESIGN.md](DESIGN.md).

## License

[MIT](LICENSE)
