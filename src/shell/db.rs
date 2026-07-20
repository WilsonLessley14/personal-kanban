use std::cell::RefCell;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::core::{
    query_all_boards, query_board_by_id, query_columns_by_board, query_priorities,
    query_tasks_by_board, Board, BoardState, Column, Priority, Task,
};

/// Embedded migration SQL files. Each entry is (migration_id, name, sql).
fn embedded_migrations() -> &'static [Migration] {
    &[Migration {
        id: 1,
        name: "001_initial_schema",
        sql: include_str!("../../migrations/001_initial_schema.sql"),
    }]
}

struct Migration {
    id: i64,
    name: &'static str,
    sql: &'static str,
}

/// Database handle wrapping a rusqlite Connection with interior mutability.
pub struct Db {
    conn: RefCell<Connection>,
}

impl Db {
    /// Open a database at the given path.
    ///
    /// Creates parent directories and the DB file if missing, then runs all pending
    /// migrations in order inside a transaction.
    pub fn open(path: &Path) -> Result<Self> {
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("failed to open database: {}", path.display()))?;

        // Run migrations
        run_migrations(&conn)?;

        Ok(Db {
            conn: RefCell::new(conn),
        })
    }

    /// Load the complete state of a board by its ID.
    pub fn load_board_state(&self, board_id: &str) -> Result<BoardState> {
        let conn = self.conn.borrow();
        let board = query_board(&conn, board_id)?
            .ok_or_else(|| anyhow::anyhow!("board not found: '{}'", board_id))?;
        let columns = query_columns(&conn, board_id)?;
        let tasks = query_tasks(&conn, board_id)?;
        let priorities = query_priorities_fn(&conn)?;

        Ok(BoardState {
            board,
            columns,
            tasks,
            priorities,
        })
    }

    /// List all boards.
    pub fn list_boards(&self) -> Result<Vec<Board>> {
        let conn = self.conn.borrow();
        query_all_boards_fn(&conn)
    }

    /// Get a mutable reference to the connection for transaction operations.
    pub(crate) fn conn_mut(&self) -> std::cell::RefMut<'_, Connection> {
        self.conn.borrow_mut()
    }
}

// ── Pure query functions that take &Connection ────────────────────────────

fn query_board(conn: &Connection, board_id: &str) -> Result<Option<Board>> {
    let mut stmt = conn.prepare(query_board_by_id())?;
    let board = stmt.query_map(params![board_id], |row| {
        Ok(Board {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        })
    })?;

    let result: Vec<Board> = board.collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(result.into_iter().next())
}

fn query_columns(conn: &Connection, board_id: &str) -> Result<Vec<Column>> {
    let mut stmt = conn.prepare(query_columns_by_board())?;
    let columns = stmt.query_map(params![board_id], |row| {
        Ok(Column {
            id: row.get(0)?,
            board_id: row.get(1)?,
            name: row.get(2)?,
            position: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;

    columns
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to load columns")
}

fn query_tasks(conn: &Connection, board_id: &str) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(query_tasks_by_board())?;
    let tasks = stmt.query_map(params![board_id], |row| {
        Ok(Task {
            id: row.get(0)?,
            column_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            priority_id: row.get(4)?,
            position: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    tasks
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to load tasks")
}

fn query_priorities_fn(conn: &Connection) -> Result<Vec<Priority>> {
    let mut stmt = conn.prepare(query_priorities())?;
    let priorities = stmt.query_map([], |row| {
        Ok(Priority {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;

    priorities
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to load priorities")
}

fn query_all_boards_fn(conn: &Connection) -> Result<Vec<Board>> {
    let mut stmt = conn.prepare(query_all_boards())?;
    let boards = stmt.query_map([], |row| {
        Ok(Board {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        })
    })?;

    boards
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to list boards")
}

/// Run all pending migrations within a transaction.
fn run_migrations(conn: &Connection) -> Result<()> {
    let migrations = embedded_migrations();
    if migrations.is_empty() {
        return Ok(());
    }

    conn.execute("BEGIN", [])
        .context("failed to begin migration transaction")?;

    let result: Result<()> = (|| {
        let applied = get_applied_migration_ids(conn)?;

        for migration in migrations {
            if !applied.contains(&migration.id) {
                conn.execute_batch(migration.sql)
                    .with_context(|| format!("failed to apply migration {}", migration.name))?;
                // Record the migration (it was created by the migration SQL for the first one)
                let exists: bool = conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM _migrations WHERE id = ?1)",
                        params![migration.id],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);
                if !exists {
                    conn.execute(
                        "INSERT INTO _migrations (id, name) VALUES (?1, ?2)",
                        params![migration.id, migration.name],
                    )
                    .with_context(|| format!("failed to record migration {}", migration.name))?;
                }
            }
        }

        conn.execute("COMMIT", [])
            .context("failed to commit migration transaction")?;
        Ok(())
    })();

    if result.is_err() {
        let _ = conn.execute("ROLLBACK", []);
    }

    result
}

/// Get the set of already-applied migration IDs.
fn get_applied_migration_ids(conn: &Connection) -> Result<Vec<i64>> {
    let stmt = conn.prepare("SELECT id FROM _migrations ORDER BY id");
    match stmt {
        Ok(mut stmt) => {
            let ids = stmt
                .query_map([], |row| row.get::<_, i64>(0))
                .map(|rows| rows.collect::<std::result::Result<Vec<_>, _>>());
            match ids {
                Ok(Ok(ids)) => Ok(ids),
                _ => Ok(vec![]),
            }
        }
        Err(_) => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn open_creates_file_and_runs_migrations() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Db::open(&path).unwrap();
        assert!(path.exists());

        // Verify tables exist by checking migrations
        let conn = db.conn.borrow();
        let migrations: Vec<i64> = conn
            .prepare("SELECT id FROM _migrations ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(migrations, vec![1]);
    }

    #[test]
    fn open_is_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        Db::open(&path).unwrap();
        let db = Db::open(&path).unwrap();

        let conn = db.conn.borrow();
        let migrations: Vec<i64> = conn
            .prepare("SELECT id FROM _migrations ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(migrations, vec![1]);
    }

    #[test]
    fn open_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("deep").join("test.db");
        let db = Db::open(&path).unwrap();
        assert!(path.exists());
        // Verify it works
        let boards = db.list_boards().unwrap();
        assert!(boards.is_empty());
    }
}
