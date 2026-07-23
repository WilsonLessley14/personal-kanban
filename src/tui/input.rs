use crossterm::event::{self, Event, KeyEvent, KeyEventKind};

use super::app::{Action, ConfirmContext, Mode};

/// Map a key event and current mode to an Action.
///
/// This is a pure function — no I/O — so it can be unit-tested directly.
pub fn handle_input(
    key: KeyEvent,
    mode: Mode,
    confirm_context: Option<ConfirmContext>,
    edit_field: usize,
) -> Action {
    // Only handle key press (not release/repeat)
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }

    match mode {
        Mode::Normal => handle_normal(key),
        Mode::Insert => handle_insert(key),
        Mode::ViewTask => handle_view_task(key),
        Mode::EditField => handle_edit_field(key, edit_field),
        Mode::Column => handle_column_mode(key),
        Mode::Confirm => handle_confirm(key, confirm_context.unwrap_or(ConfirmContext::TaskDelete)),
        Mode::Help => handle_help(key),
        Mode::Move => handle_move_mode(key),
    }
}

fn handle_normal(key: KeyEvent) -> Action {
    match (key.modifiers, key.code) {
        // Navigation
        (_, event::KeyCode::Char('q')) => Action::Quit,
        (_, event::KeyCode::Esc) => Action::Quit,
        (_, event::KeyCode::Char('h')) => Action::NavigatePrevColumn,
        (_, event::KeyCode::Char('l')) => Action::NavigateNextColumn,
        (_, event::KeyCode::Char('j')) => Action::NavigateNextTask,
        (_, event::KeyCode::Char('k')) => Action::NavigatePrevTask,
        // Operations
        (_, event::KeyCode::Char('a')) => Action::AddTask,
        (_, event::KeyCode::Enter) => Action::ViewTask,
        (_, event::KeyCode::Char('m')) => Action::EnterMoveMode,
        (_, event::KeyCode::Char('d')) => Action::DeleteTask,
        (_, event::KeyCode::Char('H')) => Action::MoveTaskPrevColumn,
        (_, event::KeyCode::Char('L')) => Action::MoveTaskNextColumn,
        (_, event::KeyCode::Char('J')) => Action::MoveTaskDown,
        (_, event::KeyCode::Char('K')) => Action::MoveTaskUp,
        (_, event::KeyCode::Char('C')) => Action::EnterColumnMode,
        (_, event::KeyCode::Char('?')) => Action::ToggleHelp,
        _ => Action::None,
    }
}

fn handle_insert(key: KeyEvent) -> Action {
    match key.code {
        event::KeyCode::Enter => Action::Save,
        event::KeyCode::Esc => Action::Cancel,
        event::KeyCode::Char(c) => Action::InsertText(c.to_string()),
        event::KeyCode::Backspace => Action::DeleteChar,
        _ => Action::None,
    }
}

fn handle_view_task(key: KeyEvent) -> Action {
    match key.code {
        event::KeyCode::Tab | event::KeyCode::Char('j') => Action::CycleField,
        event::KeyCode::Char('k') => Action::CycleFieldPrev,
        event::KeyCode::Char('i') => Action::EditField,
        event::KeyCode::Enter => Action::Save,
        event::KeyCode::Esc => Action::Cancel,
        _ => Action::None,
    }
}

fn handle_edit_field(key: KeyEvent, edit_field: usize) -> Action {
    let is_priority_field = edit_field == 2;
    match key.code {
        event::KeyCode::Enter => Action::Save,
        event::KeyCode::Esc => Action::Cancel,
        event::KeyCode::Tab if is_priority_field => Action::CyclePriority,
        event::KeyCode::Char('p') | event::KeyCode::Char('P') if is_priority_field => {
            Action::CyclePriority
        }
        event::KeyCode::Char(c) => Action::InsertText(c.to_string()),
        event::KeyCode::Backspace => Action::DeleteChar,
        _ => Action::None,
    }
}

fn handle_column_mode(key: KeyEvent) -> Action {
    match key.code {
        event::KeyCode::Char('a') => Action::ColumnAdd,
        event::KeyCode::Char('r') => Action::ColumnRename,
        event::KeyCode::Char('d') => Action::ColumnDelete,
        event::KeyCode::Char('h') => Action::ColumnMoveLeft,
        event::KeyCode::Char('l') => Action::ColumnMoveRight,
        event::KeyCode::Esc => Action::ColumnExit,
        _ => Action::None,
    }
}

fn handle_confirm(key: KeyEvent, context: ConfirmContext) -> Action {
    match context {
        ConfirmContext::TaskDelete => match key.code {
            event::KeyCode::Char('y') | event::KeyCode::Char('Y') => Action::ConfirmYes,
            event::KeyCode::Char('n') | event::KeyCode::Char('N') | event::KeyCode::Esc => {
                Action::ConfirmNo
            }
            _ => Action::None,
        },
        ConfirmContext::ColumnDelete => match key.code {
            event::KeyCode::Char('m') | event::KeyCode::Char('M') => Action::ConfirmMoveToFirst,
            event::KeyCode::Char('d') | event::KeyCode::Char('D') => Action::ConfirmDeleteAll,
            event::KeyCode::Char('n') | event::KeyCode::Char('N') | event::KeyCode::Esc => {
                Action::ConfirmNo
            }
            _ => Action::None,
        },
    }
}

fn handle_help(key: KeyEvent) -> Action {
    match key.code {
        event::KeyCode::Char('?') | event::KeyCode::Esc | event::KeyCode::Char('q') => {
            Action::ModeChange(Mode::Normal)
        }
        _ => Action::None,
    }
}

fn handle_move_mode(key: KeyEvent) -> Action {
    match key.code {
        event::KeyCode::Char('h') => Action::MoveTargetPrev,
        event::KeyCode::Char('l') => Action::MoveTargetNext,
        event::KeyCode::Enter => Action::MoveConfirm,
        event::KeyCode::Esc => Action::MoveCancel,
        _ => Action::None,
    }
}

/// Read the next key event (blocking).
pub fn read_key() -> Option<KeyEvent> {
    if event::poll(std::time::Duration::from_millis(50)).ok()? {
        let evt = event::read().ok()?;
        if let Event::Key(key) = evt {
            Some(key)
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn key(c: char) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: event::KeyEventState::NONE,
        }
    }

    fn key_code(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: event::KeyEventState::NONE,
        }
    }

    // ── Normal mode ──────────────────────────────────────────────────────

    #[test]
    fn normal_quit() {
        assert_eq!(handle_input(key('q'), Mode::Normal, None, 0), Action::Quit);
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::Normal, None, 0),
            Action::Quit
        );
    }

    #[test]
    fn normal_prev_column() {
        assert_eq!(
            handle_input(key('h'), Mode::Normal, None, 0),
            Action::NavigatePrevColumn
        );
    }

    #[test]
    fn normal_next_column() {
        assert_eq!(
            handle_input(key('l'), Mode::Normal, None, 0),
            Action::NavigateNextColumn
        );
    }

    #[test]
    fn normal_next_task() {
        assert_eq!(
            handle_input(key('j'), Mode::Normal, None, 0),
            Action::NavigateNextTask
        );
    }

    #[test]
    fn normal_prev_task() {
        assert_eq!(
            handle_input(key('k'), Mode::Normal, None, 0),
            Action::NavigatePrevTask
        );
    }

    #[test]
    fn normal_add_task() {
        assert_eq!(
            handle_input(key('a'), Mode::Normal, None, 0),
            Action::AddTask
        );
    }

    #[test]
    fn normal_enter_view_task() {
        assert_eq!(
            handle_input(key_code(KeyCode::Enter), Mode::Normal, None, 0),
            Action::ViewTask
        );
    }

    #[test]
    fn normal_enter_move_mode() {
        assert_eq!(
            handle_input(key('m'), Mode::Normal, None, 0),
            Action::EnterMoveMode
        );
    }

    #[test]
    fn normal_delete_task() {
        assert_eq!(
            handle_input(key('d'), Mode::Normal, None, 0),
            Action::DeleteTask
        );
    }

    #[test]
    fn normal_move_task_prev_column() {
        assert_eq!(
            handle_input(key('H'), Mode::Normal, None, 0),
            Action::MoveTaskPrevColumn
        );
    }

    #[test]
    fn normal_move_task_next_column() {
        assert_eq!(
            handle_input(key('L'), Mode::Normal, None, 0),
            Action::MoveTaskNextColumn
        );
    }

    #[test]
    fn normal_move_task_down() {
        assert_eq!(
            handle_input(key('J'), Mode::Normal, None, 0),
            Action::MoveTaskDown
        );
    }

    #[test]
    fn normal_move_task_up() {
        assert_eq!(
            handle_input(key('K'), Mode::Normal, None, 0),
            Action::MoveTaskUp
        );
    }

    #[test]
    fn normal_enter_column_mode() {
        assert_eq!(
            handle_input(key('C'), Mode::Normal, None, 0),
            Action::EnterColumnMode
        );
    }

    #[test]
    fn normal_toggle_help() {
        assert_eq!(
            handle_input(key('?'), Mode::Normal, None, 0),
            Action::ToggleHelp
        );
    }

    #[test]
    fn normal_unknown_key() {
        assert_eq!(handle_input(key('x'), Mode::Normal, None, 0), Action::None);
    }

    // ── Insert mode ──────────────────────────────────────────────────────

    #[test]
    fn insert_save() {
        assert_eq!(
            handle_input(key_code(KeyCode::Enter), Mode::Insert, None, 0),
            Action::Save
        );
    }

    #[test]
    fn insert_cancel() {
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::Insert, None, 0),
            Action::Cancel
        );
    }

    #[test]
    fn insert_text() {
        assert_eq!(
            handle_input(key('a'), Mode::Insert, None, 0),
            Action::InsertText("a".to_string())
        );
    }

    #[test]
    fn insert_backspace() {
        assert_eq!(
            handle_input(key_code(KeyCode::Backspace), Mode::Insert, None, 0),
            Action::DeleteChar
        );
    }

    // ── ViewTask mode ────────────────────────────────────────────────────

    #[test]
    fn view_task_cycle_field() {
        assert_eq!(
            handle_input(key_code(KeyCode::Tab), Mode::ViewTask, None, 0),
            Action::CycleField
        );
        assert_eq!(
            handle_input(key('j'), Mode::ViewTask, None, 0),
            Action::CycleField
        );
    }

    #[test]
    fn view_task_cycle_field_prev() {
        assert_eq!(
            handle_input(key('k'), Mode::ViewTask, None, 0),
            Action::CycleFieldPrev
        );
    }

    #[test]
    fn view_task_edit_field() {
        assert_eq!(
            handle_input(key('i'), Mode::ViewTask, None, 0),
            Action::EditField
        );
    }

    #[test]
    fn view_task_save() {
        assert_eq!(
            handle_input(key_code(KeyCode::Enter), Mode::ViewTask, None, 0),
            Action::Save
        );
    }

    #[test]
    fn view_task_cancel() {
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::ViewTask, None, 0),
            Action::Cancel
        );
    }

    // ── EditField mode ───────────────────────────────────────────────────

    #[test]
    fn edit_field_insert_text() {
        assert_eq!(
            handle_input(key('a'), Mode::EditField, None, 0),
            Action::InsertText("a".to_string())
        );
    }

    #[test]
    fn edit_field_delete_char() {
        assert_eq!(
            handle_input(key_code(KeyCode::Backspace), Mode::EditField, None, 0),
            Action::DeleteChar
        );
    }

    #[test]
    fn edit_field_save() {
        assert_eq!(
            handle_input(key_code(KeyCode::Enter), Mode::EditField, None, 0),
            Action::Save
        );
    }

    #[test]
    fn edit_field_cancel() {
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::EditField, None, 0),
            Action::Cancel
        );
    }

    #[test]
    fn edit_field_cycle_priority_tab() {
        // Tab cycles priority only on the priority field (edit_field == 2)
        assert_eq!(
            handle_input(key_code(KeyCode::Tab), Mode::EditField, None, 2),
            Action::CyclePriority
        );
        // Tab is ignored on text fields
        assert_eq!(
            handle_input(key_code(KeyCode::Tab), Mode::EditField, None, 0),
            Action::None
        );
        assert_eq!(
            handle_input(key_code(KeyCode::Tab), Mode::EditField, None, 1),
            Action::None
        );
    }

    #[test]
    fn edit_field_cycle_priority_p() {
        // p/P cycle priority only on the priority field (edit_field == 2)
        assert_eq!(
            handle_input(key('p'), Mode::EditField, None, 2),
            Action::CyclePriority
        );
        assert_eq!(
            handle_input(key('P'), Mode::EditField, None, 2),
            Action::CyclePriority
        );
        // p/P are typed as text on title field
        assert_eq!(
            handle_input(key('p'), Mode::EditField, None, 0),
            Action::InsertText("p".to_string())
        );
        assert_eq!(
            handle_input(key('P'), Mode::EditField, None, 0),
            Action::InsertText("P".to_string())
        );
        // p/P are typed as text on description field
        assert_eq!(
            handle_input(key('p'), Mode::EditField, None, 1),
            Action::InsertText("p".to_string())
        );
    }

    // ── Column mode ──────────────────────────────────────────────────────

    #[test]
    fn column_add() {
        assert_eq!(
            handle_input(key('a'), Mode::Column, None, 0),
            Action::ColumnAdd
        );
    }

    #[test]
    fn column_rename() {
        assert_eq!(
            handle_input(key('r'), Mode::Column, None, 0),
            Action::ColumnRename
        );
    }

    #[test]
    fn column_delete() {
        assert_eq!(
            handle_input(key('d'), Mode::Column, None, 0),
            Action::ColumnDelete
        );
    }

    #[test]
    fn column_move_left() {
        assert_eq!(
            handle_input(key('h'), Mode::Column, None, 0),
            Action::ColumnMoveLeft
        );
    }

    #[test]
    fn column_move_right() {
        assert_eq!(
            handle_input(key('l'), Mode::Column, None, 0),
            Action::ColumnMoveRight
        );
    }

    #[test]
    fn column_exit() {
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::Column, None, 0),
            Action::ColumnExit
        );
    }

    // ── Confirm mode (task delete) ─────────────────────────────────────

    #[test]
    fn confirm_task_yes() {
        assert_eq!(
            handle_input(key('y'), Mode::Confirm, Some(ConfirmContext::TaskDelete), 0),
            Action::ConfirmYes
        );
        assert_eq!(
            handle_input(key('Y'), Mode::Confirm, Some(ConfirmContext::TaskDelete), 0),
            Action::ConfirmYes
        );
    }

    #[test]
    fn confirm_task_no() {
        assert_eq!(
            handle_input(key('n'), Mode::Confirm, Some(ConfirmContext::TaskDelete), 0),
            Action::ConfirmNo
        );
        assert_eq!(
            handle_input(key('N'), Mode::Confirm, Some(ConfirmContext::TaskDelete), 0),
            Action::ConfirmNo
        );
        assert_eq!(
            handle_input(
                key_code(KeyCode::Esc),
                Mode::Confirm,
                Some(ConfirmContext::TaskDelete),
                0
            ),
            Action::ConfirmNo
        );
    }

    // ── Confirm mode (column delete - three-way) ───────────────────────

    #[test]
    fn confirm_column_move_to_first() {
        assert_eq!(
            handle_input(
                key('m'),
                Mode::Confirm,
                Some(ConfirmContext::ColumnDelete),
                0
            ),
            Action::ConfirmMoveToFirst
        );
    }

    #[test]
    fn confirm_column_delete_all() {
        assert_eq!(
            handle_input(
                key('d'),
                Mode::Confirm,
                Some(ConfirmContext::ColumnDelete),
                0
            ),
            Action::ConfirmDeleteAll
        );
    }

    #[test]
    fn confirm_column_cancel() {
        assert_eq!(
            handle_input(
                key('n'),
                Mode::Confirm,
                Some(ConfirmContext::ColumnDelete),
                0
            ),
            Action::ConfirmNo
        );
        assert_eq!(
            handle_input(
                key_code(KeyCode::Esc),
                Mode::Confirm,
                Some(ConfirmContext::ColumnDelete),
                0
            ),
            Action::ConfirmNo
        );
    }

    // ── Help mode ────────────────────────────────────────────────────────

    #[test]
    fn help_close() {
        assert_eq!(
            handle_input(key('?'), Mode::Help, None, 0),
            Action::ModeChange(Mode::Normal)
        );
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::Help, None, 0),
            Action::ModeChange(Mode::Normal)
        );
        assert_eq!(
            handle_input(key('q'), Mode::Help, None, 0),
            Action::ModeChange(Mode::Normal)
        );
    }

    // ── Move mode ────────────────────────────────────────────────────────

    #[test]
    fn move_target_prev() {
        assert_eq!(
            handle_input(key('h'), Mode::Move, None, 0),
            Action::MoveTargetPrev
        );
    }

    #[test]
    fn move_target_next() {
        assert_eq!(
            handle_input(key('l'), Mode::Move, None, 0),
            Action::MoveTargetNext
        );
    }

    #[test]
    fn move_confirm() {
        assert_eq!(
            handle_input(key_code(KeyCode::Enter), Mode::Move, None, 0),
            Action::MoveConfirm
        );
    }

    #[test]
    fn move_cancel() {
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::Move, None, 0),
            Action::MoveCancel
        );
    }

    // ── Key release should be ignored ────────────────────────────────────
    #[test]
    fn key_release_ignored() {
        let mut release_key = key('q');
        release_key.kind = KeyEventKind::Release;
        assert_eq!(
            handle_input(release_key, Mode::Normal, None, 0),
            Action::None
        );
    }
}
