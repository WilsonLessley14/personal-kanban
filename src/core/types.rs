/// A kanban board — the top-level container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A column within a board, holding tasks in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub position: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// A priority level (e.g. low, medium, high).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Priority {
    pub id: String,
    pub name: String,
}

/// A task belonging to a column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub column_id: String,
    pub title: String,
    pub description: String,
    pub priority_id: String,
    pub position: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Complete snapshot of a board's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardState {
    pub board: Board,
    /// Sorted by position.
    pub columns: Vec<Column>,
    /// Sorted by column_id, then position.
    pub tasks: Vec<Task>,
    pub priorities: Vec<Priority>,
}

/// Partial update fields for a task.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskChanges {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority_id: Option<String>,
    pub column_id: Option<String>,
    pub position: Option<i32>,
}

/// Partial update fields for a column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColumnChanges {
    pub name: Option<String>,
    pub position: Option<i32>,
}

/// Filter criteria for listing tasks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskFilter {
    pub column_id: Option<String>,
    pub priority_id: Option<String>,
}

/// What to do with orphaned tasks when their column is deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrphanAction {
    MoveToFirst,
    Delete,
}

/// A typed SQL parameter placeholder value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlParam {
    Text(String),
    Int(i32),
}

/// The entity table being addressed by a position-update SQL statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityTable {
    Column,
    Task,
}

impl EntityTable {
    /// The SQL table name. `column` is reserved, so we use `column_`.
    pub fn table_name(&self) -> &'static str {
        match self {
            EntityTable::Column => "column_",
            EntityTable::Task => "task",
        }
    }
}
