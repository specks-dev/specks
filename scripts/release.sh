#!/bin/bash
#
# Release specks
#
# Usage: ./scripts/release.sh <version>
#
# Examples:
#   ./scripts/release.sh 0.1.1
#   ./scripts/release.sh v0.2.0
#
# The script validates everything before making changes.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

error() { echo -e "${RED}Error: $1${NC}" >&2; exit 1; }
warn() { echo -e "${YELLOW}$1${NC}"; }
info() { echo -e "${GREEN}$1${NC}"; }

# --- Argument validation ---

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <version>" >&2
    echo "Example: $0 0.1.1" >&2
    exit 1
fi

# Strip 'v' prefix if provided
VERSION="${1#v}"

# Validate semver format (X.Y.Z)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    error "Invalid version format: '$VERSION'. Expected X.Y.Z (e.g., 0.1.1)"
fi

# --- Git state validation ---

# Must be on main branch
BRANCH=$(git branch --show-current)
if [[ "$BRANCH" != "main" ]]; then
    error "Must be on main branch (currently on '$BRANCH')"
fi

# Working directory must be clean
if ! git diff --quiet HEAD; then
    error "Working directory has uncommitted changes. Commit or stash them first."
fi

# Must be up to date with origin
git fetch origin main --quiet
LOCAL=$(git rev-parse HEAD)
REMOTE=$(git rev-parse origin/main)
if [[ "$LOCAL" != "$REMOTE" ]]; then
    error "Local main is not up to date with origin. Run 'git pull' first."
fi

# Tag must not already exist
if git rev-parse "v$VERSION" >/dev/null 2>&1; then
    error "Tag v$VERSION already exists"
fi

# --- Version validation ---

CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

# Compare versions (split into parts)
IFS='.' read -r CUR_MAJOR CUR_MINOR CUR_PATCH <<< "$CURRENT_VERSION"
IFS='.' read -r NEW_MAJOR NEW_MINOR NEW_PATCH <<< "$VERSION"

version_gt() {
    if [[ $NEW_MAJOR -gt $CUR_MAJOR ]]; then return 0; fi
    if [[ $NEW_MAJOR -lt $CUR_MAJOR ]]; then return 1; fi
    if [[ $NEW_MINOR -gt $CUR_MINOR ]]; then return 0; fi
    if [[ $NEW_MINOR -lt $CUR_MINOR ]]; then return 1; fi
    if [[ $NEW_PATCH -gt $CUR_PATCH ]]; then return 0; fi
    return 1
}

if ! version_gt; then
    error "New version ($VERSION) must be greater than current version ($CURRENT_VERSION)"
fi

# --- Confirmation ---

echo ""
info "Release Summary"
echo "  Current version: $CURRENT_VERSION"
echo "  New version:     $VERSION"
echo "  Tag:             v$VERSION"
echo ""
read -p "Proceed with release? [y/N] " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    warn "Aborted."
    exit 0
fi

# --- Execute release ---

info "Updating Cargo.toml..."
sed -i '' "s/^version = \"$CURRENT_VERSION\"/version = \"$VERSION\"/" Cargo.toml

info "Committing..."
git add Cargo.toml
git commit -m "Release $VERSION"

info "Tagging v$VERSION..."
git tag "v$VERSION"

info "Pushing to origin..."
git push origin main "v$VERSION"

echo ""
info "Released v$VERSION"
echo "CI will build binaries and update the Homebrew formula."
echo "Watch progress: https://github.com/kocienda/specks/actions"
