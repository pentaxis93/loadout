#!/usr/bin/env bash
set -euo pipefail

# validate.sh — Check all SKILL.md files for correctness.
#
# Validates:
#   - Directory name matches frontmatter 'name' field
#   - Name passes regex validation
#   - Required frontmatter fields present (name, description)
#   - Description length within bounds (1-1024 chars)
#
# Usage:
#   ./scripts/validate.sh              # validate all skills
#   ./scripts/validate.sh skill-name   # validate one skill

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SKILLS_DIR="$REPO_DIR/skills"

errors=0
checked=0

validate_one() {
  local dir_name="$1"
  local skill_dir="$SKILLS_DIR/$dir_name"
  local skill_file="$skill_dir/SKILL.md"

  if [ ! -f "$skill_file" ]; then
    echo "FAIL [$dir_name]: SKILL.md not found at $skill_file"
    errors=$((errors + 1))
    return
  fi

  checked=$((checked + 1))

  # Extract and validate using Python (handles YAML parsing)
  python3 -c "
import sys, re

# Read the file
with open('$skill_file') as f:
    content = f.read()

# Extract frontmatter
if not content.startswith('---'):
    print('FAIL [$dir_name]: no YAML frontmatter (must start with ---)')
    sys.exit(1)

parts = content.split('---', 2)
if len(parts) < 3:
    print('FAIL [$dir_name]: malformed frontmatter (missing closing ---)')
    sys.exit(1)

# Parse YAML
try:
    import yaml
except ImportError:
    # Fallback: basic key-value parsing
    fm = {}
    for line in parts[1].strip().split('\n'):
        line = line.strip()
        if ':' in line and not line.startswith('#'):
            key, _, val = line.partition(':')
            fm[key.strip()] = val.strip().strip('\"').strip(\"'\")
else:
    fm = yaml.safe_load(parts[1]) or {}

errors = 0

# Check required fields
if 'name' not in fm:
    print('FAIL [$dir_name]: missing required field: name')
    errors += 1

if 'description' not in fm:
    print('FAIL [$dir_name]: missing required field: description')
    errors += 1

if errors:
    sys.exit(1)

name = str(fm['name'])
desc = str(fm['description'])

# Validate name format
name_re = r'^[a-z0-9]+(-[a-z0-9]+)*$'
if not re.match(name_re, name):
    print(f'FAIL [$dir_name]: name \"{name}\" does not match {name_re}')
    errors += 1

if len(name) > 64:
    print(f'FAIL [$dir_name]: name exceeds 64 characters ({len(name)})')
    errors += 1

# Validate name matches directory
if name != '$dir_name':
    print(f'FAIL [$dir_name]: name \"{name}\" does not match directory \"$dir_name\"')
    errors += 1

# Validate description length
if len(desc) > 1024:
    print(f'FAIL [$dir_name]: description exceeds 1024 characters ({len(desc)})')
    errors += 1

if len(desc) == 0:
    print(f'FAIL [$dir_name]: description is empty')
    errors += 1

if errors:
    sys.exit(1)

print(f'  OK  [{name}]: \"{desc[:60]}{\"...\" if len(desc) > 60 else \"\"}\"')
" || errors=$((errors + 1))
}

echo "Validating skills in $SKILLS_DIR..."
echo ""

if [ $# -gt 0 ]; then
  # Validate specific skill
  validate_one "$1"
else
  # Validate all skills
  for skill_dir in "$SKILLS_DIR"/*/; do
    dir_name="$(basename "$skill_dir")"
    validate_one "$dir_name"
  done
fi

echo ""
if [ $errors -gt 0 ]; then
  echo "FAILED: $errors error(s) in $checked skill(s)"
  exit 1
else
  echo "PASSED: $checked skill(s) validated"
fi
