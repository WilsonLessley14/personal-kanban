# Spec: fix-edit-field-cycle-priority

## Context
Domain: ./DOMAIN.md
Proposed domain changes: none

Rust TUI, pure input handler architecture. Validation gate: `cargo test`, `cargo clippy -- -D warnings`, `just validate`.

## Checkpoint: 1-fix-edit-field-priority-cycle
Goal: Restrict CyclePriority key bindings to the priority field only in EditField mode.

### Requirements
- Update `handle_input` in `src/tui/input.rs` to accept an additional parameter `edit_field: usize`.
- Update `handle_edit_field` to accept `edit_field: usize` and only map `Tab` and `p/P` to `Action::CyclePriority` when `edit_field == 2`.
- When `edit_field` is 0 or 1 (title/description), `Tab` should return `Action::None` and `p/P` should return `Action::InsertText`.
- Update the call site in `src/tui/mod.rs` to pass `app.edit_field` as the fourth argument to `handle_input`.
- Update all test call sites in `src/tui/input.rs` to pass the `edit_field` argument.
- Add tests verifying that `p/P` insert text on title field (0) and description field (1), and `Tab` is ignored on text fields.

### Validation
```validation
cargo build 2>&1
cargo test input:: 2>&1
cargo clippy -- -D warnings 2>&1
just validate
```

### Notes
The code changes have already been made by the orchestrator. This checkpoint validates them through the builder/reviewer loop.
