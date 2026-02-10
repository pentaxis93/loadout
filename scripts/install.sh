#!/usr/bin/env bash
set -euo pipefail

# install.sh — Link enabled skills into tool discovery paths.
#
# Reads skills.toml to determine which skills to enable at each scope,
# then creates symlinks in the appropriate directories so that OpenCode,
# Claude Code, and other compatible tools can discover them.
#
# Usage:
#   ./scripts/install.sh              # apply skills.toml
#   ./scripts/install.sh --dry-run    # show what would happen
#   ./scripts/install.sh --clean      # remove all managed symlinks
#   ./scripts/install.sh --list       # list enabled skills per scope

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SKILLS_DIR="$REPO_DIR/skills"
CONFIG="$REPO_DIR/skills.toml"
MARKER=".managed-by-loadout"
DRY_RUN=false
CLEAN=false
LIST=false

# ── Parse args ──────────────────────────────────────────────────────────

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=true ;;
    --clean)   CLEAN=true ;;
    --list)    LIST=true ;;
    --help|-h)
      echo "Usage: install.sh [--dry-run] [--clean] [--list]"
      echo ""
      echo "  --dry-run  Show what would happen without making changes"
      echo "  --clean    Remove all managed symlinks from target directories"
      echo "  --list     List enabled skills per scope and exit"
      echo ""
      echo "Reads skills.toml from the repo root."
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

# ── Dependency check ────────────────────────────────────────────────────

if ! command -v python3 &>/dev/null; then
  echo "error: python3 is required to parse TOML" >&2
  echo "Install it or use your system package manager." >&2
  exit 1
fi

if [ ! -f "$CONFIG" ]; then
  echo "error: $CONFIG not found" >&2
  exit 1
fi

# ── TOML parser (uses Python's tomllib/tomli) ───────────────────────────

parse_toml() {
  python3 -c "
import sys, json
try:
    import tomllib
except ImportError:
    try:
        import tomli as tomllib
    except ImportError:
        print('error: Python 3.11+ or tomli package required', file=sys.stderr)
        sys.exit(1)

with open('$CONFIG', 'rb') as f:
    config = tomllib.load(f)
print(json.dumps(config))
"
}

CONFIG_JSON="$(parse_toml)"

# ── Extract config ──────────────────────────────────────────────────────

global_targets() {
  echo "$CONFIG_JSON" | python3 -c "
import sys, json
config = json.load(sys.stdin)
for t in config.get('global', {}).get('targets', []):
    print(t)
"
}

global_skills() {
  echo "$CONFIG_JSON" | python3 -c "
import sys, json
config = json.load(sys.stdin)
for s in config.get('global', {}).get('skills', []):
    print(s)
"
}

project_entries() {
  echo "$CONFIG_JSON" | python3 -c "
import sys, json
config = json.load(sys.stdin)
projects = config.get('projects', {})
for path, cfg in projects.items():
    inherit = cfg.get('inherit', True)
    skills = ','.join(cfg.get('skills', []))
    print(f'{path}|{inherit}|{skills}')
"
}

# ── Validation ──────────────────────────────────────────────────────────

validate_name() {
  local name="$1"
  if [[ ! "$name" =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]]; then
    echo "error: invalid skill name '$name'" >&2
    echo "  must be lowercase alphanumeric with single-hyphen separators" >&2
    return 1
  fi
  if [ ${#name} -gt 64 ]; then
    echo "error: skill name '$name' exceeds 64 characters" >&2
    return 1
  fi
}

validate_skill() {
  local name="$1"
  local skill_dir="$SKILLS_DIR/$name"
  local skill_file="$skill_dir/SKILL.md"

  validate_name "$name" || return 1

  if [ ! -d "$skill_dir" ]; then
    echo "error: skill directory not found: $skill_dir" >&2
    return 1
  fi
  if [ ! -f "$skill_file" ]; then
    echo "error: SKILL.md not found: $skill_file" >&2
    return 1
  fi
}

# ── Actions ─────────────────────────────────────────────────────────────

expand_path() {
  echo "${1/#\~/$HOME}"
}

place_marker() {
  local target_dir="$1"
  local marker_file="$target_dir/$MARKER"
  if [ "$DRY_RUN" = true ]; then
    echo "  [dry-run] would create marker: $marker_file"
  else
    echo "$REPO_DIR" > "$marker_file"
  fi
}

link_skill() {
  local name="$1"
  local target_dir="$2"
  local dest="$(expand_path "$target_dir")/$name"
  local src="$SKILLS_DIR/$name"

  if [ "$DRY_RUN" = true ]; then
    echo "  [dry-run] $src -> $dest"
    return
  fi

  mkdir -p "$(expand_path "$target_dir")"

  # Remove existing (managed) symlink or directory
  if [ -L "$dest" ]; then
    rm "$dest"
  elif [ -d "$dest" ]; then
    # Only remove if it's managed by us (has marker in parent)
    local parent_marker="$(expand_path "$target_dir")/$MARKER"
    if [ -f "$parent_marker" ]; then
      rm -rf "$dest"
    else
      echo "  warning: $dest exists and is not managed, skipping" >&2
      return
    fi
  fi

  ln -s "$src" "$dest"
  echo "  linked: $name -> $dest"
}

clean_target() {
  local target_dir="$(expand_path "$1")"
  local marker_file="$target_dir/$MARKER"

  if [ ! -f "$marker_file" ]; then
    return
  fi

  if [ "$DRY_RUN" = true ]; then
    echo "  [dry-run] would clean managed symlinks in $target_dir"
    return
  fi

  # Remove symlinks that point into our skills directory
  for entry in "$target_dir"/*/; do
    entry="${entry%/}"
    if [ -L "$entry" ]; then
      local link_target="$(readlink "$entry")"
      if [[ "$link_target" == "$SKILLS_DIR"/* ]]; then
        rm "$entry"
        echo "  removed: $entry"
      fi
    fi
  done

  rm "$marker_file"
  echo "  removed marker: $marker_file"

  # Remove directory if empty
  if [ -d "$target_dir" ] && [ -z "$(ls -A "$target_dir")" ]; then
    rmdir "$target_dir"
    echo "  removed empty: $target_dir"
  fi
}

# ── List mode ───────────────────────────────────────────────────────────

if [ "$LIST" = true ]; then
  echo "=== Global skills ==="
  echo "Targets:"
  global_targets | while read -r t; do
    echo "  $(expand_path "$t")"
  done
  echo "Skills:"
  global_skills | while read -r s; do
    if [ -n "$s" ]; then
      echo "  $s"
    fi
  done
  if [ -z "$(global_skills)" ]; then
    echo "  (none)"
  fi

  echo ""
  echo "=== Project skills ==="
  project_entries | while IFS='|' read -r path inherit skills; do
    if [ -n "$path" ]; then
      echo "  $path (inherit=$inherit): $skills"
    fi
  done
  if [ -z "$(project_entries)" ]; then
    echo "  (none configured)"
  fi
  exit 0
fi

# ── Clean mode ──────────────────────────────────────────────────────────

if [ "$CLEAN" = true ]; then
  echo "Cleaning managed symlinks..."
  global_targets | while read -r t; do
    clean_target "$t"
  done
  project_entries | while IFS='|' read -r path inherit skills; do
    if [ -n "$path" ]; then
      clean_target "$path/.claude/skills"
      clean_target "$path/.opencode/skills"
      clean_target "$path/.agents/skills"
    fi
  done
  echo "Done."
  exit 0
fi

# ── Install mode ────────────────────────────────────────────────────────

echo "Installing skills from $CONFIG..."

# Validate all skills first
errors=0
global_skills | while read -r s; do
  if [ -n "$s" ]; then
    validate_skill "$s" || errors=$((errors + 1))
  fi
done

project_entries | while IFS='|' read -r path inherit skills; do
  IFS=',' read -ra skill_array <<< "$skills"
  for s in "${skill_array[@]}"; do
    if [ -n "$s" ]; then
      validate_skill "$s" || errors=$((errors + 1))
    fi
  done
done

# Link global skills
echo ""
echo "--- Global scope ---"
global_targets | while read -r target; do
  expanded="$(expand_path "$target")"
  echo "Target: $expanded"
  mkdir -p "$expanded"
  place_marker "$expanded"
  global_skills | while read -r s; do
    if [ -n "$s" ]; then
      link_skill "$s" "$target"
    fi
  done
done

# Link project skills
project_entries | while IFS='|' read -r path inherit skills; do
  if [ -z "$path" ]; then continue; fi

  echo ""
  echo "--- Project: $path ---"

  # Determine project targets
  for subdir in ".claude/skills" ".opencode/skills" ".agents/skills"; do
    project_target="$path/$subdir"
    expanded="$(expand_path "$project_target")"
    echo "Target: $expanded"
    mkdir -p "$expanded"
    place_marker "$expanded"

    # If inheriting, link global skills first
    if [ "$inherit" = "True" ] || [ "$inherit" = "true" ]; then
      global_skills | while read -r gs; do
        if [ -n "$gs" ]; then
          link_skill "$gs" "$project_target"
        fi
      done
    fi

    # Link project-specific skills
    IFS=',' read -ra skill_array <<< "$skills"
    for s in "${skill_array[@]}"; do
      if [ -n "$s" ]; then
        link_skill "$s" "$project_target"
      fi
    done
  done
done

echo ""
echo "Done. Skills are now discoverable by OpenCode and Claude Code."
