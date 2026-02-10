---
name: _template
description: >-
  Template skill. Copy this directory, rename it, and replace this
  content with your own. Delete this skill once you have a real one.
---

# _template

This is a placeholder. To create your first skill:

1. Copy this directory:
   ```
   cp -r skills/_template skills/your-skill-name
   ```

2. Edit `skills/your-skill-name/SKILL.md`:
   - Set `name:` to match the directory name (lowercase, hyphens only)
   - Write a `description:` that helps the agent decide when to use it
   - Replace everything below the frontmatter with your instructions

3. Enable it in `skills.toml`:
   ```toml
   [global]
   skills = ["your-skill-name"]
   ```

4. Run `./scripts/install.sh` to link it into discovery paths.

## Name rules

- Lowercase alphanumeric with single-hyphen separators
- 1-64 characters
- Must match the directory name
- Regex: `^[a-z0-9]+(-[a-z0-9]+)*$`

## Frontmatter fields

Required:
- `name` — skill identifier
- `description` — what it does and when to use it (1-1024 chars)

Optional (Claude Code, ignored by OpenCode):
- `disable-model-invocation: true` — only user can invoke via /name
- `user-invocable: false` — only the model can invoke, hidden from / menu
- `allowed-tools: Read, Grep` — tools permitted without approval
- `context: fork` — run in isolated subagent
- `agent: Explore` — which subagent type for context: fork

Optional (OpenCode, ignored by Claude Code):
- `license: MIT`
- `compatibility: opencode`
- `metadata:` — string-to-string map
