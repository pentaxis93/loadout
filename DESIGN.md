# Agent Skills — Design

A skill management system for AI development tools (OpenCode, Claude Code).

## Architecture

```
                 ┌─────────────────────┐
                 │   agent-skills repo  │  source of truth
                 │   skills/            │  organized by category
                 │   skills.toml        │  activation config
                 └─────────┬───────────┘
                           │
                    install.sh
                           │
            ┌──────────────┼──────────────┐
            ▼              ▼              ▼
   ~/.claude/skills/  ~/.config/     ~/.agents/skills/
                      opencode/
                      skills/
        ▲                 ▲               ▲
        │                 │               │
   Claude Code       OpenCode        Any tool using
                                     .agents/ convention
```

### Three layers

| Layer        | Concern           | Artifact            |
|------------- |-------------------|---------------------|
| **Storage**  | Skill definitions | `skills/<name>/SKILL.md` |
| **Activation** | What's enabled where | `skills.toml`    |
| **Delivery** | Tool discovery    | `install.sh` symlinks |

### Why this separation

Skills are version-controlled content. Which skills are active in which
context is a configuration concern. How tools discover them is a delivery
concern. Mixing these makes skills hard to share, hard to override, and
hard to audit.

## Compatibility

Both OpenCode and Claude Code discover skills from:

| Path                              | Scope   | Tool         |
|-----------------------------------|---------|--------------|
| `.claude/skills/<name>/SKILL.md`  | Project | Both         |
| `~/.claude/skills/<name>/SKILL.md`| Global  | Both         |
| `.opencode/skills/<name>/SKILL.md`| Project | OpenCode     |
| `~/.config/opencode/skills/<name>/SKILL.md` | Global | OpenCode |
| `.agents/skills/<name>/SKILL.md`  | Project | Both         |
| `~/.agents/skills/<name>/SKILL.md`| Global  | Both         |

The install script symlinks enabled skills into the appropriate discovery
paths. Claude Code extensions in frontmatter (`disable-model-invocation`,
`context`, `allowed-tools`) are ignored by OpenCode (unknown fields are
silently skipped).

## SKILL.md format

```yaml
---
name: skill-name          # required, must match directory name
description: What it does  # required, 1-1024 chars
# --- optional, Claude Code extensions (ignored by OpenCode) ---
disable-model-invocation: true
user-invocable: false
allowed-tools: Read, Grep
context: fork
agent: Explore
# --- optional, OpenCode extensions (ignored by Claude Code) ---
license: MIT
compatibility: opencode
metadata:
  audience: developers
---

Markdown content with instructions.
```

## Name validation

Names must be lowercase alphanumeric with single-hyphen separators:

```
^[a-z0-9]+(-[a-z0-9]+)*$
```

- 1-64 characters
- No leading/trailing hyphens
- No consecutive hyphens
- Must match the containing directory name

## skills.toml

See `skills.toml` at repo root for the full annotated configuration.

## Adding a skill

1. Create `skills/<name>/SKILL.md` with valid frontmatter
2. Add the skill to `skills.toml` under the appropriate scope
3. Run `./scripts/install.sh` to link it into discovery paths
