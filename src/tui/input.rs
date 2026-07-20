use crossterm::event::{self, Event, KeyEvent, KeyEventKind};

use super::app::{Action, ConfirmContext, Mode};

/// Map a key event and current mode to an Action.
///
/// This is a pure function — no I/O — so it can be unit-tested directly.
pub fn handle_input(
    key: KeyEvent,
    mode: Mode,
    confirm_context: Option<ConfirmContext>,
) -> Action {
    // Only handle key press (not release/repeat)
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }

    match mode {
        Mode::Normal => handle_normal(key),
        Mode::Insert => handle_insert(key),
        Mode::Edit => handle_edit(key),
        Mode::Column => handle_column_mode(key),
        Mode::Confirm => handle_confirm(
            key,
            confirm_context.unwrap_or(ConfirmContext::TaskDelete),
        ),
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
        (_, event::KeyCode::Char('e')) => Action::EditTask,
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

fn handle_edit(key: KeyEvent) -> Action {
    match key.code {
        event::KeyCode::Enter => Action::Save,
        event::KeyCode::Esc => Action::Cancel,
        event::KeyCode::Tab => Action::CycleField,
        event::KeyCode::Char('p') | event::KeyCode::Char('P') => Action::CyclePriority,
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
            event::KeyCode::Char('n')
            | event::KeyCode::Char('N')
            | event::KeyCode::Esc => Action::ConfirmNo,
            _ => Action::None,
        },
        ConfirmContext::ColumnDelete => match key.code {
            event::KeyCode::Char('m') | event::KeyCode::Char('M') => Action::ConfirmMoveToFirst,
            event::KeyCode::Char('d') | event::KeyCode::Char('D') => Action::ConfirmDeleteAll,
            event::KeyCode::Char('n')
            | event::KeyCode::Char('N')
            | event::KeyCode::Esc => Action::ConfirmNo,
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
        assert_eq!(handle_input(key('q'), Mode::Normal, None), Action::Quit);
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::Normal, None),
            Action::Quit
        );
    }

    #[test]
    fn normal_prev_column() {
        assert_eq!(
            handle_input(key('h'), Mode::Normal, None),
            Action::NavigatePrevColumn
        );
    }

    #[test]
    fn normal_next_column() {
        assert_eq!(
            handle_input(key('l'), Mode::Normal, None),
            Action::NavigateNextColumn
        );
    }

    #[test]
    fn normal_next_task() {
        assert_eq!(
            handle_input(key('j'), Mode::Normal, None),
            Action::NavigateNextTask
        );
    }

    #[test]
    fn normal_prev_task() {
        assert_eq!(
            handle_input(key('k'), Mode::Normal, None),
            Action::NavigatePrevTask
        );
    }

    #[test]
    fn normal_add_task() {
        assert_eq!(handle_input(key('a'), Mode::Normal, None), Action::AddTask);
    }

    #[test]
    fn normal_edit_task() {
        assert_eq!(handle_input(key('e'), Mode::Normal, None), Action::EditTask);
    }

    #[test]
    fn normal_enter_move_mode() {
        assert_eq!(handle_input(key('m'), Mode::Normal, None), Action::EnterMoveMode);
    }

    #[test]
    fn normal_delete_task() {
        assert_eq!(handle_input(key('d'), Mode::Normal, None), Action::DeleteTask);
    }

    #[test]
    fn normal_move_task_prev_column() {
        assert_eq!(
            handle_input(key('H'), Mode::Normal, None),
            Action::MoveTaskPrevColumn
        );
    }

    #[test]
    fn normal_move_task_next_column() {
        assert_eq!(
            handle_input(key('L'), Mode::Normal, None),
            Action::MoveTaskNextColumn
        );
    }

    #[test]
    fn normal_move_task_down() {
        assert_eq!(handle_input(key('J'), Mode::Normal, None), Action::MoveTaskDown);
    }

    #[test]
    fn normal_move_task_up() {
        assert_eq!(handle_input(key('K'), Mode::Normal, None), Action::MoveTaskUp);
    }

    #[test]
    fn normal_enter_column_mode() {
        assert_eq!(
            handle_input(key('C'), Mode::Normal, None),
            Action::EnterColumnMode
        );
    }

    #[test]
    fn normal_toggle_help() {
        assert_eq!(handle_input(key('?'), Mode::Normal, None), Action::ToggleHelp);
    }

    #[test]
    fn normal_unknown_key() {
        assert_eq!(handle_input(key('x'), Mode::Normal, None), Action::None);
    }

    // ── Insert mode ──────────────────────────────────────────────────────

    #[test]
    fn insert_save() {
        assert_eq!(
            handle_input(key_code(KeyCode::Enter), Mode::Insert, None),
            Action::Save
        );
    }

    #[test]
    fn insert_cancel() {
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::Insert, None),
            Action::Cancel
        );
    }

    #[test]
    fn insert_text() {
        assert_eq!(
            handle_input(key('a'), Mode::Insert, None),
            Action::InsertText("a".to_string())
        );
    }

    #[test]
    fn insert_backspace() {
        assert_eq!(
            handle_input(key_code(KeyCode::Backspace), Mode::Insert, None),
            Action::DeleteChar
        );
    }

    // ── Edit mode ────────────────────────────────────────────────────────

    #[test]
    fn edit_save() {
        assert_eq!(
            handle_input(key_code(KeyCode::Enter), Mode::Edit, None),
            Action::Save
        );
    }

    #[test]
    fn edit_cancel() {
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::Edit, None),
            Action::Cancel
        );
    }

    #[test]
    fn edit_cycle_field() {
        assert_eq!(
            handle_input(key_code(KeyCode::Tab), Mode::Edit, None),
            Action::CycleField
        );
    }

    // ── Column mode ──────────────────────────────────────────────────────

    #[test]
    fn column_add() {
        assert_eq!(handle_input(key('a'), Mode::Column, None), Action::ColumnAdd);
    }

    #[test]
    fn column_rename() {
        assert_eq!(handle_input(key('r'), Mode::Column, None), Action::ColumnRename);
    }

    #[test]
    fn column_delete() {
        assert_eq!(handle_input(key('d'), Mode::Column, None), Action::ColumnDelete);
    }

    #[test]
    fn column_move_left() {
        assert_eq!(handle_input(key('h'), Mode::Column, None), Action::ColumnMoveLeft);
    }

    #[test]
    fn column_move_right() {
        assert_eq!(
            handle_input(key('l'), Mode::Column, None),
            Action::ColumnMoveRight
        );
    }

    #[test]
    fn column_exit() {
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::Column, None),
            Action::ColumnExit
        );
    }

    // ── Confirm mode (task delete) ─────────────────────────────────────

    #[test]
    fn confirm_task_yes() {
        assert_eq!(
            handle_input(
                key('y'),
                Mode::Confirm,
                Some(ConfirmContext::TaskDelete)
            ),
            Action::ConfirmYes
        );
        assert_eq!(
            handle_input(
                key('Y'),
                Mode::Confirm,
                Some(ConfirmContext::TaskDelete)
            ),
            Action::ConfirmYes
        );
    }

    #[test]
    fn confirm_task_no() {
        assert_eq!(
            handle_input(
                key('n'),
                Mode::Confirm,
                Some(ConfirmContext::TaskDelete)
            ),
            Action::ConfirmNo
        );
        assert_eq!(
            handle_input(
                key('N'),
                Mode::Confirm,
                Some(ConfirmContext::TaskDelete)
            ),
            Action::ConfirmNo
        );
        assert_eq!(
            handle_input(
                key_code(KeyCode::Esc),
                Mode::Confirm,
                Some(ConfirmContext::TaskDelete)
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
                Some(ConfirmContext::ColumnDelete)
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
                Some(ConfirmContext::ColumnDelete)
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
                Some(ConfirmContext::ColumnDelete)
            ),
            Action::ConfirmNo
        );
        assert_eq!(
            handle_input(
                key_code(KeyCode::Esc),
                Mode::Confirm,
                Some(ConfirmContext::ColumnDelete)
            ),
            Action::ConfirmNo
        );
    }

    // ── Help mode ────────────────────────────────────────────────────────

    #[test]
    fn help_close() {
        assert_eq!(
            handle_input(key('?'), Mode::Help, None),
            Action::ModeChange(Mode::Normal)
        );
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::Help, None),
            Action::ModeChange(Mode::Normal)
        );
        assert_eq!(
            handle_input(key('q'), Mode::Help, None),
            Action::ModeChange(Mode::Normal)
        );
    }

    // ── Move mode ────────────────────────────────────────────────────────

    #[test]
    fn move_target_prev() {
        assert_eq!(
            handle_input(key('h'), Mode::Move, None),
            Action::MoveTargetPrev
        );
    }

    #[test]
    fn move_target_next() {
        assert_eq!(
            handle_input(key('l'), Mode::Move, None),
            Action::MoveTargetNext
        );
    }

    #[test]
    fn move_confirm() {
        assert_eq!(
            handle_input(key_code(KeyCode::Enter), Mode::Move, None),
            Action::MoveConfirm
        );
    }

    #[test]
    fn move_cancel() {
        assert_eq!(
            handle_input(key_code(KeyCode::Esc), Mode::Move, None),
            Action::MoveCancel
        );
    }

    // ── Key release should be ignored ────────────────────────────────────
    #[test]
    fn key_release_ignored() {
        let mut release_key = key('q');
        release_key.kind = KeyEventKind::Release;
        assert_eq!(
            handle_input(release_key, Mode::Normal, None),
            Action::None
        );
    }
}
