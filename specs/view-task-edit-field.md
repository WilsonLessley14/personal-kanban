# Spec: view-task-edit-field

## Context
Domain: ./DOMAIN.md
Proposed domain changes:
- Removes `Mode::Edit`, adds `Mode::ViewTask` and `Mode::EditField` to the TUI mode set
- Removes `Action::EditTask`, adds `Action::ViewTask`, `Action::EditField`, `Action::CycleFieldPrev`
- Removes `e` keybinding from `Mode::Normal`; adds `enter` for view-task, `i` for edit-field
- `tab`, `j`, `k` now cycle fields in `Mode::ViewTask` (j/Tab=next, k=prev)

Rust, ratatui/crossterm TUI. No new dependencies. Pure `handle_input` → `execute_action` dispatch
pattern in `src/tui/`. All validation via `just validate`.

**Key files:**
- `src/tui/app.rs` — `Mode` enum, `Action` enum, `App` struct
- `src/tui/input.rs` — `handle_input` (pure), per-mode handlers
- `src/tui/mod.rs` — `execute_action` (imperative), renders via `render_board`
- `src/tui/widgets/board.rs` — `render_board` dispatch, `render_status_bar` hints
- `src/tui/widgets/dialog.rs` — `render_edit_overlay` (edit/view dialog)
- `src/tui/widgets/help.rs` — `render_help_overlay`

**Existing state fields reused:**
- `app.editing_task: Option<Task>` — holds the task being viewed/edited (cloned from focused task)
- `app.edit_field: usize` — currently selected field (0=title, 1=description, 2=priority)
- `app.input_buffer: String` — text buffer for the active field during `EditField`

**EditField UX detail:** when on the priority field (edit_field == 2), `tab` and `p` cycle
through priorities (not to next field). `j`/`k` are inert on priority in EditField.

## Checkpoint: 1-modes-and-actions
Goal: Replace `Mode::Edit` with `Mode::ViewTask` and `Mode::EditField`, and update the `Action` enum accordingly.

### Requirements

**In `src/tui/app.rs`:**

1. Replace `Mode::Edit` in the `Mode` enum with two new variants: `ViewTask` and `EditField`.
   The enum should now be: `Normal`, `Insert`, `ViewTask`, `EditField`, `Column`, `Confirm`,
   `Help`, `Move`.

2. Update the `impl Display for Mode` to format `ViewTask` as `"VIEW"` and `EditField` as
   `"EDIT-FIELD"`.

3. Remove `Action::EditTask` from the `Action` enum.

4. Add three new `Action` variants:
   - `ViewTask` — enter view mode for the focused task
   - `EditField` — enter field-edit mode for the currently selected field
   - `CycleFieldPrev` — cycle to the previous field (k in ViewTask)

5. No changes to the `App` struct fields — reuse `editing_task`, `edit_field`, and
   `input_buffer` as-is.

### Validation
```validation
cargo build 2>&1
cargo test 2>&1
cargo clippy -- -D warnings 2>&1
```

### Notes
This checkpoint intentionally breaks compilation in `input.rs` and `mod.rs` which still
reference `Mode::Edit` and `Action::EditTask`. Those are fixed in subsequent checkpoints.
The `#[derive(...)]` on `Action` may need updating if `EditTask` was the only non-Copy
variant — verify derives still compile.

## Checkpoint: 2-input-handlers
Goal: Wire input handlers for `Mode::ViewTask` and `Mode::EditField`; remove `Mode::Edit` handler.

### Requirements

**In `src/tui/input.rs`:**

1. In `handle_normal`: remove `(_, event::KeyCode::Char('e')) => Action::EditTask`.
   Add `(_, event::KeyCode::Enter) => Action::ViewTask`.

2. In `handle_input` (the top-level match on `mode`): replace
   `Mode::Edit => handle_edit(key)` with:
   - `Mode::ViewTask => handle_view_task(key)`
   - `Mode::EditField => handle_edit_field(key)`

3. Delete the `handle_edit` function entirely.

4. Add `fn handle_view_task(key: KeyEvent) -> Action`:
   - `Tab` → `Action::CycleField`
   - `Char('j')` → `Action::CycleField`
   - `Char('k')` → `Action::CycleFieldPrev`
   - `Char('i')` → `Action::EditField`
   - `Enter` → `Action::Save`
   - `Esc` → `Action::Cancel`

5. Add `fn handle_edit_field(key: KeyEvent) -> Action`:
   - `Char(c)` → `Action::InsertText(c.to_string())` (text input for title/description)
   - `Backspace` → `Action::DeleteChar`
   - `Enter` → `Action::Save`
   - `Esc` → `Action::Cancel`
   - `Tab` → `Action::CyclePriority` (cycles priority when on priority field)
   - `Char('p')` or `Char('P')` → `Action::CyclePriority`

6. Add unit tests for the new handlers:
   - `view_task_cycle_field` (Tab → CycleField)
   - `view_task_cycle_field_prev` (k → CycleFieldPrev)
   - `view_task_edit_field` (i → EditField)
   - `view_task_save` (Enter → Save)
   - `view_task_cancel` (Esc → Cancel)
   - `edit_field_insert_text` (a → InsertText("a"))
   - `edit_field_delete_char` (Backspace → DeleteChar)
   - `edit_field_save` (Enter → Save)
   - `edit_field_cancel` (Esc → Cancel)
   - `edit_field_cycle_priority_tab` (Tab → CyclePriority)
   - `edit_field_cycle_priority_p` (p → CyclePriority)
   - `normal_enter_view_task` (Enter → ViewTask)

7. Remove the old `edit_*` unit tests that tested `Mode::Edit` handlers
   (`edit_save`, `edit_cancel`, `edit_cycle_field`).

### Validation
```validation
cargo build 2>&1
cargo test tui::input -- --nocapture 2>&1
cargo clippy -- -D warnings 2>&1
```

## Checkpoint: 3-action-execution
Goal: Implement the action handlers in `execute_action` for the new modes.

### Requirements

**In `src/tui/mod.rs`:**

1. Replace `Action::EditTask` handler with `Action::ViewTask`:
   - Clone the focused task into `app.editing_task`
   - Set `app.edit_field = 0`
   - Set `app.input_buffer = task.title`
   - Set `app.mode = Mode::ViewTask`
   - If no task is focused, call `app.set_error("No task selected to view".into())`

2. Add `Action::EditField` handler:
   - If `app.edit_field < 2` (title or description): set `app.input_buffer` to the
     corresponding field value from `app.editing_task`, then set `app.mode = Mode::EditField`
   - If `app.edit_field == 2` (priority): just set `app.mode = Mode::EditField`
     (no buffer needed; cycling uses CyclePriority)

3. Add `Action::CycleFieldPrev` handler:
   - Works in both `Mode::ViewTask` and `Mode::EditField`
   - If in `Mode::EditField`: first save current field from `input_buffer` into
     `editing_task` (title for field 0, description for field 1)
   - Decrement `app.edit_field` (saturating_sub to 0)
   - Update `input_buffer` for the new field (from `editing_task` if text field,
     clear if priority field)

4. Modify `Action::CycleField` handler: update the mode check from `Mode::Edit` to
   handle both `Mode::ViewTask` and `Mode::EditField`. The save-before-cycle logic
   should apply when in `Mode::EditField`.

5. Modify `Action::Save` handler:
   - Add a branch for `app.mode == Mode::EditField`:
     - Save current field from `input_buffer` into `editing_task` (title for 0,
       description for 1; priority is already in `editing_task` via CyclePriority)
     - Return to `Mode::ViewTask`
     - Return `(false, false)` (no DB reload needed — just local state change)
   - Add a branch for `app.mode == Mode::ViewTask`:
     - Validate `editing_task.title` is non-empty (error if empty)
     - If `editing_task.id.is_empty()`: this is a new task creation — call
       `shell::add_task` with the task data (same logic as the old `Mode::Edit`
       new-task branch). Set `mode = Mode::Normal`, clear state, return `(true, false)`
     - If `editing_task.id` is non-empty: diff `editing_task` against the original
       in `app.state`, call `shell::edit_task` with `TaskChanges`. Set `mode = Mode::Normal`,
       clear state, return `(true, false)`

6. Modify `Action::Cancel` handler:
   - If `app.mode == Mode::EditField`:
     - Restore `input_buffer` from the current field of `editing_task`
       (title for field 0, description for field 1, cleared for field 2)
     - Set `app.mode = Mode::ViewTask`
   - Otherwise (existing behavior for other modes):
     - Set `app.mode = Mode::Normal`
     - Clear `editing_task` and `input_buffer`

7. Update `Action::CyclePriority`: the existing handler works for any mode with
   `editing_task` set — no changes needed. Verify it compiles.

8. Remove all remaining references to `Mode::Edit` in `execute_action`.

### Validation
```validation
cargo build 2>&1
cargo test tui -- --nocapture 2>&1
cargo clippy -- -D warnings 2>&1
```

## Checkpoint: 4-rendering-and-help
Goal: Update rendering for the new modes — status bar hints, edit overlay cursor behavior, help overlay.

### Requirements

**In `src/tui/widgets/board.rs`:**

1. In `render_board`'s content match: replace `Mode::Edit => render_edit_overlay(...)` with:
   ```
   Mode::ViewTask | Mode::EditField => render_edit_overlay(frame, content, app),
   ```

2. In `render_status_bar`, replace the `Mode::Edit` match arm with two new arms:
   - `Mode::ViewTask` → `"VIEW — Tab/j/k cycle | i edit | Enter save | Esc cancel"`
   - `Mode::EditField` → use a match on `app.edit_field` to show:
     - Field 0 (title) or 1 (description): `"EDIT FIELD — Enter save | Esc cancel"`
     - Field 2 (priority): `"EDIT PRIORITY — Tab/p cycle | Enter save | Esc cancel"`

**In `src/tui/widgets/dialog.rs`:**

3. In `render_edit_overlay`, modify the cursor rendering: the cursor `█` should appear
   only when `app.mode == Mode::EditField`. When `app.mode == Mode::ViewTask`, all fields
   show their values from `editing_task` without a cursor (read-only display).

   Specifically, the three conditional expressions `if app.edit_field == N` that control
   cursor display should become `if app.mode == Mode::EditField && app.edit_field == N`.

4. The priority field rendering (reversed style when focused) should work the same in
   both `ViewTask` and `EditField` modes — no change needed there.

**In `src/tui/widgets/help.rs`:**

5. In `render_help_overlay`:
   - Replace `"e  edit task"` with `"Enter  view task"` in the Normal Mode section
   - Update the Insert/Edit section to reflect the new modes. Replace the existing
     Insert/Edit lines with:
     ```
     Line::raw("  View Task                                   "),
     Line::raw("  Tab/j/k  cycle field  i  edit  Enter save  Esc cancel"),
     Line::raw("  Edit Field                                  "),
     Line::raw("  Enter  save  Esc  cancel  Tab/p cycle (priority)    "),
     ```

### Validation
```validation
cargo build 2>&1
cargo test 2>&1
cargo clippy -- -D warnings 2>&1
just validate
```

### Notes
The `render_edit_overlay` function reads `app.mode` to decide cursor visibility. The
function signature does not change — it already receives `&App` which has the mode.

The status bar for `Mode::EditField` requires reading `app.edit_field` inside the match
arm. Since `app` is passed as `&mut App` to `render_status_bar`, this is accessible.
