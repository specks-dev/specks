# Specks development commands

default:
    @just --list

# Development
build:
    cargo build --workspace

test:
    cargo nextest run --workspace

# Quality
fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace -- -D warnings

check:
    cargo check --workspace

# CI (runs all checks)
ci: fmt lint test

# Release
build-release:
    cargo build --release

# Run the CLI
run *ARGS:
    cargo run -p specks -- {{ARGS}}

# Update golden files (for intentional schema changes)
update-golden:
    SPECKS_UPDATE_GOLDEN=1 cargo nextest run -p specks golden

# Generate documentation
doc:
    cargo doc --workspace --open

# Install locally
install:
    cargo install --path crates/specks

# Release a new version (e.g., just release 0.1.1)
release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail

    VERSION="{{VERSION}}"
    VERSION="${VERSION#v}"

    # Validate semver format
    if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "Error: Invalid version format. Expected X.Y.Z" >&2
        exit 1
    fi

    # Must be on main
    if [[ "$(git branch --show-current)" != "main" ]]; then
        echo "Error: Must be on main branch" >&2
        exit 1
    fi

    # Check if this version was already successfully released
    if gh release view "v$VERSION" &>/dev/null; then
        echo "Error: v$VERSION already released. Bump version number." >&2
        exit 1
    fi

    # Clean up orphaned remote tag from failed release (if any)
    if git ls-remote --tags origin | grep -q "refs/tags/v$VERSION$"; then
        echo "Cleaning up failed release tag..."
        git push origin ":refs/tags/v$VERSION" 2>/dev/null || true
    fi

    # Version must be >= current (equal allowed for retry)
    CURRENT=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    IFS='.' read -r CUR_MAJ CUR_MIN CUR_PAT <<< "$CURRENT"
    IFS='.' read -r NEW_MAJ NEW_MIN NEW_PAT <<< "$VERSION"

    if [[ $NEW_MAJ -lt $CUR_MAJ ]] || \
       [[ $NEW_MAJ -eq $CUR_MAJ && $NEW_MIN -lt $CUR_MIN ]] || \
       [[ $NEW_MAJ -eq $CUR_MAJ && $NEW_MIN -eq $CUR_MIN && $NEW_PAT -lt $CUR_PAT ]]; then
        echo "Error: $VERSION must be >= current ($CURRENT)" >&2
        exit 1
    fi

    # Auto-fix: format and lint
    cargo fmt --all
    cargo clippy --workspace --fix --allow-dirty --allow-staged -- -D warnings

    # Verify clean build (warnings are errors)
    cargo clippy --workspace -- -D warnings

    # Auto-fix: update version in Cargo.toml
    sed -i '' "s/^version = .*/version = \"$VERSION\"/" Cargo.toml

    # Auto-fix: stage and commit any changes
    git add -A
    git diff --cached --quiet || git commit -m "Release $VERSION"

    # Delete local tag if present (failed release retry)
    git tag -d "v$VERSION" 2>/dev/null || true

    # Sync with origin
    git pull --rebase origin main || true
    git push origin main

    # Tag and push
    git tag "v$VERSION"
    git push origin "v$VERSION"

    echo ""
    echo "Released v$VERSION"
    echo "https://github.com/kocienda/specks/actions"
