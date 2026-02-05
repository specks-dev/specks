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

# Release a new version
release VERSION:
    ./scripts/release.sh {{VERSION}}
