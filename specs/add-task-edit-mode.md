# Spec: add-task-edit-mode

## Context

The TUI lives in `src/tui/`. The entry point for action dispatch is `src/tui/mod.rs` where `handle_action` matches on `Action` variants.

**Key files:**
- `src/tui/mod.rs` — action handlers (`Action::AddTask`, `Action::Save`, `Action::CyclePriority`, etc.)
- `src/tui/app.rs` — `Mode` enum (`Normal`, `Insert`, `Edit`, `Column`, `Confirm`, `Help`, `Move`) and `Action` enum
- `src/shell/ops.rs` — `add_task(db, column_id_prefix, title, desc, priority_id_prefix)` and `edit_task(db, task_id_prefix, changes)`
- `src/core/types.rs` — `Task` struct with fields: `id`, `column_id`, `title`, `description`, `priority_id`, `position`, `created_at`, `updated_at`
- `src/shell/ops.rs` — `load_board_state_for_tasks()` returns `BoardState` with `priorities: Vec<Priority>`

**Current AddTask flow:** `Action::AddTask` sets `Mode::Insert` with empty buffer. On `Action::Save` in `Mode::Insert`, a task is created with the input buffer as title, empty description, and default "medium" priority.

**Current EditTask flow:** `Action::EditTask` clones the focused task into `editing_task`, sets `edit_field = 0`, populates `input_buffer` with `task.title`, and enters `Mode::Edit`. User can Tab-cycle fields (title → description → priority). On `Action::Save` in `Mode::Edit`, the current field is saved, the task is diffed against original, and `shell::edit_task` is called.

**CyclePriority:** Works whenever `editing_task` is `Some`. It cycles through available priorities by name, updating `editing_task.priority_id`.

**The Save handler for Mode::Insert** currently also handles column operations (column add via `editing_task.id.is_empty()` and column rename via `editing_task.position == -1`). This column logic in `Mode::Insert` must be preserved.

## Checkpoint: add-task-edit-mode

Goal: Refactor `Action::AddTask` to use `Mode::Edit` with a new empty Task, so users can fill in title, description, and priority before saving.

### Requirements

**1. Modify `Action::AddTask` in `src/tui/mod.rs` (around line 200):**

Instead of entering `Mode::Insert`, the handler should:
- Create a new `Task` with all empty/default fields (`id: ""`, `column_id: ""`, `title: ""`, `description: ""`, `priority_id: ""`, `position: 0`, `created_at: ""`, `updated_at: ""`)
- Set `app.editing_task = Some(task)`
- Set `app.edit_field = 0` (start on title)
- Clear `app.input_buffer`
- Set `app.mode = Mode::Edit`

**2. Modify `Action::Save` when `app.mode == Mode::Edit` (around line 606):**

The existing `Mode::Edit` Save block handles editing an existing task. Add a new branch at the top of this block to handle the case when `editing_task.id` is empty (new task creation):

- After saving the current field from `input_buffer` into `editing_task`:
- Validate title is non-empty (reuse existing check)
- Look up the focused column from `app.state` and `app.focused_col_idx`
- Resolve the priority: if `editing_task.priority_id` is non-empty, use it. Otherwise, default to the "medium" priority from `state.priorities`
- Look up the priority name from `state.priorities` by matching `editing_task.priority_id`
- Call `shell::add_task(db, &col.name, &title, &description, &priority_name)`
- On success: set `mode = Mode::Normal`, clear `editing_task` and `input_buffer`, return `(true, false)`
- On error: set error message, return `(false, false)`

The existing edit-task branch (when `task.id` is non-empty) must remain unchanged.

**3. Preserve all existing behavior:**

- `Action::CyclePriority` already works for any `editing_task` — no changes needed
- `Action::CycleField` already works for `Mode::Edit` — no changes needed
- Column add/rename in `Mode::Insert` must continue to work (the column-mode handlers enter `Mode::Insert`, not `Mode::Edit`)
- `Action::EditTask` (editing an existing task) must continue to work unchanged
- `Action::Cancel` already clears `editing_task` and returns to `Mode::Normal` — no changes needed

### Validation

```validation
cargo build 2>&1
cargo test tui:: -- --nocapture 2>&1
cargo clippy -- -D warnings 2>&1
```

### Notes

The priority for a new task should default to "medium" if the user hasn't cycled to another priority. The `editing_task.priority_id` starts empty, so in the Save handler, check: if `editing_task.priority_id` is empty, look up the "medium" priority id from `state.priorities` and use that.

The user experience should be: press `a` → Enter edit mode with empty title → type title → Tab to description → type description → Tab to priority → (priority shows current selection, can cycle with appropriate key) → Enter to save → task is created with all three fields populated.
