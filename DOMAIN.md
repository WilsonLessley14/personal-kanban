# Domain: personal-kanban

## Purpose

A terminal-based personal kanban board backed by SQLite. It provides a CLI (`kanban`
binary) and a TUI for managing tasks across columns with priorities and reordering.

**Out of scope:** sharing boards across a network, multiple boards, tags/labels beyond
title/description/priority, due dates, attachments, or collaboration features.

## Contracts

### Surfaced

- **CLI:** `kanban` binary with `clap`-derived subcommands. Every TUI action has an
  equivalent CLI subcommand and vice versa.
- **TUI modes:** `Mode::Normal`, `Mode::Insert`, `Mode::ViewTask`, `Mode::EditField`,
  `Mode::Column`, `Mode::Confirm`, `Mode::Help`, `Mode::Move`.
- **TUI action dispatch:** `handle_input(key, mode, confirm_context) -> Action` in
  `src/tui/input.rs` (pure function), executed by `execute_action(app, db, action)` in
  `src/tui/mod.rs`.
- **Database schema:** SQLite with migrations in `migrations/`. Board, columns, tasks,
  and priorities tables.

### Respected

- **Three-layer architecture:** `src/core/` (pure types, parsing, SQL constructors,
  validation — no I/O) ← `src/shell/` (SQLite operations, filesystem, interactive
  prompts) ← `src/tui/` + `src/cli/` (user-facing frontends).
- **State reload:** after any mutation, board state is reloaded fresh from SQLite. The
  TUI never mutates `app.state` directly — it calls shell operations and reloads.
- **Input → Action separation:** `handle_input` in `src/tui/input.rs` is a pure function
  mapping `(KeyEvent, Mode) -> Action`. `execute_action` in `src/tui/mod.rs` handles the
  imperative side-effects.
- **Vim-like keybindings:** `j`/`k` for downward/upward navigation, `h`/`l` for
  left/right, `i` for enter edit, `esc` for cancel/quit.

### Required

- `ratatui` for terminal rendering, `crossterm` for input/terminal control, `rusqlite`
  with `bundled` SQLite.
- `just` recipes (`just validate`) as the CI-equivalent validation gate.

## Testing Commitments

- **Unit tests** in `#[cfg(test)]` modules co-located with source. `cargo test` must pass.
- **Input handling** (`src/tui/input.rs`) must have tests covering all mode/key mappings.
- **Linting:** `cargo clippy -- -D warnings` and `cargo fmt --check` must pass.
- **Full gate:** `just validate` (format + lint + test + build) is the acceptance bar.
- No E2E or TUI integration tests by choice — the pure `handle_input` function is the
  test surface for behavioral correctness.
