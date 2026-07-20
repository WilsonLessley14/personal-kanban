use personal_kanban::core::{OrphanAction, TaskChanges, TaskFilter};
use personal_kanban::shell::{self, Db};
use tempfile::tempdir;

/// Helper: create a temp DB and init a board, returning the board ID.
fn setup() -> (tempfile::TempDir, Db, String) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let db = Db::open(&path).unwrap();
    let board = shell::init_board(&db, "Test Board").unwrap();
    (dir, db, board.id)
}

// ── init_board ────────────────────────────────────────────────────────────

#[test]
fn init_board_seeds_default_columns_and_priorities() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();

    assert_eq!(state.board.name, "Test Board");

    // 4 default columns in order
    assert_eq!(state.columns.len(), 4);
    assert_eq!(state.columns[0].name, "Backlog");
    assert_eq!(state.columns[0].position, 0);
    assert_eq!(state.columns[1].name, "Todo");
    assert_eq!(state.columns[1].position, 1);
    assert_eq!(state.columns[2].name, "Doing");
    assert_eq!(state.columns[2].position, 2);
    assert_eq!(state.columns[3].name, "Done");
    assert_eq!(state.columns[3].position, 3);

    // 3 default priorities
    assert_eq!(state.priorities.len(), 3);
    let names: Vec<&str> = state.priorities.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"low"));
    assert!(names.contains(&"medium"));
    assert!(names.contains(&"high"));
}

#[test]
fn init_board_rejects_duplicate_name() {
    let (_dir, db, _board_id) = setup();
    let result = shell::init_board(&db, "Test Board");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("already exists"));
}

// ── Board operations ──────────────────────────────────────────────────────

#[test]
fn list_boards_returns_initialized_board() {
    let (_dir, db, _board_id) = setup();
    let boards = shell::list_boards(&db).unwrap();
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "Test Board");
}

#[test]
fn rename_board_by_short_id() {
    let (_dir, db, board_id) = setup();
    // Use just the first 3 chars of the board ID
    let prefix = &board_id[..3];
    shell::rename_board(&db, prefix, "Renamed Board").unwrap();
    let boards = shell::list_boards(&db).unwrap();
    assert_eq!(boards[0].name, "Renamed Board");
}

// ── Column operations ─────────────────────────────────────────────────────

#[test]
fn add_column_appends_to_board() {
    let (_dir, db, board_id) = setup();
    let col = shell::add_column(&db, &board_id, "Review").unwrap();
    assert_eq!(col.name, "Review");

    let state = db.load_board_state(&board_id).unwrap();
    assert_eq!(state.columns.len(), 5);
    let added = state.columns.iter().find(|c| c.name == "Review").unwrap();
    assert_eq!(added.position, 4); // appended after position 3
}

#[test]
fn add_column_rejects_duplicate_name() {
    let (_dir, db, board_id) = setup();
    let result = shell::add_column(&db, &board_id, "Backlog");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn rename_column_by_short_id() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let backlog_id = &state.columns[0].id;
    let prefix = &backlog_id[..3];
    shell::rename_column(&db, prefix, "Inbox").unwrap();

    let state = db.load_board_state(&board_id).unwrap();
    assert_eq!(state.columns[0].name, "Inbox");
}

#[test]
fn move_column_reorders_positions() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let done_id = &state.columns[3].id; // "Done" at position 3
    let prefix = &done_id[..3];

    // Move "Done" from position 3 to position 0
    shell::move_column(&db, prefix, 0).unwrap();

    let state = db.load_board_state(&board_id).unwrap();
    assert_eq!(state.columns[0].name, "Done");
    assert_eq!(state.columns[1].name, "Backlog");
    assert_eq!(state.columns[2].name, "Todo");
    assert_eq!(state.columns[3].name, "Doing");
}

#[test]
fn list_columns_returns_all_columns() {
    let (_dir, db, board_id) = setup();
    let columns = shell::list_columns(&db, &board_id).unwrap();
    assert_eq!(columns.len(), 4);
}

// ── Task operations ──────────────────────────────────────────────────────

#[test]
fn add_task_roundtrip() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let backlog_id = &state.columns[0].id;
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();

    let task = shell::add_task(
        &db,
        backlog_id,
        "Test task",
        "A description",
        &medium_priority,
    )
    .unwrap();
    assert_eq!(task.title, "Test task");
    assert_eq!(task.description, "A description");
    assert_eq!(task.column_id, *backlog_id);

    // Verify it shows up in the board state
    let state = db.load_board_state(&board_id).unwrap();
    let found = state.tasks.iter().find(|t| t.id == task.id).unwrap();
    assert_eq!(found.title, "Test task");
}

#[test]
fn add_task_by_column_name() {
    let (_dir, db, board_id) = setup();
    let medium_priority = db
        .load_board_state(&board_id)
        .unwrap()
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();

    // Reference column by name
    let task = shell::add_task(&db, "backlog", "Name-based task", "", &medium_priority).unwrap();
    assert_eq!(task.title, "Name-based task");
}

#[test]
fn add_task_rejects_empty_title() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let backlog_id = &state.columns[0].id;
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();

    let result = shell::add_task(&db, backlog_id, "", "desc", &medium_priority);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
}

#[test]
fn edit_task_changes_title_and_priority() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let backlog_id = &state.columns[0].id;
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();
    let high_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "high")
        .unwrap()
        .id
        .clone();

    let task = shell::add_task(&db, backlog_id, "Original", "", &medium_priority).unwrap();
    let task_id = task.id;

    let changes = TaskChanges {
        title: Some("Edited".to_string()),
        description: Some("New desc".to_string()),
        priority_id: Some(high_priority.clone()),
        column_id: None,
        position: None,
    };
    shell::edit_task(&db, &task_id, &changes).unwrap();

    let task = shell::show_task(&db, &task_id).unwrap();
    assert_eq!(task.title, "Edited");
    assert_eq!(task.description, "New desc");
    assert_eq!(task.priority_id, high_priority);
}

#[test]
fn edit_task_by_short_id() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let backlog_id = &state.columns[0].id;
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();

    let task = shell::add_task(&db, backlog_id, "Short ID edit", "", &medium_priority).unwrap();
    let short_prefix = &task.id[..3];

    let changes = TaskChanges {
        title: Some("Short-edited".to_string()),
        description: None,
        priority_id: None,
        column_id: None,
        position: None,
    };
    shell::edit_task(&db, short_prefix, &changes).unwrap();

    let found = shell::show_task(&db, short_prefix).unwrap();
    assert_eq!(found.title, "Short-edited");
}

#[test]
fn move_task_between_columns() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let backlog_id = &state.columns[0].id;
    let todo_id = &state.columns[1].id;
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();

    let task = shell::add_task(&db, backlog_id, "To move", "", &medium_priority).unwrap();
    shell::move_task(&db, &task.id, todo_id, None).unwrap();

    let moved = shell::show_task(&db, &task.id).unwrap();
    assert_eq!(moved.column_id, *todo_id);
}

#[test]
fn move_task_with_explicit_position() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let backlog_id = &state.columns[0].id;
    let todo_id = &state.columns[1].id;
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();

    let _task1 = shell::add_task(&db, backlog_id, "Task 1", "", &medium_priority).unwrap();
    let task2 = shell::add_task(&db, backlog_id, "Task 2", "", &medium_priority).unwrap();

    // Move task2 to todo at position 0
    shell::move_task(&db, &task2.id, todo_id, Some(0)).unwrap();
    let moved = shell::show_task(&db, &task2.id).unwrap();
    assert_eq!(moved.column_id, *todo_id);
    assert_eq!(moved.position, 0);
}

#[test]
fn remove_task() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let backlog_id = &state.columns[0].id;
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();

    let task = shell::add_task(&db, backlog_id, "Remove me", "", &medium_priority).unwrap();
    shell::remove_task(&db, &task.id).unwrap();

    let result = shell::show_task(&db, &task.id);
    assert!(result.is_err());
}

#[test]
fn show_task_by_short_id() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let backlog_id = &state.columns[0].id;
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();

    let task = shell::add_task(&db, backlog_id, "Show short", "", &medium_priority).unwrap();
    let short = &task.id[..2];
    let found = shell::show_task(&db, short).unwrap();
    assert_eq!(found.id, task.id);
}

#[test]
fn list_tasks_unfiltered() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let backlog_id = &state.columns[0].id;
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();
    let high_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "high")
        .unwrap()
        .id
        .clone();

    shell::add_task(&db, backlog_id, "Task A", "", &medium_priority).unwrap();
    shell::add_task(&db, backlog_id, "Task B", "", &high_priority).unwrap();

    let tasks = shell::list_tasks(
        &db,
        &board_id,
        &TaskFilter {
            column_id: None,
            priority_id: None,
        },
    )
    .unwrap();
    assert_eq!(tasks.len(), 2);
}

#[test]
fn list_tasks_filtered_by_column() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let backlog_id = &state.columns[0].id;
    let todo_id = &state.columns[1].id;
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();

    shell::add_task(&db, backlog_id, "In backlog", "", &medium_priority).unwrap();
    shell::add_task(&db, todo_id, "In todo", "", &medium_priority).unwrap();

    let tasks = shell::list_tasks(
        &db,
        &board_id,
        &TaskFilter {
            column_id: Some(backlog_id.clone()),
            priority_id: None,
        },
    )
    .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "In backlog");
}

#[test]
fn list_tasks_filtered_by_priority() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let backlog_id = &state.columns[0].id;
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();
    let high_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "high")
        .unwrap()
        .id
        .clone();

    shell::add_task(&db, backlog_id, "Med task", "", &medium_priority).unwrap();
    shell::add_task(&db, backlog_id, "High task", "", &high_priority).unwrap();

    let tasks = shell::list_tasks(
        &db,
        &board_id,
        &TaskFilter {
            column_id: None,
            priority_id: Some(high_priority.clone()),
        },
    )
    .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "High task");
}

// ── remove_column paths ───────────────────────────────────────────────────

#[test]
fn remove_column_empty() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let todo_id = &state.columns[1].id; // "Todo" is empty
    let prefix = &todo_id[..3];

    shell::remove_column(&db, prefix, OrphanAction::Delete).unwrap();

    let state = db.load_board_state(&board_id).unwrap();
    assert_eq!(state.columns.len(), 3);
    assert!(!state.columns.iter().any(|c| c.name == "Todo"));
    // Positions recomputed: 0, 1, 2
    for (i, col) in state.columns.iter().enumerate() {
        assert_eq!(col.position, i as i32);
    }
}

#[test]
fn remove_column_move_tasks_to_first() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let _backlog_id = &state.columns[0].id;
    let todo_id = &state.columns[1].id; // Remove this one
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();

    // Add tasks to the "Todo" column
    shell::add_task(&db, todo_id, "Task in todo 1", "", &medium_priority.clone()).unwrap();
    shell::add_task(&db, todo_id, "Task in todo 2", "", &medium_priority.clone()).unwrap();

    let todo_prefix = &todo_id[..3];
    shell::remove_column(&db, todo_prefix, OrphanAction::MoveToFirst).unwrap();

    let state = db.load_board_state(&board_id).unwrap();
    assert_eq!(state.columns.len(), 3);
    // "Backlog" should now be at position 0; tasks moved there
    let new_backlog = state.columns.iter().find(|c| c.name == "Backlog").unwrap();
    assert_eq!(new_backlog.position, 0);
    let moved_tasks: Vec<&_> = state
        .tasks
        .iter()
        .filter(|t| t.column_id == new_backlog.id)
        .collect();
    assert_eq!(moved_tasks.len(), 2);
}

#[test]
fn remove_column_delete_tasks() {
    let (_dir, db, board_id) = setup();
    let state = db.load_board_state(&board_id).unwrap();
    let todo_id = &state.columns[1].id;
    let medium_priority = state
        .priorities
        .iter()
        .find(|p| p.name == "medium")
        .unwrap()
        .id
        .clone();

    shell::add_task(
        &db,
        todo_id,
        "Will be deleted",
        "",
        &medium_priority.clone(),
    )
    .unwrap();

    let todo_prefix = &todo_id[..3];
    shell::remove_column(&db, todo_prefix, OrphanAction::Delete).unwrap();

    let state = db.load_board_state(&board_id).unwrap();
    assert_eq!(state.columns.len(), 3);
    assert_eq!(
        state
            .tasks
            .iter()
            .filter(|t| t.title == "Will be deleted")
            .count(),
        0
    );
}

#[test]
fn remove_column_cannot_delete_last() {
    let (_dir, db, board_id) = setup();
    // Remove 3 columns, leaving only 1
    let state = db.load_board_state(&board_id).unwrap();
    for col in &state.columns[1..] {
        // Remove Todo, Doing, Done
        shell::remove_column(&db, &col.id[..3], OrphanAction::Delete).unwrap();
    }

    // Now try to delete the last column
    let state = db.load_board_state(&board_id).unwrap();
    assert_eq!(state.columns.len(), 1);
    let last_col = &state.columns[0].id;
    let result = shell::remove_column(&db, &last_col[..3], OrphanAction::Delete);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("last column"));
}
