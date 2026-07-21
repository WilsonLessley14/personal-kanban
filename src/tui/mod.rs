//! TUI module — ratatui/crossterm terminal UI for the kanban board.
//!
//! Calls only shell operations. Driven by a synchronous tick-based event loop.
//! After any mutation, board state is reloaded fresh from SQLite.

pub mod app;
pub mod input;
pub mod render;
pub mod widgets;

use std::io;

use anyhow::{Context, Result};
use crossterm::event::{Event, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::core::{OrphanAction, TaskChanges};
use crate::shell;
use crate::tui::app::{Action, App, ConfirmContext, Mode};
use crate::tui::input::handle_input;
use crate::tui::widgets::render_board;

// ── Public entry point ─────────────────────────────────────────────────────

/// Launch the TUI with the given database path.
pub fn run(db_path: &std::path::Path) -> Result<()> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).context("failed to create terminal")?;

    // Setup terminal
    setup_terminal()?;

    let result = run_tui_loop(&mut terminal, db_path);

    // Restore terminal
    restore_terminal()?;

    result
}

/// Setup crossterm terminal for alternate screen + raw mode.
fn setup_terminal() -> Result<()> {
    crossterm::terminal::enable_raw_mode().context("failed to enable raw mode")?;
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide
    )
    .context("failed to setup terminal")?;
    Ok(())
}

/// Restore crossterm terminal.
fn restore_terminal() -> Result<()> {
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )
    .ok();
    crossterm::terminal::disable_raw_mode().ok();
    Ok(())
}

// ── TUI event loop ─────────────────────────────────────────────────────────

fn run_tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    db_path: &std::path::Path,
) -> Result<()> {
    // Open the database
    let db = shell::Db::open(db_path).context("failed to open database")?;

    // Ensure a board exists
    ensure_board(&db)?;

    // Load initial state
    let state = load_state(&db)?;
    let mut app = App::with_state(state);

    loop {
        // Tick
        app.tick();

        // Draw
        terminal.draw(|frame| {
            render_board(frame, &mut app);
        })?;

        // Read input (blocking with timeout)
        let action = if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let Ok(Event::Key(key)) = crossterm::event::read() {
                if key.kind == KeyEventKind::Press {
                    handle_input(key, app.mode, app.confirm_context)
                } else {
                    Action::None
                }
            } else if let Ok(Event::Resize(_, _)) = crossterm::event::read() {
                Action::None
            } else {
                Action::None
            }
        } else {
            Action::None
        };

        // Clear error on any keypress (spec: "~3 seconds or until the next keypress")
        if action != Action::None {
            app.clear_error();
        }

        // Execute action
        let (should_reload, should_quit) = execute_action(&mut app, &db, action)?;

        if should_quit {
            break;
        }

        // Reload state from SQLite after any mutation
        if should_reload {
            if let Ok(new_state) = load_state(&db) {
                app.state = Some(new_state);
            }
            // Clear error after successful reload
            app.clear_error();
        }

        // Clamp focus to valid ranges
        app.clamp_focus();
    }

    Ok(())
}

/// Ensure at least one board exists in the database.
fn ensure_board(db: &shell::Db) -> Result<()> {
    let boards = shell::list_boards(db).context("failed to list boards")?;
    if boards.is_empty() {
        shell::init_board(db, "Personal").context("failed to initialize default board")?;
    }
    Ok(())
}

/// Load the current board state from SQLite.
fn load_state(db: &shell::Db) -> Result<crate::core::BoardState> {
    let boards = shell::list_boards(db)?;
    let board = boards
        .first()
        .ok_or_else(|| anyhow::anyhow!("no boards found"))?;
    db.load_board_state(&board.id)
        .context("failed to load board state")
}

/// Execute an action, returning (should_reload_state, should_quit).
fn execute_action(app: &mut App, db: &shell::Db, action: Action) -> Result<(bool, bool)> {
    match action {
        Action::None => Ok((false, false)),
        Action::Quit => Ok((false, true)),
        Action::ModeChange(mode) => {
            app.mode = mode;
            Ok((false, false))
        }
        Action::NavigatePrevColumn => {
            if let Some(ref state) = app.state {
                if !state.columns.is_empty() {
                    app.focused_col_idx = app
                        .focused_col_idx
                        .saturating_sub(1)
                        .min(state.columns.len() - 1);
                    app.focused_task_idx = 0;
                }
            }
            Ok((false, false))
        }
        Action::NavigateNextColumn => {
            if let Some(ref state) = app.state {
                if !state.columns.is_empty() {
                    app.focused_col_idx = (app.focused_col_idx + 1).min(state.columns.len() - 1);
                    app.focused_task_idx = 0;
                }
            }
            Ok((false, false))
        }
        Action::NavigatePrevTask => {
            let tasks = app.focused_column_tasks();
            if !tasks.is_empty() {
                app.focused_task_idx = app.focused_task_idx.saturating_sub(1);
            }
            Ok((false, false))
        }
        Action::NavigateNextTask => {
            let tasks = app.focused_column_tasks();
            if app.focused_task_idx < tasks.len().saturating_sub(1) {
                app.focused_task_idx += 1;
            }
            Ok((false, false))
        }
        Action::AddTask => {
            app.editing_task = Some(crate::core::Task {
                id: String::new(),
                column_id: String::new(),
                title: String::new(),
                description: String::new(),
                priority_id: String::new(),
                position: 0,
                created_at: String::new(),
                updated_at: String::new(),
            });
            app.edit_field = 0;
            app.input_buffer.clear();
            app.mode = Mode::Edit;
            Ok((false, false))
        }
        Action::EditTask => {
            let task_clone = app.focused_task().cloned();
            if let Some(task) = task_clone {
                app.editing_task = Some(task.clone());
                app.edit_field = 0;
                app.input_buffer = task.title;
                app.mode = Mode::Edit;
            } else {
                app.set_error("No task selected to edit".into());
            }
            Ok((false, false))
        }
        Action::DeleteTask => {
            if app.focused_task().is_some() {
                app.mode = Mode::Confirm;
                app.confirm_context = Some(ConfirmContext::TaskDelete);
            } else {
                app.set_error("No task selected to delete".into());
            }
            Ok((false, false))
        }
        Action::EnterMoveMode => {
            if app.focused_task().is_some() {
                app.mode = Mode::Move;
                app.move_target_col_idx = app.focused_col_idx;
            } else {
                app.set_error("No task selected to move".into());
            }
            Ok((false, false))
        }
        Action::MoveTaskPrevColumn => {
            if let Some(task) = app.focused_task() {
                if let Some(ref state) = app.state {
                    if app.focused_col_idx > 0 {
                        let target_col = &state.columns[app.focused_col_idx - 1];
                        let col_name = target_col.name.clone();
                        match shell::move_task(db, &task.id, &col_name, None) {
                            Ok(()) => Ok((true, false)),
                            Err(e) => {
                                app.set_error(e.to_string());
                                Ok((false, false))
                            }
                        }
                    } else {
                        app.set_error("Already at first column".into());
                        Ok((false, false))
                    }
                } else {
                    Ok((false, false))
                }
            } else {
                app.set_error("No task selected to move".into());
                Ok((false, false))
            }
        }
        Action::MoveTaskNextColumn => {
            if let Some(task) = app.focused_task() {
                if let Some(ref state) = app.state {
                    if app.focused_col_idx < state.columns.len().saturating_sub(1) {
                        let target_col = &state.columns[app.focused_col_idx + 1];
                        let col_name = target_col.name.clone();
                        match shell::move_task(db, &task.id, &col_name, None) {
                            Ok(()) => Ok((true, false)),
                            Err(e) => {
                                app.set_error(e.to_string());
                                Ok((false, false))
                            }
                        }
                    } else {
                        app.set_error("Already at last column".into());
                        Ok((false, false))
                    }
                } else {
                    Ok((false, false))
                }
            } else {
                app.set_error("No task selected to move".into());
                Ok((false, false))
            }
        }
        Action::MoveTaskDown => {
            let task_clone = app.focused_task().cloned();
            if let Some(task) = task_clone {
                match shell::reorder_task(db, &task.id, shell::OrderDirection::Down) {
                    Ok(()) => {
                        app.focused_task_idx += 1;
                        Ok((true, false))
                    }
                    Err(e) => {
                        app.set_error(e.to_string());
                        Ok((false, false))
                    }
                }
            } else {
                app.set_error("No task selected".into());
                Ok((false, false))
            }
        }
        Action::MoveTaskUp => {
            let task_clone = app.focused_task().cloned();
            if let Some(task) = task_clone {
                match shell::reorder_task(db, &task.id, shell::OrderDirection::Up) {
                    Ok(()) => {
                        app.focused_task_idx = app.focused_task_idx.saturating_sub(1);
                        Ok((true, false))
                    }
                    Err(e) => {
                        app.set_error(e.to_string());
                        Ok((false, false))
                    }
                }
            } else {
                app.set_error("No task selected".into());
                Ok((false, false))
            }
        }
        Action::EnterColumnMode => {
            app.mode = Mode::Column;
            Ok((false, false))
        }
        Action::ToggleHelp => {
            if app.mode == Mode::Help {
                app.mode = Mode::Normal;
            } else {
                app.mode = Mode::Help;
            }
            Ok((false, false))
        }
        // Move mode
        Action::MoveConfirm => {
            if let Some(task) = app.focused_task() {
                if let Some(ref state) = app.state {
                    let target_col = state.columns.get(app.move_target_col_idx);
                    if let Some(target) = target_col {
                        match shell::move_task(db, &task.id, &target.name, None) {
                            Ok(()) => {
                                app.mode = Mode::Normal;
                                app.focused_col_idx = app.move_target_col_idx;
                                app.focused_task_idx = 0;
                                Ok((true, false))
                            }
                            Err(e) => {
                                app.set_error(e.to_string());
                                Ok((false, false))
                            }
                        }
                    } else {
                        app.set_error("Invalid target column".into());
                        Ok((false, false))
                    }
                } else {
                    Ok((false, false))
                }
            } else {
                app.set_error("No task to move".into());
                Ok((false, false))
            }
        }
        Action::MoveCancel => {
            app.mode = Mode::Normal;
            Ok((false, false))
        }
        Action::MoveTargetPrev => {
            if let Some(ref _state) = app.state {
                app.move_target_col_idx = app.move_target_col_idx.saturating_sub(1);
            }
            Ok((false, false))
        }
        Action::MoveTargetNext => {
            if let Some(ref state) = app.state {
                app.move_target_col_idx =
                    (app.move_target_col_idx + 1).min(state.columns.len().saturating_sub(1));
            }
            Ok((false, false))
        }
        // Column mode actions
        Action::ColumnAdd => {
            app.mode = Mode::Insert;
            app.input_buffer.clear();
            app.editing_task = Some(crate::core::Task {
                id: String::new(),
                column_id: String::new(),
                title: String::new(),
                description: String::new(),
                priority_id: String::new(),
                position: 0,
                created_at: String::new(),
                updated_at: String::new(),
            });
            // Mark as column add via a special state
            // We detect it: if focused task is None, we're in column-add context
            Ok((false, false))
        }
        Action::ColumnRename => {
            let col_clone = app.focused_column().cloned();
            if let Some(col) = col_clone {
                app.mode = Mode::Insert;
                app.input_buffer = col.name.clone();
                app.editing_task = Some(crate::core::Task {
                    id: col.id,
                    column_id: String::new(),
                    title: String::new(),
                    description: String::new(),
                    priority_id: String::new(),
                    position: -1, // sentinel for column rename
                    created_at: String::new(),
                    updated_at: String::new(),
                });
            }
            Ok((false, false))
        }
        Action::ColumnDelete => {
            if let Some(ref state) = app.state {
                if state.columns.len() <= 1 {
                    app.set_error("Cannot delete the last column".into());
                } else {
                    app.mode = Mode::Confirm;
                    app.confirm_context = Some(ConfirmContext::ColumnDelete);
                }
            }
            Ok((false, false))
        }
        Action::ColumnMoveLeft => {
            if let Some(ref state) = app.state {
                if app.focused_col_idx > 0 {
                    let col = &state.columns[app.focused_col_idx];
                    let new_pos = (app.focused_col_idx - 1) as i32;
                    match shell::move_column(db, &col.id, new_pos) {
                        Ok(()) => {
                            app.focused_col_idx = app.focused_col_idx.saturating_sub(1);
                            Ok((true, false))
                        }
                        Err(e) => {
                            app.set_error(e.to_string());
                            Ok((false, false))
                        }
                    }
                } else {
                    app.set_error("Already at first position".into());
                    Ok((false, false))
                }
            } else {
                Ok((false, false))
            }
        }
        Action::ColumnMoveRight => {
            if let Some(ref state) = app.state {
                if app.focused_col_idx < state.columns.len().saturating_sub(1) {
                    let col = &state.columns[app.focused_col_idx];
                    let new_pos = (app.focused_col_idx + 1) as i32;
                    match shell::move_column(db, &col.id, new_pos) {
                        Ok(()) => {
                            app.focused_col_idx = (app.focused_col_idx + 1)
                                .min(state.columns.len().saturating_sub(1));
                            Ok((true, false))
                        }
                        Err(e) => {
                            app.set_error(e.to_string());
                            Ok((false, false))
                        }
                    }
                } else {
                    app.set_error("Already at last position".into());
                    Ok((false, false))
                }
            } else {
                Ok((false, false))
            }
        }
        Action::ColumnExit => {
            app.mode = Mode::Normal;
            Ok((false, false))
        }
        // Insert/Edit
        Action::Save => {
            if app.mode == Mode::Insert {
                // Check if we're in a column context (column add/rename)
                // Detect: editing_task has position -1 => column rename
                //         focused task is None and no tasks selected => column add
                if let Some(ref edit_task) = app.editing_task {
                    if edit_task.position == -1 {
                        // Column rename
                        if let Some(col) = app.focused_column() {
                            let new_name = app.input_buffer.trim().to_string();
                            if new_name.is_empty() {
                                app.set_error("Column name cannot be empty".into());
                                return Ok((false, false));
                            }
                            match shell::rename_column(db, &col.id, &new_name) {
                                Ok(()) => {
                                    app.mode = Mode::Column;
                                    app.editing_task = None;
                                    app.input_buffer.clear();
                                    Ok((true, false))
                                }
                                Err(e) => {
                                    app.set_error(e.to_string());
                                    Ok((false, false))
                                }
                            }
                        } else {
                            app.set_error("No column to rename".into());
                            Ok((false, false))
                        }
                    } else if edit_task.id.is_empty() {
                        // Column add
                        let col_name = app.input_buffer.trim().to_string();
                        if col_name.is_empty() {
                            app.set_error("Column name cannot be empty".into());
                            return Ok((false, false));
                        }
                        if let Some(ref state) = app.state {
                            let board_id = &state.board.id;
                            match shell::add_column(db, board_id, &col_name) {
                                Ok(_new_col) => {
                                    app.mode = Mode::Column;
                                    app.focused_col_idx = state.columns.len(); // new col at end
                                    app.editing_task = None;
                                    app.input_buffer.clear();
                                    Ok((true, false))
                                }
                                Err(e) => {
                                    app.set_error(e.to_string());
                                    Ok((false, false))
                                }
                            }
                        } else {
                            app.set_error("No board state".into());
                            Ok((false, false))
                        }
                    } else {
                        // Task insert (normal)
                        let title = app.input_buffer.trim().to_string();
                        if title.is_empty() {
                            app.set_error("Title cannot be empty".into());
                            return Ok((false, false));
                        }
                        if let Some(ref state) = app.state {
                            let col = state
                                .columns
                                .get(app.focused_col_idx)
                                .ok_or_else(|| anyhow::anyhow!("no focused column"))?;
                            let default_priority = state
                                .priorities
                                .iter()
                                .find(|p| p.name == "medium")
                                .map(|p| p.name.as_str())
                                .unwrap_or("medium");

                            match shell::add_task(db, &col.name, &title, "", default_priority) {
                                Ok(_) => {
                                    app.mode = Mode::Normal;
                                    app.editing_task = None;
                                    app.input_buffer.clear();
                                    Ok((true, false))
                                }
                                Err(e) => {
                                    app.set_error(e.to_string());
                                    Ok((false, false))
                                }
                            }
                        } else {
                            app.set_error("No board state".into());
                            Ok((false, false))
                        }
                    }
                } else {
                    // Task insert (normal, no editing_task)
                    let title = app.input_buffer.trim().to_string();
                    if title.is_empty() {
                        app.set_error("Title cannot be empty".into());
                        return Ok((false, false));
                    }
                    if let Some(ref state) = app.state {
                        let col = state
                            .columns
                            .get(app.focused_col_idx)
                            .ok_or_else(|| anyhow::anyhow!("no focused column"))?;
                        let default_priority = state
                            .priorities
                            .iter()
                            .find(|p| p.name == "medium")
                            .map(|p| p.name.as_str())
                            .unwrap_or("medium");

                        match shell::add_task(db, &col.name, &title, "", default_priority) {
                            Ok(_) => {
                                app.mode = Mode::Normal;
                                app.input_buffer.clear();
                                Ok((true, false))
                            }
                            Err(e) => {
                                app.set_error(e.to_string());
                                Ok((false, false))
                            }
                        }
                    } else {
                        app.set_error("No board state".into());
                        Ok((false, false))
                    }
                }
            } else if app.mode == Mode::Edit {
                // Save edited task — first commit current field, then diff against original
                if let Some(ref mut task) = app.editing_task {
                    // Save the currently active field from input_buffer into editing_task
                    match app.edit_field {
                        0 => task.title = app.input_buffer.clone(),
                        1 => task.description = app.input_buffer.clone(),
                        _ => {}
                    }
                    let title = task.title.trim().to_string();
                    if title.is_empty() {
                        app.set_error("Title cannot be empty".into());
                        return Ok((false, false));
                    }

                    // New task creation (id is empty)
                    if task.id.is_empty() {
                        if let Some(ref state) = app.state {
                            let col = state
                                .columns
                                .get(app.focused_col_idx)
                                .ok_or_else(|| anyhow::anyhow!("no focused column"))?;
                            // Resolve priority: use task's priority_id if set, else default to "medium"
                            let priority_id = if task.priority_id.is_empty() {
                                state
                                    .priorities
                                    .iter()
                                    .find(|p| p.name == "medium")
                                    .map(|p| p.id.clone())
                                    .unwrap_or_default()
                            } else {
                                task.priority_id.clone()
                            };
                            // Look up priority name
                            let priority_name = state
                                .priorities
                                .iter()
                                .find(|p| p.id == priority_id)
                                .map(|p| p.name.as_str())
                                .unwrap_or("medium");
                            let description = task.description.clone();
                            match shell::add_task(db, &col.name, &title, &description, priority_name) {
                                Ok(_) => {
                                    app.mode = Mode::Normal;
                                    app.editing_task = None;
                                    app.input_buffer.clear();
                                    Ok((true, false))
                                }
                                Err(e) => {
                                    app.set_error(e.to_string());
                                    Ok((false, false))
                                }
                            }
                        } else {
                            app.set_error("No board state".into());
                            Ok((false, false))
                        }
                    } else {
                        // Edit existing task
                        // Diff editing_task against the original in board state
                        let original = app
                            .state
                            .as_ref()
                            .and_then(|s| s.tasks.iter().find(|t| t.id == task.id));
                        let mut changes = TaskChanges::default();
                        if let Some(orig) = original {
                            if task.title != orig.title {
                                changes.title = Some(task.title.clone());
                            }
                            if task.description != orig.description {
                                changes.description = Some(task.description.clone());
                            }
                            if task.priority_id != orig.priority_id {
                                changes.priority_id = Some(task.priority_id.clone());
                            }
                        }
                        if changes == TaskChanges::default() {
                            app.mode = Mode::Normal;
                            app.editing_task = None;
                            app.input_buffer.clear();
                            return Ok((false, false));
                        }
                        match shell::edit_task(db, &task.id, &changes) {
                            Ok(()) => {
                                app.mode = Mode::Normal;
                                app.editing_task = None;
                                app.input_buffer.clear();
                                Ok((true, false))
                            }
                            Err(e) => {
                                app.set_error(e.to_string());
                                Ok((false, false))
                            }
                        }
                    }
                } else {
                    app.mode = Mode::Normal;
                    app.input_buffer.clear();
                    Ok((false, false))
                }
            } else {
                app.mode = Mode::Normal;
                app.input_buffer.clear();
                Ok((false, false))
            }
        }
        Action::Cancel => {
            app.mode = Mode::Normal;
            app.editing_task = None;
            app.input_buffer.clear();
            Ok((false, false))
        }
        Action::CycleField => {
            if app.mode == Mode::Edit {
                // Save current field before cycling
                if let Some(ref mut task) = app.editing_task {
                    match app.edit_field {
                        0 => task.title = app.input_buffer.clone(),
                        1 => task.description = app.input_buffer.clone(),
                        2 => {}
                        _ => {}
                    }
                }

                if app.edit_field < 2 {
                    app.edit_field += 1;
                } else {
                    app.edit_field = 0;
                }

                // Update input buffer for the new field
                if let Some(ref task) = app.editing_task {
                    match app.edit_field {
                        0 => app.input_buffer = task.title.clone(),
                        1 => app.input_buffer = task.description.clone(),
                        _ => app.input_buffer.clear(),
                    }
                }
            }
            Ok((false, false))
        }
        Action::CyclePriority => {
            if let Some(ref mut task) = app.editing_task {
                if let Some(ref state) = app.state {
                    let current_priority_name = state
                        .priorities
                        .iter()
                        .find(|p| p.id == task.priority_id)
                        .map(|p| p.name.as_str())
                        .unwrap_or("medium");
                    let priorities: Vec<&str> =
                        state.priorities.iter().map(|p| p.name.as_str()).collect();
                    if let Some(idx) = priorities.iter().position(|p| *p == current_priority_name) {
                        let next_idx = (idx + 1) % priorities.len();
                        let next_priority_name = priorities[next_idx];
                        task.priority_id = state
                            .priorities
                            .iter()
                            .find(|p| p.name == next_priority_name)
                            .map(|p| p.id.clone())
                            .unwrap_or_default();
                    }
                }
            }
            Ok((false, false))
        }
        Action::InsertText(text) => {
            app.input_buffer.push_str(&text);
            Ok((false, false))
        }
        Action::DeleteChar => {
            app.input_buffer.pop();
            Ok((false, false))
        }
        Action::ConfirmYes => {
            // Task delete confirmation (y key)
            if let Some(task) = app.focused_task() {
                let task_id = task.id.clone();
                match shell::remove_task(db, &task_id) {
                    Ok(()) => {
                        app.mode = Mode::Normal;
                        app.confirm_context = None;
                        let tasks = app.focused_column_tasks();
                        if app.focused_task_idx >= tasks.len() {
                            app.focused_task_idx = tasks.len().saturating_sub(1);
                        }
                        Ok((true, false))
                    }
                    Err(e) => {
                        app.set_error(e.to_string());
                        Ok((false, false))
                    }
                }
            } else {
                app.mode = Mode::Normal;
                app.confirm_context = None;
                Ok((false, false))
            }
        }
        Action::ConfirmNo => {
            app.mode = Mode::Normal;
            app.confirm_context = None;
            Ok((false, false))
        }
        Action::ConfirmMoveToFirst => {
            if let Some(col) = app.focused_column() {
                let col_id = col.id.clone();
                match shell::remove_column(db, &col_id, OrphanAction::MoveToFirst) {
                    Ok(()) => {
                        app.mode = Mode::Normal;
                        app.confirm_context = None;
                        app.focused_col_idx = app.focused_col_idx.saturating_sub(1);
                        app.focused_task_idx = 0;
                        Ok((true, false))
                    }
                    Err(e) => {
                        app.set_error(e.to_string());
                        Ok((false, false))
                    }
                }
            } else {
                app.mode = Mode::Normal;
                app.confirm_context = None;
                Ok((false, false))
            }
        }
        Action::ConfirmDeleteAll => {
            if let Some(col) = app.focused_column() {
                let col_id = col.id.clone();
                match shell::remove_column(db, &col_id, OrphanAction::Delete) {
                    Ok(()) => {
                        app.mode = Mode::Normal;
                        app.confirm_context = None;
                        app.focused_col_idx = app.focused_col_idx.saturating_sub(1);
                        app.focused_task_idx = 0;
                        Ok((true, false))
                    }
                    Err(e) => {
                        app.set_error(e.to_string());
                        Ok((false, false))
                    }
                }
            } else {
                app.mode = Mode::Normal;
                app.confirm_context = None;
                Ok((false, false))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn tui_module_compiles() {
        // Compilation test — the TUI needs a real terminal for full testing
        // Unit tests for input handling and rendering are in input/ and render/
    }
}
