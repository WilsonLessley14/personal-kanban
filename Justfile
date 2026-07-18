# Justfile — the single source of truth for validation commands.
#
# Spec validation commands should invoke `just <recipe>` (not raw `cargo
# clippy`, `rustup`, etc). Every tool these recipes call is provided by the
# flake devShell, so a builder agent running inside `nix develop` (or with
# direnv active) always has them on PATH. If a recipe needs a new tool, add it
# to flake.nix's devShell — do not install it at runtime.

# List available recipes
default:
    @just --list

# Format all code in the repository
format:
    cargo fmt
    alejandra .

# Check formatting without modifying files (CI gate)
check-format:
    cargo fmt --check
    alejandra --check .

# Lint all code, warnings are errors
lint:
    cargo clippy -- -D warnings
    statix check .

# Run all tests
test:
    cargo test

# Build a release binary
build:
    cargo build --release

# Test coverage report
coverage:
    cargo tarpaulin --out Stdout

# Full validation gate — the CI equivalent. Spec checkpoints that want a
# "everything passes" gate should validate with `just validate`.
validate: check-format lint test build
