#!/usr/bin/env bash
set -euo pipefail

# new.sh — Scaffold a new skill directory.
#
# Usage:
#   ./scripts/new.sh my-skill-name "Short description of what it does"

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SKILLS_DIR="$REPO_DIR/skills"

if [ $# -lt 1 ]; then
  echo "Usage: ./scripts/new.sh <skill-name> [description]"
  echo ""
  echo "Example:"
  echo "  ./scripts/new.sh git-commit \"Create conventional commits with scope and body\""
  exit 1
fi

NAME="$1"
DESC="${2:-TODO: describe what this skill does and when to use it}"

# Validate name
if [[ ! "$NAME" =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]]; then
  echo "error: invalid skill name '$NAME'" >&2
  echo "  must be lowercase alphanumeric with single-hyphen separators" >&2
  echo "  regex: ^[a-z0-9]+(-[a-z0-9]+)*\$" >&2
  exit 1
fi

if [ ${#NAME} -gt 64 ]; then
  echo "error: skill name exceeds 64 characters" >&2
  exit 1
fi

SKILL_DIR="$SKILLS_DIR/$NAME"

if [ -d "$SKILL_DIR" ]; then
  echo "error: $SKILL_DIR already exists" >&2
  exit 1
fi

mkdir -p "$SKILL_DIR"

cat > "$SKILL_DIR/SKILL.md" << EOF
---
name: $NAME
description: >-
  $DESC
---

# $NAME

<!-- Replace this with your skill instructions. -->
<!-- The content below the frontmatter is what the agent receives -->
<!-- when it loads this skill. -->

## What I do

- TODO

## When to use me

TODO
EOF

echo "Created $SKILL_DIR/SKILL.md"
echo ""
echo "Next steps:"
echo "  1. Edit $SKILL_DIR/SKILL.md with your instructions"
echo "  2. Add '$NAME' to skills.toml under [global] or a project"
echo "  3. Run ./scripts/install.sh to link it"
echo "  4. Run ./scripts/validate.sh $NAME to check"
