#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# --- Prerequisites ---

if [[ ! -f "Cargo.toml" ]]; then
  echo "Error: Cargo.toml not found at repo root ($REPO_ROOT)" >&2
  exit 1
fi

if ! command -v git >/dev/null 2>&1; then
  echo "Error: git is not available on PATH" >&2
  exit 1
fi

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "Error: not inside a git repository" >&2
  exit 1
fi

if ! git diff-index --quiet HEAD -- 2>/dev/null; then
  echo "Error: working tree is dirty. Clean up before releasing:" >&2
  git status --short >&2
  exit 1
fi

# --- Extract current version from [package] section only ---

CURRENT_VERSION="$(awk '
  /^\[/ { section = substr($0, 2, index($0, "]") - 2) }
  section == "package" && /^version[[:space:]]*=[[:space:]]*"[^"]+"/ {
    match($0, /"([^"]+)"/, arr)
    print arr[1]
    exit
  }
' Cargo.toml)"

if [[ -z "$CURRENT_VERSION" ]]; then
  echo "Error: Could not find version in [package] section of Cargo.toml" >&2
  exit 1
fi

# --- Validate semver format ---

if [[ ! "$CURRENT_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: version '$CURRENT_VERSION' is not a strict X.Y.Z semver string" >&2
  exit 1
fi

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# --- Prompt user for bump type ---

echo "Current version: $CURRENT_VERSION"
echo "Select bump type:"
echo "  1) patch  -> $MAJOR.$MINOR.$((PATCH + 1))"
echo "  2) minor  -> $MAJOR.$((MINOR + 1)).0"
printf "Choice [1]: "
read -r CHOICE

CHOICE="${CHOICE:-1}"

case "$CHOICE" in
  1|patch|p)
    NEW_VERSION="$MAJOR.$MINOR.$((PATCH + 1))"
    ;;
  2|minor|m)
    NEW_VERSION="$MAJOR.$((MINOR + 1)).0"
    ;;
  *)
    echo "Error: invalid choice '$CHOICE' — expected 1/patch/p or 2/minor/m" >&2
    exit 1
    ;;
esac

# --- Check tag does not already exist (before committing) ---

if git rev-parse -q --verify "refs/tags/v$NEW_VERSION" >/dev/null 2>&1; then
  echo "Error: Tag v$NEW_VERSION already exists" >&2
  exit 1
fi

# --- Update Cargo.toml (section-aware, awk-based) ---

cp Cargo.toml Cargo.toml.bak

awk -v new_ver="$NEW_VERSION" '
  /^\[/ { section = substr($0, 2, index($0, "]") - 2) }
  section == "package" && /^version[[:space:]]*=[[:space:]]*"[^"]+"/ && !done {
    sub(/"[^"]+"/, "\"" new_ver "\"")
    done = 1
  }
  { print }
' Cargo.toml.bak > Cargo.toml

# --- Verify the rewrite was correct ---

WRITTEN_VERSION="$(awk '
  /^\[/ { section = substr($0, 2, index($0, "]") - 2) }
  section == "package" && /^version[[:space:]]*=[[:space:]]*"[^"]+"/ {
    match($0, /"([^"]+)"/, arr)
    print arr[1]
    exit
  }
' Cargo.toml)"

if [[ "$WRITTEN_VERSION" != "$NEW_VERSION" ]]; then
  echo "Error: Cargo.toml rewrite verification failed (got '$WRITTEN_VERSION', expected '$NEW_VERSION') — restoring backup" >&2
  mv Cargo.toml.bak Cargo.toml
  exit 1
fi

rm -f Cargo.toml.bak

# --- Commit and tag ---

git add Cargo.toml
git commit -m "chore: release v$NEW_VERSION"

COMMIT_SHORT="$(git rev-parse --short HEAD)"

git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"

# --- Summary ---

echo ""
echo "Released:"
echo "  old version: $CURRENT_VERSION"
echo "  new version: $NEW_VERSION"
echo "  commit:      $COMMIT_SHORT"
echo "  tag:         v$NEW_VERSION"
echo ""
echo "Next step: git push origin main --follow-tags"
