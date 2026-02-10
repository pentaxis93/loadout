# Loadout — Design

Architecture and rationale for the skill management system.
For usage instructions, see [README.md](README.md).

## Architecture

```
~/.config/loadout/              YOUR CONFIG (private, XDG-compliant)
├── loadout.toml                 what's enabled and where
└── skills/                      your skill definitions
    └── <name>/SKILL.md

loadout repo                    THE TOOL (public, git-managed)
├── scripts/                     install, validate, new
├── schema/                      JSON Schema for SKILL.md
├── skills/_template/            starter template
└── loadout.example.toml         annotated config example

         ↓ install.sh reads loadout.toml, resolves skills
           from source dirs, symlinks into targets

~/.claude/skills/<name>/        DISCOVERY PATHS (tool-specific)
~/.config/opencode/skills/<name>/
~/.agents/skills/<name>/
```

## Configuration location

The config file is resolved in this order:

1. `$LOADOUT_CONFIG` environment variable (if set)
2. `$XDG_CONFIG_HOME/loadout/loadout.toml` (if `XDG_CONFIG_HOME` is set)
3. `~/.config/loadout/loadout.toml` (default)

This means you can maintain multiple loadouts and switch between them:

```bash
LOADOUT_CONFIG=~/work-loadout.toml ./scripts/install.sh
```

## Three layers

| Layer | Concern | Location |
|-------|---------|----------|
| **Storage** | Skill definitions | `~/.config/loadout/skills/` (or any source dir) |
| **Activation** | What's enabled where | `~/.config/loadout/loadout.toml` |
| **Delivery** | Tool discovery | `install.sh` symlinks into target paths |

## Why separate the tool from the skills

The public repo is the engine. Your skills are the fuel. Mixing them
means you can't share the tool without exposing your personal workflow,
and you can't version your skills without pulling in tool updates.

Separation gives you:
- **Public repo stays clean** — no personal skills leak into commits
- **XDG compliance** — config in `~/.config/loadout/`, not scattered
- **Multiple sources** — layer personal, team, and community skills
- **Independent versioning** — tool updates don't touch your skills

## Skill resolution

When `install.sh` encounters a skill name, it searches configured
source directories in order. First match wins. This lets you:

```toml
[sources]
skills = [
  "~/.config/loadout/skills",        # personal (highest priority)
  "/team/shared-skills/skills",       # team
  "/path/to/loadout/skills",          # community (lowest priority)
]
```

Override a team skill by creating one with the same name in your
personal directory. The install script will use yours.

## Compatibility

Both OpenCode and Claude Code discover skills from:

| Path | Scope | Tool |
|------|-------|------|
| `.claude/skills/<name>/SKILL.md` | Project | Both |
| `~/.claude/skills/<name>/SKILL.md` | Global | Both |
| `.opencode/skills/<name>/SKILL.md` | Project | OpenCode |
| `~/.config/opencode/skills/<name>/SKILL.md` | Global | OpenCode |
| `.agents/skills/<name>/SKILL.md` | Project | Both |
| `~/.agents/skills/<name>/SKILL.md` | Global | Both |

The install script symlinks into the appropriate target paths.
Claude Code extensions in frontmatter (`disable-model-invocation`,
`context`, `allowed-tools`) are ignored by OpenCode (unknown fields
are silently skipped).

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
