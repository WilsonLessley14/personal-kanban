use anyhow::{anyhow, Context, Result};

use crate::core::{
    delete_column_sql, delete_task_sql, delete_tasks_by_column_sql, insert_board_sql,
    insert_column_sql, insert_priority_sql, insert_task_sql, move_tasks_to_column_sql,
    next_position, positions_after_move, recompute_positions, resolve_id, update_position_sql,
    update_task_sql, validate_column_exists, validate_column_name, validate_priority,
    validate_title, Board, Column, DomainError, EntityTable, OrphanAction, Priority, SqlParam,
    Task, TaskChanges, TaskFilter,
};

use super::db::Db;

// ── Helper to bind SqlParam values for rusqlite ───────────────────────────

fn exec_with_params(conn: &rusqlite::Connection, sql: &str, params: &[SqlParam]) -> Result<()> {
    let rusqlite_params: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|p| match p {
            SqlParam::Text(s) => s as &dyn rusqlite::types::ToSql,
            SqlParam::Int(i) => i as &dyn rusqlite::types::ToSql,
        })
        .collect();
    let mut stmt = conn.prepare(sql).context("failed to prepare statement")?;
    stmt.execute(rusqlite::params_from_iter(rusqlite_params.iter().copied()))
        .context("failed to execute statement")?;
    Ok(())
}

fn exec_with_params_raw(
    conn: &rusqlite::Transaction,
    sql: &str,
    params: &[SqlParam],
) -> Result<()> {
    let rusqlite_params: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|p| match p {
            SqlParam::Text(s) => s as &dyn rusqlite::types::ToSql,
            SqlParam::Int(i) => i as &dyn rusqlite::types::ToSql,
        })
        .collect();
    let mut stmt = conn.prepare(sql).context("failed to prepare statement")?;
    stmt.execute(rusqlite::params_from_iter(rusqlite_params.iter().copied()))
        .context("failed to execute statement")?;
    Ok(())
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn gen_id() -> String {
    nanoid::nanoid!(10)
}

// ── Board operations ──────────────────────────────────────────────────────

/// Initialize a new board with default columns and priorities.
pub fn init_board(db: &Db, name: &str) -> Result<Board> {
    // Check for duplicate board name
    let existing = db.list_boards().context("failed to list boards")?;
    if existing.iter().any(|b| b.name == name) {
        return Err(DomainError::DuplicateBoardName {
            name: name.to_string(),
        }
        .into());
    }

    let board_id = gen_id();
    let timestamp = now();
    let board = Board {
        id: board_id.clone(),
        name: name.to_string(),
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
    };

    let default_columns = [("Backlog", 0), ("Todo", 1), ("Doing", 2), ("Done", 3)];
    let default_priorities = ["low", "medium", "high"];

    let mut conn = db.conn_mut();
    let tx = conn.transaction().context("failed to begin transaction")?;

    // Insert board
    let (sql, params) = insert_board_sql(&board);
    exec_with_params_raw(&tx, &sql, &params)?;

    // Insert columns
    for (col_name, pos) in &default_columns {
        let col = Column {
            id: gen_id(),
            board_id: board_id.clone(),
            name: col_name.to_string(),
            position: *pos,
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
        };
        let (sql, params) = insert_column_sql(&col);
        exec_with_params_raw(&tx, &sql, &params)?;
    }

    // Insert priorities
    for prio_name in &default_priorities {
        let prio = Priority {
            id: gen_id(),
            name: prio_name.to_string(),
        };
        let (sql, params) = insert_priority_sql(&prio);
        exec_with_params_raw(&tx, &sql, &params)?;
    }

    tx.commit().context("failed to commit transaction")?;

    Ok(board)
}

/// List all boards.
pub fn list_boards(db: &Db) -> Result<Vec<Board>> {
    db.list_boards()
}

/// Rename a board.
pub fn rename_board(db: &Db, board_id: &str, new_name: &str) -> Result<()> {
    let boards = db.list_boards()?;
    let resolved_id = resolve_id(
        board_id,
        &boards.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(),
    )
    .map_err(|e| anyhow!("{}", e))?;

    // Check for duplicate name (excluding this board)
    if boards
        .iter()
        .any(|b| b.id != resolved_id && b.name == new_name)
    {
        return Err(DomainError::DuplicateBoardName {
            name: new_name.to_string(),
        }
        .into());
    }

    let conn = db.conn_mut();
    let sql = "UPDATE board SET name = ?1, updated_at = ?2 WHERE id = ?3";
    conn.execute(sql, rusqlite::params![new_name, now(), resolved_id])
        .context("failed to rename board")?;

    Ok(())
}

// ── Column operations ─────────────────────────────────────────────────────

/// Add a column to a board.
pub fn add_column(db: &Db, board_id: &str, name: &str) -> Result<Column> {
    validate_column_name(name, &[]).map_err(|e| anyhow!("{}", e))?;

    let state = db
        .load_board_state(board_id)
        .map_err(|_| DomainError::BoardNotFound {
            id: board_id.to_string(),
        })?;

    validate_column_name(name, &state.columns).map_err(|e| anyhow!("{}", e))?;

    let positions: Vec<i32> = state.columns.iter().map(|c| c.position).collect();
    let next_pos = next_position(&positions);

    let column = Column {
        id: gen_id(),
        board_id: board_id.to_string(),
        name: name.to_string(),
        position: next_pos,
        created_at: now(),
        updated_at: now(),
    };

    let conn = db.conn_mut();
    let (sql, params) = insert_column_sql(&column);
    exec_with_params(&conn, &sql, &params)?;

    Ok(column)
}

/// Rename a column.
pub fn rename_column(db: &Db, column_id_prefix: &str, new_name: &str) -> Result<()> {
    validate_column_name(new_name, &[]).map_err(|e| anyhow!("{}", e))?;

    let state = load_board_state_for_columns(db)?;
    let resolved_id = resolve_column_id(&state, column_id_prefix)?;
    validate_column_name(new_name, &state.columns).map_err(|e| anyhow!("{}", e))?;

    let conn = db.conn_mut();
    let sql = "UPDATE column_ SET name = ?1, updated_at = ?2 WHERE id = ?3";
    conn.execute(sql, rusqlite::params![new_name, now(), resolved_id])
        .context("failed to rename column")?;

    Ok(())
}

/// Remove a column with the specified orphan action.
pub fn remove_column(db: &Db, column_id_prefix: &str, orphan_action: OrphanAction) -> Result<()> {
    let state = load_board_state_for_columns(db)?;
    let resolved_id = resolve_column_id(&state, column_id_prefix)?;

    // Cannot delete the last column
    if state.columns.len() <= 1 {
        return Err(DomainError::CannotDeleteLastColumn.into());
    }

    // Find the column's board_id
    let board_id = state
        .columns
        .iter()
        .find(|c| c.id == resolved_id)
        .map(|c| c.board_id.clone())
        .ok_or_else(|| DomainError::ColumnNotFound {
            id: resolved_id.clone(),
        })?;

    let mut conn = db.conn_mut();

    // Count tasks in this column
    let task_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM task WHERE column_id = ?1",
            rusqlite::params![&resolved_id],
            |row| row.get(0),
        )
        .context("failed to count tasks")?;

    let tx = conn.transaction().context("failed to begin transaction")?;

    if task_count > 0 {
        match orphan_action {
            OrphanAction::MoveToFirst => {
                // Find the column that will have position 0 after deletion
                let remaining: Vec<&Column> = state
                    .columns
                    .iter()
                    .filter(|c| c.id != resolved_id)
                    .collect();
                let sorted = recompute_positions(
                    &remaining
                        .iter()
                        .map(|c| (c.id.clone(), c.position))
                        .collect::<Vec<_>>(),
                );
                // The first column after recomputation gets position 0
                let target_column_id = &sorted[0].0;

                // Move all tasks to the target column
                let (sql, params) = move_tasks_to_column_sql(&resolved_id, target_column_id);
                exec_with_params_raw(&tx, &sql, &params)?;

                // Fetch all tasks in target column and recompute positions
                let new_positions =
                    {
                        let mut stmt = tx.prepare(
                        "SELECT id, position FROM task WHERE column_id = ?1 ORDER BY position"
                    ).context("failed to prepare task query")?;
                        let task_rows = stmt
                            .query_map(rusqlite::params![target_column_id], |row| {
                                Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
                            })
                            .context("failed to query tasks")?;
                        let task_rows: Vec<(String, i32)> = task_rows
                            .collect::<std::result::Result<Vec<_>, _>>()
                            .context("failed to collect tasks")?;
                        recompute_positions(&task_rows)
                    };
                for (id, pos) in &new_positions {
                    let (sql, params) = update_position_sql(EntityTable::Task, id, *pos);
                    exec_with_params_raw(&tx, &sql, &params)?;
                }
            }
            OrphanAction::Delete => {
                let (sql, params) = delete_tasks_by_column_sql(&resolved_id);
                exec_with_params_raw(&tx, &sql, &params)?;
            }
        }
    }

    // Delete the column
    let (sql, params) = delete_column_sql(&resolved_id);
    exec_with_params_raw(&tx, &sql, &params)?;

    // Recompute remaining column positions
    let new_positions = {
        let mut stmt = tx
            .prepare("SELECT id, position FROM column_ WHERE board_id = ?1 ORDER BY position")
            .context("failed to prepare column query")?;
        let remaining_columns = stmt
            .query_map(rusqlite::params![&board_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
            })
            .context("failed to query columns")?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("failed to collect columns")?;
        recompute_positions(&remaining_columns)
    };
    for (id, pos) in &new_positions {
        let (sql, params) = update_position_sql(EntityTable::Column, id, *pos);
        exec_with_params_raw(&tx, &sql, &params)?;
    }

    tx.commit().context("failed to commit transaction")?;

    Ok(())
}

/// Move a column to a new position.
pub fn move_column(db: &Db, column_id_prefix: &str, new_position: i32) -> Result<()> {
    let state = load_board_state_for_columns(db)?;
    let resolved_id = resolve_column_id(&state, column_id_prefix)?;

    let max_pos = if state.columns.is_empty() {
        0
    } else {
        state.columns.len() as i32
    };
    if new_position < 0 || new_position >= max_pos {
        return Err(DomainError::PositionOutOfRange {
            position: new_position,
            max: max_pos,
        }
        .into());
    }

    let column = state
        .columns
        .iter()
        .find(|c| c.id == resolved_id)
        .ok_or_else(|| DomainError::ColumnNotFound {
            id: resolved_id.clone(),
        })?;

    let old_position = column.position;

    let mut conn = db.conn_mut();
    let tx = conn.transaction().context("failed to begin transaction")?;

    // Compute new positions for all columns
    let items: Vec<(String, i32)> = state
        .columns
        .iter()
        .map(|c| (c.id.clone(), c.position))
        .collect();

    let shifted = positions_after_move(&items, old_position, new_position);
    let new_positions = recompute_positions(&shifted);

    for (id, pos) in &new_positions {
        let (sql, params) = update_position_sql(EntityTable::Column, id, *pos);
        exec_with_params_raw(&tx, &sql, &params)?;
    }

    tx.commit().context("failed to commit transaction")?;

    Ok(())
}

/// List columns for a board.
pub fn list_columns(db: &Db, board_id: &str) -> Result<Vec<Column>> {
    let state = db
        .load_board_state(board_id)
        .map_err(|_| DomainError::BoardNotFound {
            id: board_id.to_string(),
        })?;
    Ok(state.columns)
}

// ── Task operations ───────────────────────────────────────────────────────

/// Add a task to a column.
pub fn add_task(
    db: &Db,
    column_id_prefix: &str,
    title: &str,
    desc: &str,
    priority_id_prefix: &str,
) -> Result<Task> {
    validate_title(title).map_err(|e| anyhow!("{}", e))?;

    let state = load_board_state_for_tasks(db)?;
    let resolved_column_id = resolve_column_id(&state, column_id_prefix)?;
    validate_column_exists(&resolved_column_id, &state.columns).map_err(|e| anyhow!("{}", e))?;

    // Resolve priority by ID or name
    let resolved_priority_id = resolve_priority_id(&state, priority_id_prefix)?;

    validate_priority(&resolved_priority_id, &state.priorities).map_err(|e| anyhow!("{}", e))?;

    // Find tasks in the target column to compute next position
    let column_tasks: Vec<&Task> = state
        .tasks
        .iter()
        .filter(|t| t.column_id == resolved_column_id)
        .collect();
    let positions: Vec<i32> = column_tasks.iter().map(|t| t.position).collect();
    let next_pos = next_position(&positions);

    let task = Task {
        id: gen_id(),
        column_id: resolved_column_id,
        title: title.to_string(),
        description: desc.to_string(),
        priority_id: resolved_priority_id,
        position: next_pos,
        created_at: now(),
        updated_at: now(),
    };

    let conn = db.conn_mut();
    let (sql, params) = insert_task_sql(&task);
    exec_with_params(&conn, &sql, &params)?;

    Ok(task)
}

/// Edit a task.
pub fn edit_task(db: &Db, task_id_prefix: &str, changes: &TaskChanges) -> Result<()> {
    let state = load_board_state_for_tasks(db)?;
    let resolved_id = resolve_task_id(&state, task_id_prefix)?;

    // Validate priority if changed
    if let Some(ref priority_id) = changes.priority_id {
        validate_priority(priority_id, &state.priorities).map_err(|e| anyhow!("{}", e))?;
    }

    // Validate column if changed
    if let Some(ref column_id) = changes.column_id {
        validate_column_exists(column_id, &state.columns).map_err(|e| anyhow!("{}", e))?;
    }

    // Validate title if changed
    if let Some(ref title) = changes.title {
        validate_title(title).map_err(|e| anyhow!("{}", e))?;
    }

    let conn = db.conn_mut();
    let (sql, params) = update_task_sql(&resolved_id, changes);
    exec_with_params(&conn, &sql, &params)?;

    Ok(())
}

/// Move a task to a different column/position.
pub fn move_task(
    db: &Db,
    task_id_prefix: &str,
    target_column_prefix: &str,
    position: Option<i32>,
) -> Result<()> {
    let state = load_board_state_for_tasks(db)?;
    let resolved_task_id = resolve_task_id(&state, task_id_prefix)?;
    let resolved_column_id = resolve_column_id(&state, target_column_prefix)?;

    let _task = state
        .tasks
        .iter()
        .find(|t| t.id == resolved_task_id)
        .ok_or_else(|| DomainError::TaskNotFound {
            id: resolved_task_id.clone(),
        })?;

    let mut conn = db.conn_mut();
    let tx = conn.transaction().context("failed to begin transaction")?;

    let target_position = if let Some(pos) = position {
        pos
    } else {
        // Append at end of target column
        let target_tasks: Vec<i32> = state
            .tasks
            .iter()
            .filter(|t| t.column_id == resolved_column_id && t.id != resolved_task_id)
            .map(|t| t.position)
            .collect();
        next_position(&target_tasks)
    };

    let sql = "UPDATE task SET column_id = ?1, position = ?2, updated_at = ?3 WHERE id = ?4";
    tx.execute(
        sql,
        rusqlite::params![resolved_column_id, target_position, now(), resolved_task_id],
    )
    .context("failed to move task")?;

    tx.commit().context("failed to commit transaction")?;

    Ok(())
}

/// Remove a task.
pub fn remove_task(db: &Db, task_id_prefix: &str) -> Result<()> {
    let state = load_board_state_for_tasks(db)?;
    let resolved_id = resolve_task_id(&state, task_id_prefix)?;

    let conn = db.conn_mut();
    let (sql, params) = delete_task_sql(&resolved_id);
    exec_with_params(&conn, &sql, &params)?;

    Ok(())
}

/// Show a single task.
pub fn show_task(db: &Db, task_id_prefix: &str) -> Result<Task> {
    let state = load_board_state_for_tasks(db)?;
    let resolved_id = resolve_task_id(&state, task_id_prefix)?;

    state
        .tasks
        .iter()
        .find(|t| t.id == resolved_id)
        .cloned()
        .ok_or_else(|| anyhow!("task not found: '{}'", resolved_id))
}

/// Reorder a task within its column (swap with adjacent task).
pub fn reorder_task(db: &Db, task_id: &str, direction: OrderDirection) -> Result<()> {
    let state = load_board_state_for_tasks(db)?;
    let task = state
        .tasks
        .iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| DomainError::TaskNotFound {
            id: task_id.to_string(),
        })?;

    let col_tasks: Vec<&Task> = state
        .tasks
        .iter()
        .filter(|t| t.column_id == task.column_id)
        .collect();

    let current_pos = task.position;
    let adjacent = match direction {
        OrderDirection::Up => col_tasks
            .iter()
            .find(|t| t.position < current_pos)
            .map(|t| t.position),
        OrderDirection::Down => col_tasks
            .iter()
            .filter(|t| t.position > current_pos)
            .min_by_key(|t| t.position)
            .map(|t| t.position),
    };

    let adjacent_pos = adjacent.ok_or_else(|| anyhow!("already at {direction}"))?;
    let adjacent_task = col_tasks
        .iter()
        .find(|t| t.position == adjacent_pos && t.id != task_id)
        .ok_or_else(|| anyhow!("no adjacent task to swap"))?;

    let mut conn = db.conn_mut();
    let tx = conn.transaction().context("tx fail")?;
    let now = now();

    // Swap positions
    let sql = "UPDATE task SET position = ?1, updated_at = ?2 WHERE id = ?3";
    tx.execute(sql, rusqlite::params![current_pos, &now, adjacent_task.id])
        .context("swap fail")?;
    tx.execute(sql, rusqlite::params![adjacent_pos, &now, task_id])
        .context("swap fail")?;

    tx.commit().context("commit fail")?;
    Ok(())
}

/// Direction for task reordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDirection {
    Up,
    Down,
}

impl std::fmt::Display for OrderDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderDirection::Up => write!(f, "top"),
            OrderDirection::Down => write!(f, "bottom"),
        }
    }
}

/// List tasks with optional filtering.
pub fn list_tasks(db: &Db, board_id: &str, filter: &TaskFilter) -> Result<Vec<Task>> {
    let state = db
        .load_board_state(board_id)
        .map_err(|_| DomainError::BoardNotFound {
            id: board_id.to_string(),
        })?;

    // Resolve filters before taking ownership of tasks
    let resolved_col_id = if let Some(ref col_id) = filter.column_id {
        Some(resolve_column_id(&state, col_id)?)
    } else {
        None
    };
    let resolved_prio_id = if let Some(ref prio_id) = filter.priority_id {
        Some(resolve_priority_id(&state, prio_id)?)
    } else {
        None
    };

    let mut tasks = state.tasks;

    // Apply column filter
    if let Some(ref col_id) = resolved_col_id {
        tasks.retain(|t| t.column_id == *col_id);
    }

    // Apply priority filter
    if let Some(ref prio_id) = resolved_prio_id {
        tasks.retain(|t| t.priority_id == *prio_id);
    }

    Ok(tasks)
}

// ── Internal helpers ──────────────────────────────────────────────────────

fn resolve_column_id(state: &crate::core::BoardState, prefix: &str) -> Result<String> {
    let ids: Vec<&str> = state.columns.iter().map(|c| c.id.as_str()).collect();

    // Try ID resolution first
    if let Ok(id) = resolve_id(prefix, &ids) {
        return Ok(id);
    }

    // Fall back to case-insensitive name matching
    let matches: Vec<&Column> = state
        .columns
        .iter()
        .filter(|c| c.name.eq_ignore_ascii_case(prefix))
        .collect();

    match matches.len() {
        0 => Err(DomainError::ColumnNotFound {
            id: prefix.to_string(),
        }
        .into()),
        1 => Ok(matches[0].id.clone()),
        _ => Err(anyhow!("multiple columns match name '{}'", prefix)),
    }
}

fn resolve_task_id(state: &crate::core::BoardState, prefix: &str) -> Result<String> {
    let ids: Vec<&str> = state.tasks.iter().map(|t| t.id.as_str()).collect();
    resolve_id(prefix, &ids).map_err(|e| anyhow!("{}", e))
}

/// Resolve a priority by ID prefix or case-insensitive name.
fn resolve_priority_id(state: &crate::core::BoardState, prefix: &str) -> Result<String> {
    let ids: Vec<&str> = state.priorities.iter().map(|p| p.id.as_str()).collect();

    // Try ID resolution first
    if let Ok(id) = resolve_id(prefix, &ids) {
        return Ok(id);
    }

    // Fall back to case-insensitive name matching
    let matches: Vec<&Priority> = state
        .priorities
        .iter()
        .filter(|p| p.name.eq_ignore_ascii_case(prefix))
        .collect();

    match matches.len() {
        0 => Err(DomainError::PriorityNotFound {
            id: prefix.to_string(),
        }
        .into()),
        1 => Ok(matches[0].id.clone()),
        _ => Err(anyhow!("multiple priorities match name '{}'", prefix)),
    }
}

/// Load board state for operations that need columns (auto-detects board).
fn load_board_state_for_columns(db: &Db) -> Result<crate::core::BoardState> {
    let boards = db.list_boards()?;
    let board = boards
        .first()
        .ok_or_else(|| anyhow::anyhow!("no boards found; run 'init' first"))?;

    db.load_board_state(&board.id)
}

/// Load board state for task operations (same as columns, loads all data).
fn load_board_state_for_tasks(db: &Db) -> Result<crate::core::BoardState> {
    load_board_state_for_columns(db)
}
