use crate::core::types::{
    Board, Column, ColumnChanges, EntityTable, Priority, SqlParam, Task, TaskChanges,
};

// ── Read queries (static SQL strings) ──────────────────────────────────────

/// SELECT a board by its ID.
pub fn query_board_by_id() -> &'static str {
    "SELECT id, name, created_at, updated_at FROM board WHERE id = ?1"
}

/// SELECT columns for a board, ordered by position.
pub fn query_columns_by_board() -> &'static str {
    "SELECT id, board_id, name, position, created_at, updated_at FROM column_ WHERE board_id = ?1 ORDER BY position"
}

/// SELECT tasks for a board (via column), ordered by column and position.
pub fn query_tasks_by_board() -> &'static str {
    "SELECT t.id, t.column_id, t.title, t.description, t.priority_id, t.position, t.created_at, t.updated_at FROM task t JOIN column_ c ON t.column_id = c.id WHERE c.board_id = ?1 ORDER BY t.column_id, t.position"
}

/// SELECT all priorities.
pub fn query_priorities() -> &'static str {
    "SELECT id, name FROM priority ORDER BY name"
}

/// SELECT all boards, ordered by name.
pub fn query_all_boards() -> &'static str {
    "SELECT id, name, created_at, updated_at FROM board ORDER BY name"
}

// ── Write query constructors ───────────────────────────────────────────────

/// Build an INSERT statement for a board.
pub fn insert_board_sql(board: &Board) -> (String, Vec<SqlParam>) {
    let sql =
        "INSERT INTO board (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)".to_string();
    let params = vec![
        SqlParam::Text(board.id.clone()),
        SqlParam::Text(board.name.clone()),
        SqlParam::Text(board.created_at.clone()),
        SqlParam::Text(board.updated_at.clone()),
    ];
    (sql, params)
}

/// Build an INSERT statement for a column.
pub fn insert_column_sql(column: &Column) -> (String, Vec<SqlParam>) {
    let sql = "INSERT INTO column_ (id, board_id, name, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)".to_string();
    let params = vec![
        SqlParam::Text(column.id.clone()),
        SqlParam::Text(column.board_id.clone()),
        SqlParam::Text(column.name.clone()),
        SqlParam::Int(column.position),
        SqlParam::Text(column.created_at.clone()),
        SqlParam::Text(column.updated_at.clone()),
    ];
    (sql, params)
}

/// Build an INSERT statement for a task.
pub fn insert_task_sql(task: &Task) -> (String, Vec<SqlParam>) {
    let sql = "INSERT INTO task (id, column_id, title, description, priority_id, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)".to_string();
    let params = vec![
        SqlParam::Text(task.id.clone()),
        SqlParam::Text(task.column_id.clone()),
        SqlParam::Text(task.title.clone()),
        SqlParam::Text(task.description.clone()),
        SqlParam::Text(task.priority_id.clone()),
        SqlParam::Int(task.position),
        SqlParam::Text(task.created_at.clone()),
        SqlParam::Text(task.updated_at.clone()),
    ];
    (sql, params)
}

/// Build an UPDATE statement for a task based on the provided changes.
pub fn update_task_sql(id: &str, changes: &TaskChanges) -> (String, Vec<SqlParam>) {
    let mut sets = Vec::new();
    let mut params = Vec::new();

    if let Some(ref title) = changes.title {
        sets.push("title = ?".to_string());
        params.push(SqlParam::Text(title.clone()));
    }
    if let Some(ref description) = changes.description {
        sets.push("description = ?".to_string());
        params.push(SqlParam::Text(description.clone()));
    }
    if let Some(ref priority_id) = changes.priority_id {
        sets.push("priority_id = ?".to_string());
        params.push(SqlParam::Text(priority_id.clone()));
    }
    if let Some(ref column_id) = changes.column_id {
        sets.push("column_id = ?".to_string());
        params.push(SqlParam::Text(column_id.clone()));
    }
    if let Some(position) = changes.position {
        sets.push("position = ?".to_string());
        params.push(SqlParam::Int(position));
    }

    params.push(SqlParam::Text(id.to_string()));
    let sql = format!(
        "UPDATE task SET {}, updated_at = datetime('now') WHERE id = ?{}",
        sets.join(", "),
        params.len()
    );
    (sql, params)
}

/// Build an UPDATE statement for a column based on the provided changes.
pub fn update_column_sql(id: &str, changes: &ColumnChanges) -> (String, Vec<SqlParam>) {
    let mut sets = Vec::new();
    let mut params = Vec::new();

    if let Some(ref name) = changes.name {
        sets.push("name = ?".to_string());
        params.push(SqlParam::Text(name.clone()));
    }
    if let Some(position) = changes.position {
        sets.push("position = ?".to_string());
        params.push(SqlParam::Int(position));
    }

    params.push(SqlParam::Text(id.to_string()));
    let sql = format!(
        "UPDATE column_ SET {}, updated_at = datetime('now') WHERE id = ?{}",
        sets.join(", "),
        params.len()
    );
    (sql, params)
}

/// Build a DELETE statement for a task by ID.
pub fn delete_task_sql(id: &str) -> (String, Vec<SqlParam>) {
    let sql = "DELETE FROM task WHERE id = ?1".to_string();
    let params = vec![SqlParam::Text(id.to_string())];
    (sql, params)
}

/// Build a DELETE statement for a column by ID.
pub fn delete_column_sql(id: &str) -> (String, Vec<SqlParam>) {
    let sql = "DELETE FROM column_ WHERE id = ?1".to_string();
    let params = vec![SqlParam::Text(id.to_string())];
    (sql, params)
}

/// Build a DELETE statement for all tasks in a column.
pub fn delete_tasks_by_column_sql(column_id: &str) -> (String, Vec<SqlParam>) {
    let sql = "DELETE FROM task WHERE column_id = ?1".to_string();
    let params = vec![SqlParam::Text(column_id.to_string())];
    (sql, params)
}

/// Build an UPDATE statement to move all tasks from one column to another.
pub fn move_tasks_to_column_sql(
    from_column_id: &str,
    to_column_id: &str,
) -> (String, Vec<SqlParam>) {
    let sql = "UPDATE task SET column_id = ?1 WHERE column_id = ?2".to_string();
    let params = vec![
        SqlParam::Text(to_column_id.to_string()),
        SqlParam::Text(from_column_id.to_string()),
    ];
    (sql, params)
}

/// Build an UPDATE statement for the position of an entity in the given table.
pub fn update_position_sql(table: EntityTable, id: &str, position: i32) -> (String, Vec<SqlParam>) {
    let table_name = table.table_name();
    let sql = format!(
        "UPDATE {} SET position = ?1, updated_at = datetime('now') WHERE id = ?2",
        table_name
    );
    let params = vec![SqlParam::Int(position), SqlParam::Text(id.to_string())];
    (sql, params)
}

/// Build an INSERT statement for a priority.
pub fn insert_priority_sql(priority: &Priority) -> (String, Vec<SqlParam>) {
    let sql = "INSERT INTO priority (id, name) VALUES (?1, ?2)".to_string();
    let params = vec![
        SqlParam::Text(priority.id.clone()),
        SqlParam::Text(priority.name.clone()),
    ];
    (sql, params)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_board(id: &str, name: &str) -> Board {
        Board {
            id: id.to_string(),
            name: name.to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn sample_column(id: &str, board_id: &str, name: &str, pos: i32) -> Column {
        Column {
            id: id.to_string(),
            board_id: board_id.to_string(),
            name: name.to_string(),
            position: pos,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn sample_task(id: &str, column_id: &str, title: &str, priority_id: &str, pos: i32) -> Task {
        Task {
            id: id.to_string(),
            column_id: column_id.to_string(),
            title: title.to_string(),
            description: "".to_string(),
            priority_id: priority_id.to_string(),
            position: pos,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn sample_priority(id: &str, name: &str) -> Priority {
        Priority {
            id: id.to_string(),
            name: name.to_string(),
        }
    }

    // ── Read query tests ─────────────────────────────────────────────────

    #[test]
    fn query_board_by_id_returns_sql() {
        let sql = query_board_by_id();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("board"));
        assert!(sql.contains("WHERE id ="));
    }

    #[test]
    fn query_columns_by_board_returns_sql() {
        let sql = query_columns_by_board();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("column_"));
        assert!(sql.contains("ORDER BY position"));
    }

    #[test]
    fn query_tasks_by_board_returns_sql() {
        let sql = query_tasks_by_board();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("task"));
        assert!(sql.contains("JOIN column_"));
    }

    #[test]
    fn query_priorities_returns_sql() {
        let sql = query_priorities();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("priority"));
    }

    #[test]
    fn query_all_boards_returns_sql() {
        let sql = query_all_boards();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("board"));
        assert!(sql.contains("ORDER BY"));
    }

    // ── Insert query tests ───────────────────────────────────────────────

    #[test]
    fn insert_board_sql_contains_values() {
        let board = sample_board("b1", "Test Board");
        let (sql, params) = insert_board_sql(&board);
        assert!(sql.contains("INSERT INTO board"));
        assert_eq!(params.len(), 4);
        assert_eq!(params[0], SqlParam::Text("b1".to_string()));
        assert_eq!(params[1], SqlParam::Text("Test Board".to_string()));
    }

    #[test]
    fn insert_column_sql_contains_values() {
        let col = sample_column("c1", "b1", "Backlog", 0);
        let (sql, params) = insert_column_sql(&col);
        assert!(sql.contains("INSERT INTO column_"));
        assert_eq!(params.len(), 6);
        assert_eq!(params[0], SqlParam::Text("c1".to_string()));
        assert_eq!(params[3], SqlParam::Int(0));
    }

    #[test]
    fn insert_task_sql_contains_values() {
        let task = sample_task("t1", "c1", "Fix bug", "p1", 0);
        let (sql, params) = insert_task_sql(&task);
        assert!(sql.contains("INSERT INTO task"));
        assert_eq!(params.len(), 8);
        assert_eq!(params[0], SqlParam::Text("t1".to_string()));
        assert_eq!(params[2], SqlParam::Text("Fix bug".to_string()));
        assert_eq!(params[5], SqlParam::Int(0));
    }

    #[test]
    fn insert_priority_sql_contains_values() {
        let prio = sample_priority("p1", "high");
        let (sql, params) = insert_priority_sql(&prio);
        assert!(sql.contains("INSERT INTO priority"));
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], SqlParam::Text("p1".to_string()));
        assert_eq!(params[1], SqlParam::Text("high".to_string()));
    }

    // ── Update query tests ───────────────────────────────────────────────

    #[test]
    fn update_task_sql_single_field() {
        let changes = TaskChanges {
            title: Some("New title".to_string()),
            ..Default::default()
        };
        let (sql, params) = update_task_sql("t1", &changes);
        assert!(sql.contains("UPDATE task SET"));
        assert!(sql.contains("title = ?"));
        assert_eq!(params.len(), 2); // 1 for title + 1 for WHERE id
        assert_eq!(params[0], SqlParam::Text("New title".to_string()));
        assert_eq!(params[1], SqlParam::Text("t1".to_string()));
    }

    #[test]
    fn update_task_sql_multiple_fields() {
        let changes = TaskChanges {
            title: Some("Updated".to_string()),
            priority_id: Some("p2".to_string()),
            position: Some(5),
            ..Default::default()
        };
        let (sql, params) = update_task_sql("t1", &changes);
        assert!(sql.contains("title = ?"));
        assert!(sql.contains("priority_id = ?"));
        assert!(sql.contains("position = ?"));
        assert_eq!(params.len(), 4); // 3 changes + 1 WHERE id
    }

    #[test]
    fn update_column_sql_single_field() {
        let changes = ColumnChanges {
            name: Some("Renamed".to_string()),
            ..Default::default()
        };
        let (sql, params) = update_column_sql("c1", &changes);
        assert!(sql.contains("UPDATE column_ SET"));
        assert!(sql.contains("name = ?"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn update_column_sql_position() {
        let changes = ColumnChanges {
            position: Some(3),
            ..Default::default()
        };
        let (sql, params) = update_column_sql("c1", &changes);
        assert!(sql.contains("position = ?"));
        assert_eq!(params[0], SqlParam::Int(3));
    }

    // ── Delete query tests ───────────────────────────────────────────────

    #[test]
    fn delete_task_sql_test() {
        let (sql, params) = super::delete_task_sql("t1");
        assert!(sql.contains("DELETE FROM task"));
        assert!(sql.contains("WHERE id = ?1"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], SqlParam::Text("t1".to_string()));
    }

    #[test]
    fn delete_column_sql_test() {
        let (sql, params) = super::delete_column_sql("c1");
        assert!(sql.contains("DELETE FROM column_"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], SqlParam::Text("c1".to_string()));
    }

    #[test]
    fn delete_tasks_by_column_sql_test() {
        let (sql, params) = super::delete_tasks_by_column_sql("c1");
        assert!(sql.contains("DELETE FROM task"));
        assert!(sql.contains("WHERE column_id = ?1"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], SqlParam::Text("c1".to_string()));
    }

    // ── Move query tests ─────────────────────────────────────────────────

    #[test]
    fn move_tasks_to_column_sql_test() {
        let (sql, params) = super::move_tasks_to_column_sql("c1", "c2");
        assert!(sql.contains("UPDATE task SET column_id = ?1"));
        assert!(sql.contains("WHERE column_id = ?2"));
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], SqlParam::Text("c2".to_string()));
        assert_eq!(params[1], SqlParam::Text("c1".to_string()));
    }

    // ── Position update tests ────────────────────────────────────────────

    #[test]
    fn update_position_sql_column() {
        let (sql, params) = update_position_sql(EntityTable::Column, "c1", 5);
        assert!(sql.contains("UPDATE column_ SET position = ?1"));
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], SqlParam::Int(5));
    }

    #[test]
    fn update_position_sql_task() {
        let (sql, params) = update_position_sql(EntityTable::Task, "t1", 2);
        assert!(sql.contains("UPDATE task SET position = ?1"));
        assert_eq!(params[0], SqlParam::Int(2));
    }

    #[test]
    fn entity_table_table_name() {
        assert_eq!(EntityTable::Column.table_name(), "column_");
        assert_eq!(EntityTable::Task.table_name(), "task");
    }
}
