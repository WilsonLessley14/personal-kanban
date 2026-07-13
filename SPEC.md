# Personal Kanban — Specification

## Overview

A personal kanban board tool with a CLI and TUI interface, built in Rust. The architecture
follows a three-layer pattern:

1. **Core** — Pure types, enums, parsing logic, SQL query constructors, validation. No side
   effects, no I/O.
2. **Imperative Shell** — Executes operations against SQLite using core types and queries.
   Owns all I/O: database reads/writes, filesystem operations, interactive prompts.
3. **UX Layer** — User-facing frontends (CLI and TUI) that translate user intent into shell
   operations and render results. Future: REST API.

Both the CLI and TUI invoke the same shell operations. Every action available in the TUI is
a CLI subcommand and vice versa.

Binary name: `kanban` (with a `pk` alias installed alongside it).

---

## 1. Data Model

### Board

A board is a self-contained unit. MVP ships with a single default board created on first run.
The core models a board as an isolated instance so multi-board support later is just
instantiation + UX glue.

| Field        | Type       | Notes                          |
|--------------|------------|--------------------------------|
| `id`         | `TEXT PK`  | nanoid                         |
| `name`       | `TEXT`     | Human-readable board name      |
| `created_at` | `DATETIME` | Auto-set on creation           |
| `updated_at` | `DATETIME` | Auto-set on mutation           |

### Column

Columns are ordered containers for tasks within a board.

| Field        | Type       | Notes                                    |
|--------------|------------|------------------------------------------|
| `id`         | `TEXT PK`  | nanoid                                   |
| `board_id`   | `TEXT FK`  | References `board.id`                    |
| `name`       | `TEXT`     | Display name (e.g. "Doing")              |
| `position`   | `INTEGER`  | Explicit ordering, 0-indexed             |
| `created_at` | `DATETIME` | Auto-set                                 |
| `updated_at` | `DATETIME` | Auto-set                                 |

### Priority

Priority is a first-class entity stored in its own table, not a hardcoded enum. This allows
users to define custom priority levels in future versions.

| Field  | Type       | Notes                          |
|--------|------------|--------------------------------|
| `id`   | `TEXT PK`  | nanoid                         |
| `name` | `TEXT`     | Display name (e.g. "high")     |

Default priorities seeded on init: `low`, `medium`, `high`.

### Task

| Field         | Type       | Notes                                    |
|---------------|------------|------------------------------------------|
| `id`          | `TEXT PK`  | nanoid                                   |
| `column_id`   | `TEXT FK`  | References `column_.id`                  |
| `title`       | `TEXT`     | Required, non-empty                      |
| `description` | `TEXT`     | Optional, can be empty                   |
| `priority_id` | `TEXT FK`  | References `priority.id`                 |
| `position`    | `INTEGER`  | Ordering within column, 0-indexed        |
| `created_at`  | `DATETIME` | Auto-set                                 |
| `updated_at`  | `DATETIME` | Auto-set                                 |

### ID Generation and Short-ID Resolution

All entity IDs use nanoid (short random strings, e.g. `V1StGXR8_Z5jdHi6B`).

**Short-ID matching:** Users can reference any entity by a prefix of its ID — only enough
characters to be unique within the selectable set for that operation. For example, if the
board has tasks with IDs `a3x9k2`, `a3bQ7f`, and `m8rT2p`, then:

- `m` uniquely matches `m8rT2p` (only one ID starts with `m`)
- `a3x` uniquely matches `a3x9k2` (both `a3...` IDs need 3 chars to disambiguate)
- `a3` is ambiguous → error with the matching candidates listed

**Core function:** `resolve_id(prefix: &str, candidates: &[&str]) -> Result<String, IdError>`

Returns the full ID if exactly one candidate matches the prefix. Returns
`IdError::Ambiguous { prefix, matches }` if multiple match. Returns
`IdError::NotFound { prefix }` if none match.

**TUI display:** In the TUI, task IDs are rendered with the minimum-unique prefix in bold
(or bright color) and the remaining characters dimmed, so the user can see at a glance what
to type. The minimum-unique length is computed per render frame against the visible set.

**Core function:** `min_unique_prefixes(ids: &[&str]) -> Vec<(String, usize)>`

Returns each ID paired with the minimum prefix length needed for uniqueness.

### Default Board State

A new board is initialized with four columns in this order:

1. Backlog (position 0)
2. Todo (position 1)
3. Doing (position 2)
4. Done (position 3)

And three default priorities: `low`, `medium`, `high`.

Default priority for new tasks: `medium`.

---

## 2. Storage

### Format: SQLite

Single-file SQLite database via `rusqlite` with the `bundled` feature (SQLite amalgamation
compiled into the binary — no external C dependency at runtime).

Default location: `$XDG_DATA_HOME/kanban/kanban.db`
(typically `~/.local/share/kanban/kanban.db`)

Overridable via:
- `--db <path>` CLI flag
- `KANBAN_DB` environment variable

Priority: CLI flag > env var > XDG default.

### Schema

```sql
CREATE TABLE board (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE priority (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE column_ (
    id         TEXT PRIMARY KEY,
    board_id   TEXT NOT NULL REFERENCES board(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    position   INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE task (
    id          TEXT PRIMARY KEY,
    column_id   TEXT NOT NULL REFERENCES column_(id),
    title       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    priority_id TEXT NOT NULL REFERENCES priority(id),
    position    INTEGER NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE _migrations (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_column_board ON column_(board_id, position);
CREATE INDEX idx_task_column ON task(column_id, position);
```

Note: `column_` has a trailing underscore because `column` is a SQL reserved word.

Note: `task.column_id` does NOT use `ON DELETE CASCADE`. Column deletion requires explicit
user decision about orphaned tasks (see section 5, Column Deletion Behavior).

### Migrations

Migrations are stored as numbered SQL files in the source tree:

```
migrations/
├── 001_initial_schema.sql
├── 002_some_future_change.sql
└── ...
```

These are embedded into the binary at compile time (via `include_str!` or `rust-embed`).

On startup, the shell:
1. Creates the `_migrations` table if it doesn't exist
2. Reads which migrations have been applied (by `id`)
3. Runs any unapplied migrations in order, within a transaction
4. Records each applied migration in `_migrations`

Migration files are plain SQL. Each file name follows the pattern
`NNN_description.sql` where `NNN` is a zero-padded sequential number.

### Performance Note

This database serves a single user's personal kanban. Read/write volume is negligible.
The schema prioritizes clarity and correctness over optimization. Foreign keys, separate
tables for priority, and explicit position columns are all fine at this scale.

---

## 3. Architecture: Core / Shell / UX

### Layer 1: Core (`core/`)

Pure library code. No I/O, no side effects, no `std::fs`, no database access.

The core provides:

#### Types and Enums

```rust
/// Domain entity: a kanban board
struct Board {
    id: String,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Domain entity: a column within a board
struct Column {
    id: String,
    board_id: String,
    name: String,
    position: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Domain entity: a priority level
struct Priority {
    id: String,
    name: String,
}

/// Domain entity: a task within a column
struct Task {
    id: String,
    column_id: String,
    title: String,
    description: String,
    priority_id: String,
    position: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Complete snapshot of a board's state, used as input to core functions
struct BoardState {
    board: Board,
    columns: Vec<Column>,       // sorted by position
    tasks: Vec<Task>,           // sorted by column_id, then position
    priorities: Vec<Priority>,
}

/// Describes a change to a task (only populated fields are updated)
struct TaskChanges {
    title: Option<String>,
    description: Option<String>,
    priority_id: Option<String>,
    column_id: Option<String>,
    position: Option<i32>,
}

/// Describes a change to a column
struct ColumnChanges {
    name: Option<String>,
    position: Option<i32>,
}
```

#### Validation Functions

```rust
/// Validate a task title (non-empty, reasonable length)
fn validate_title(title: &str) -> Result<(), DomainError>

/// Validate a column name (non-empty, unique within board)
fn validate_column_name(name: &str, existing: &[Column]) -> Result<(), DomainError>

/// Validate a priority exists
fn validate_priority(priority_id: &str, priorities: &[Priority]) -> Result<(), DomainError>

/// Validate a column exists within the board
fn validate_column_exists(column_id: &str, columns: &[Column]) -> Result<(), DomainError>
```

#### Position Computation

```rust
/// Compute the next position for a new item in an ordered list
fn next_position(existing_positions: &[i32]) -> i32

/// Recompute positions to be gap-free (0, 1, 2, ...) after a delete or move
fn recompute_positions(items: &[(String, i32)]) -> Vec<(String, i32)>

/// Compute new positions after inserting at a specific index
fn positions_after_insert(existing: &[(String, i32)], insert_at: i32) -> Vec<(String, i32)>

/// Compute new positions after moving an item from one index to another
fn positions_after_move(existing: &[(String, i32)], from: i32, to: i32) -> Vec<(String, i32)>
```

#### ID Resolution

```rust
/// Resolve a user-provided ID prefix to a full ID
/// Returns the full ID if exactly one candidate starts with the prefix
fn resolve_id(prefix: &str, candidates: &[&str]) -> Result<String, IdError>

/// Compute the minimum unique prefix length for each ID in a set
/// Returns Vec<(full_id, min_prefix_len)>
fn min_unique_prefixes(ids: &[&str]) -> Vec<(String, usize)>
```

#### SQL Query Constructors

The core constructs SQL query strings and parameter lists, but never executes them. This
keeps query logic testable without a database.

```rust
/// Queries for reading state
fn query_board_by_id() -> &'static str
fn query_columns_by_board() -> &'static str
fn query_tasks_by_board() -> &'static str
fn query_priorities() -> &'static str
fn query_all_boards() -> &'static str

/// Queries for mutations — return (sql, params) tuples
fn insert_board_sql(board: &Board) -> (String, Vec<SqlParam>)
fn insert_column_sql(column: &Column) -> (String, Vec<SqlParam>)
fn insert_task_sql(task: &Task) -> (String, Vec<SqlParam>)
fn update_task_sql(id: &str, changes: &TaskChanges) -> (String, Vec<SqlParam>)
fn update_column_sql(id: &str, changes: &ColumnChanges) -> (String, Vec<SqlParam>)
fn delete_task_sql(id: &str) -> (String, Vec<SqlParam>)
fn delete_column_sql(id: &str) -> (String, Vec<SqlParam>)
fn delete_tasks_by_column_sql(column_id: &str) -> (String, Vec<SqlParam>)
fn move_tasks_to_column_sql(from_column_id: &str, to_column_id: &str) -> (String, Vec<SqlParam>)
fn update_position_sql(table: EntityTable, id: &str, position: i32) -> (String, Vec<SqlParam>)
fn insert_priority_sql(priority: &Priority) -> (String, Vec<SqlParam>)

/// Param type for SQL query constructors
enum SqlParam {
    Text(String),
    Int(i32),
}

/// Which table a position update targets
enum EntityTable {
    Column,
    Task,
}
```

#### Error Types

```rust
#[derive(Debug, thiserror::Error)]
enum DomainError {
    #[error("title cannot be empty")]
    EmptyTitle,

    #[error("title exceeds maximum length of {max} characters")]
    TitleTooLong { max: usize },

    #[error("column name cannot be empty")]
    EmptyColumnName,

    #[error("column '{name}' already exists on this board")]
    DuplicateColumnName { name: String },

    #[error("column not found: '{id}'")]
    ColumnNotFound { id: String },

    #[error("task not found: '{id}'")]
    TaskNotFound { id: String },

    #[error("board not found: '{id}'")]
    BoardNotFound { id: String },

    #[error("priority not found: '{id}'")]
    PriorityNotFound { id: String },

    #[error("cannot delete the last column on a board")]
    CannotDeleteLastColumn,

    #[error("board '{name}' already exists")]
    DuplicateBoardName { name: String },

    #[error("position {position} is out of range (0..{max})")]
    PositionOutOfRange { position: i32, max: i32 },
}

#[derive(Debug, thiserror::Error)]
enum IdError {
    #[error("no match for ID prefix '{prefix}'")]
    NotFound { prefix: String },

    #[error("ambiguous ID prefix '{prefix}' matches: {matches:?}")]
    Ambiguous { prefix: String, matches: Vec<String> },
}
```

### Layer 2: Imperative Shell (`shell/`)

Owns all I/O. Executes operations against SQLite using core types, query constructors,
and validation functions.

The shell exposes an **operations API** — one function per user-facing action. Each
operation follows the same pattern:

1. Load relevant state from SQLite (using core query constructors)
2. Validate inputs using core validation functions
3. Construct mutation queries using core SQL constructors
4. Execute queries within a transaction
5. Return a result type that the UX layer can render

```rust
/// Database handle wrapper
struct Db {
    conn: rusqlite::Connection,
}

impl Db {
    /// Open or create the database, run pending migrations
    fn open(path: &Path) -> Result<Self>

    /// Load a full board snapshot
    fn load_board_state(&self, board_id: &str) -> Result<BoardState>

    /// Load all boards (id + name only, for listing)
    fn list_boards(&self) -> Result<Vec<Board>>
}

/// Shell operations — each corresponds to a user-facing command
/// All operations take &Db (or &mut for writes) and return Result<T>

// -- Board operations --
fn init_board(db: &Db, name: &str) -> Result<Board>
fn list_boards(db: &Db) -> Result<Vec<Board>>
fn rename_board(db: &Db, board_id: &str, new_name: &str) -> Result<()>

// -- Column operations --
fn add_column(db: &Db, board_id: &str, name: &str) -> Result<Column>
fn rename_column(db: &Db, column_id_prefix: &str, new_name: &str) -> Result<()>
fn remove_column(db: &Db, column_id_prefix: &str, orphan_action: OrphanAction) -> Result<()>
fn move_column(db: &Db, column_id_prefix: &str, new_position: i32) -> Result<()>
fn list_columns(db: &Db, board_id: &str) -> Result<Vec<Column>>

// -- Task operations --
fn add_task(db: &Db, column_id_prefix: &str, title: &str, desc: &str, priority_id_prefix: &str) -> Result<Task>
fn edit_task(db: &Db, task_id_prefix: &str, changes: &TaskChanges) -> Result<()>
fn move_task(db: &Db, task_id_prefix: &str, target_column_prefix: &str, position: Option<i32>) -> Result<()>
fn remove_task(db: &Db, task_id_prefix: &str) -> Result<()>
fn show_task(db: &Db, task_id_prefix: &str) -> Result<Task>
fn list_tasks(db: &Db, board_id: &str, filter: &TaskFilter) -> Result<Vec<Task>>

/// What to do with tasks when their column is deleted
enum OrphanAction {
    MoveToFirst,  // Reassign to first column (lowest position)
    Delete,       // Delete along with column
}

/// Filter criteria for task listing
struct TaskFilter {
    column_id: Option<String>,
    priority_id: Option<String>,
}
```

Each shell function that accepts an ID prefix internally calls `resolve_id()` from the
core to expand it to a full ID before proceeding.

### Layer 3: UX (`cli/`, `tui/`)

User-facing frontends. They translate user intent into shell operation calls and render
the results. They handle:

- Argument parsing (CLI) or keypress handling (TUI)
- Interactive prompting for missing required fields (CLI)
- Confirmation dialogs (CLI: y/n prompt; TUI: confirm dialog widget)
- Output formatting (CLI: text tables; TUI: ratatui widgets)
- Error display (CLI: stderr messages; TUI: status bar / error popup)

The UX layer **never** touches SQLite directly. It only calls shell operations.

### Dependency Direction

```
UX (cli/, tui/)
    ↓ calls
Shell (shell/)
    ↓ uses
Core (core/)
```

Core has zero dependencies on shell or UX. Shell depends on core. UX depends on shell
(and transitively on core for types).

---

## 4. Column Deletion Behavior

When a user requests column deletion:

1. Shell checks if the column has any tasks
2. If empty: delete immediately (still confirm in UX layer)
3. If non-empty: UX layer prompts the user to choose:
   - **Move tasks to first column** — tasks are reassigned to the column with `position = 0`
     (typically Backlog), appended at the end
   - **Delete tasks** — all tasks in the column are deleted, then the column is deleted
4. After deletion, remaining columns have their positions recomputed (gap-free)

The shell exposes this as `remove_column(db, id, OrphanAction)`. The UX layer is
responsible for determining the `OrphanAction` — via interactive prompt (CLI) or
dialog (TUI).

If the column being deleted IS the first column, tasks are moved to the new first column
(the column that will have position 0 after recomputation).

Deleting the last column on a board is an error (`DomainError::CannotDeleteLastColumn`).

---

## 5. Command Interface

This is the shared vocabulary. Every command maps to a shell operation.

### Initialization

| Command        | Shell operation         | Description                                |
|----------------|-------------------------|--------------------------------------------|
| `kanban init`  | `init_board(name)`      | Create a new board with default columns and priorities. Auto-runs on first use if no board exists. |

### Board Commands

| Command           | Shell operation              | Description                    |
|-------------------|------------------------------|--------------------------------|
| `board list`      | `list_boards()`              | List all boards by name. Aliased to `kanban ls`. |
| `board rename`    | `rename_board(id, name)`     | Rename the current board.      |
| `board show`      | `load_board_state(id)`       | Print the board as a text table (CLI) or render it (TUI). |

### Column Commands

| Command                          | Shell operation                           | Description                    |
|----------------------------------|-------------------------------------------|--------------------------------|
| `column add <name>`              | `add_column(board_id, name)`              | Add a new column at the end.   |
| `column rename <id> <new-name>`  | `rename_column(id, new_name)`             | Rename a column.               |
| `column remove <id>`             | `remove_column(id, orphan_action)`        | Delete a column (prompts for task handling). |
| `column move <id> <position>`    | `move_column(id, position)`               | Move column to a new position. |
| `column list`                    | `list_columns(board_id)`                  | List all columns in order.     |

### Task Commands

| Command                                                            | Shell operation                                     | Description                    |
|--------------------------------------------------------------------|-----------------------------------------------------|--------------------------------|
| `task add <title> --column <col> [--desc <d>] [--priority <p>]`    | `add_task(col, title, desc, priority)`               | Create a task. Prompts for missing required args. |
| `task edit <id> [--title <t>] [--desc <d>] [--priority <p>]`       | `edit_task(id, changes)`                             | Edit task fields.              |
| `task move <id> <column> [--position <pos>]`                       | `move_task(id, column, position)`                    | Move task to another column.   |
| `task remove <id>`                                                 | `remove_task(id)`                                    | Delete a task.                 |
| `task show <id>`                                                   | `show_task(id)`                                      | Show task details.             |
| `task list [--column <col>] [--priority <p>]`                      | `list_tasks(board_id, filter)`                       | List tasks, optionally filtered. |

### ID Resolution in Commands

All `<id>` arguments accept full IDs or short prefixes. The shell resolves them via
`core::resolve_id()`. If ambiguous, the error message lists matching candidates with
their full IDs and titles for disambiguation.

### Column Name Resolution

Columns can also be referenced by name (case-insensitive) in addition to ID prefix.
Shell tries ID resolution first, falls back to name matching. Duplicate column names are
disallowed, so name matching is always unambiguous.

---

## 6. CLI Design

### Argument Parsing

Use `clap` with derive macros.

```
kanban [--db <path>] [-y | --yes] <subcommand>

Subcommands:
  init    Initialize a new board
  ls      List boards (alias for `board list`)
  board   Board operations (list, show, rename)
  column  Column operations (add, rename, remove, move, list)
  task    Task operations (add, edit, move, remove, show, list)
```

### Interactive Prompting

The CLI supports **mixed input modes**. Required arguments can be provided as flags OR
entered interactively when omitted. Optional arguments are never prompted for.

Example flow for `kanban task add`:

```
$ kanban task add "Fix login bug" --column backlog
Description: Investigate the OAuth redirect loop on mobile browsers
Priority [low/medium/high] (medium):
> high

About to create task:
  Title:       Fix login bug
  Column:      Backlog
  Description: Investigate the OAuth redirect loop on mobile browsers
  Priority:    high

Proceed? [Y/n] y
Created task a3x9k2 in Backlog.
```

Rules:
- If a required argument is missing, prompt for it interactively
- Optional arguments (`--desc`, `--priority`) are NOT prompted — they use defaults
- After all inputs are gathered, show a confirmation summary
- `--yes` / `-y` flag skips the confirmation prompt (for scripting)
- In non-interactive terminals (piped stdin), missing required args are an error

### Output Format

Default: human-readable text tables.

### First Run

If the database doesn't exist when any command is run:
1. Create the database file and parent directories
2. Run migrations
3. Run `init_board("Personal")` to create the default board

This means `kanban task add "My first task"` works immediately with zero setup.

### Alias

The Nix flake installs both `kanban` and `pk` binaries. `pk` is a symlink to `kanban`.
Behavior is identical regardless of `argv[0]`.

`kanban ls` is a built-in alias for `kanban board list`.

---

## 7. TUI Design

### Library: ratatui + crossterm

`ratatui` for rendering, `crossterm` for terminal input/event handling.

### Entry Point

`kanban tui` launches the TUI. If invoked as just `kanban` with no subcommand,
the TUI is the default action.

### Layout

```
┌─ Personal Kanban ─────────────────────────────────────────────────┐
│ Backlog (2)     │ Todo (3)        │ Doing (1)       │ Done (4)    │
│─────────────────│─────────────────│─────────────────│─────────────│
│ ┌─────────────┐ │ ┌─────────────┐ │ ┌─────────────┐ │ ┌─────────┐│
│ │ a3x Fix log │ │ │ m8r Write d │ │ │ qW2 API ref │ │ │ p9k Set ││
│ │ [high]      │ │ │ [medium]    │ │ │ [high]      │ │ │ [low]   ││
│ └─────────────┘ │ └─────────────┘ │ └─────────────┘ │ └─────────┘│
│ ┌─────────────┐ │ ┌─────────────┐ │                 │ ┌─────────┐│
│ │ kR7 Update  │ │ │ bQ3 Review  │ │                 │ │ nL5 Add ││
│ │ [low]       │ │ │ [medium]    │ │                 │ │ [medium]││
│ └─────────────┘ │ └─────────────┘ │                 │ └─────────┘│
├───────────────────────────────────────────────────────────────────┤
│ [h/l] column  [j/k] task  [a]dd  [e]dit  [m]ove  [d]elete  [?]  │
└───────────────────────────────────────────────────────────────────┘
```

- Each task card shows its short ID (minimum-unique prefix in **bold**, rest dimmed)
- Columns are equal-width vertical panes, horizontally scrollable if they exceed
  terminal width
- Currently selected task is highlighted
- Status bar at bottom shows keybindings for current mode

### ID Display in TUI

Task IDs are rendered with the jj-style treatment:
- Compute `min_unique_prefixes()` across all visible task IDs
- Render the unique prefix portion in bold/bright
- Render the remaining characters in dim/grey
- Example: if `a3x` is the minimum unique prefix of `a3x9k2`, display as **a3x**9k2

### Event Loop

The TUI uses a synchronous tick-based event loop:

```
loop {
    terminal.draw(|frame| render(frame, &app_state))?;
    if let Event::Key(key) = crossterm::event::read()? {
        let action = handle_input(key, &app_state.mode);
        match action {
            Action::Quit => break,
            Action::ShellOp(op) => {
                execute_shell_op(&db, op)?;
                app_state.board = db.load_board_state(board_id)?;
            }
            Action::ModeChange(mode) => app_state.mode = mode,
            Action::Navigate(nav) => app_state.apply_navigation(nav),
            Action::None => {}
        }
    }
}
```

After any mutation (ShellOp), the board state is reloaded from SQLite. No in-memory
caching of board state across mutations — always read fresh. At this scale, SQLite
reads are sub-millisecond.

### Modes

| Mode     | Description                                            |
|----------|--------------------------------------------------------|
| Normal   | Navigate between columns and tasks. Default mode.      |
| Insert   | Text input for creating new tasks/columns.             |
| Edit     | Editing a task's fields (title, description, priority). |
| Column   | Column operations (add, rename, delete, reorder).      |
| Confirm  | Yes/no or choice confirmation for destructive actions.  |
| Help     | Help overlay showing all keybindings.                  |

### Keybindings — Normal Mode

| Key        | Action                                        |
|------------|-----------------------------------------------|
| `h` / `l`  | Move focus to previous / next column          |
| `j` / `k`  | Move focus to next / previous task in column  |
| `a`        | Add a new task to the focused column          |
| `e`        | Edit the focused task                         |
| `m`        | Move the focused task (enter move mode)       |
| `d`        | Delete the focused task (with confirmation)   |
| `H` / `L`  | Move focused task to previous / next column   |
| `J` / `K`  | Move focused task down / up within column     |
| `C`        | Enter column command mode                     |
| `?`        | Toggle help overlay                           |
| `q`        | Quit                                          |

### Keybindings — Move Mode (after `m`)

| Key     | Action                                          |
|---------|-------------------------------------------------|
| `h`/`l` | Highlight destination column                    |
| `Enter` | Confirm move to highlighted column              |
| `Esc`   | Cancel                                          |

### Keybindings — Column Mode (after `C`)

| Key     | Action                        |
|---------|-------------------------------|
| `a`     | Add new column                |
| `r`     | Rename focused column         |
| `d`     | Delete focused column         |
| `h`/`l` | Reorder column (move left/right) |
| `Esc`   | Exit column mode              |

### Keybindings — Insert / Edit Mode

| Key     | Action                       |
|---------|------------------------------|
| `Enter` | Confirm / save               |
| `Esc`   | Cancel                       |
| `Tab`   | Cycle between fields (edit)  |
| Standard text input keys for editing |

### Task Edit View

When editing a task, render a centered overlay/popup with fields:
- Title (text input)
- Description (text input, multi-line)
- Priority (cycle through available priorities with Tab or arrow keys)

### Confirm Dialog

For destructive actions (delete task, delete column), render a dialog:
- Show what will be affected
- For column deletion with tasks: present "Move to [first column]" / "Delete all" / "Cancel"
- For task deletion: "Delete [task title]? [y/N]"

---

## 8. Error Handling

### Error Flow

Core functions return `DomainError` or `IdError`. Shell functions wrap these (and I/O
errors) in `anyhow::Error`. The UX layer decides how to present them.

### CLI Error Display

Errors are printed to stderr with a prefix:

```
$ kanban task move a3 Done
error: ambiguous ID prefix 'a3' matches: a3x9k2 (Fix login bug), a3bQ7f (Update deps)

$ kanban column add "Todo"
error: column 'Todo' already exists on this board

$ kanban task add ""
error: title cannot be empty

$ kanban column remove backlog
error: cannot delete the last column on a board
```

Exit codes:
- `0` — success
- `1` — domain error (bad input, not found, etc.)
- `2` — I/O error (database, filesystem)

### TUI Error Display

Errors are shown in a status bar at the bottom of the screen, in red, for 3 seconds
(or until the next keypress). For destructive operation errors, use a dialog popup
instead of the status bar.

### Error Catalog

| Error                    | When                                              | CLI message                                                       |
|--------------------------|---------------------------------------------------|-------------------------------------------------------------------|
| `EmptyTitle`             | Creating/editing task with empty title             | `error: title cannot be empty`                                    |
| `TitleTooLong`           | Title exceeds max length                           | `error: title exceeds maximum length of {max} characters`         |
| `EmptyColumnName`        | Creating/editing column with empty name            | `error: column name cannot be empty`                              |
| `DuplicateColumnName`    | Column name already exists on board                | `error: column '{name}' already exists on this board`             |
| `ColumnNotFound`         | ID/name doesn't match any column                   | `error: column not found: '{id}'`                                 |
| `TaskNotFound`           | ID doesn't match any task                          | `error: task not found: '{id}'`                                   |
| `BoardNotFound`          | Board ID doesn't match                             | `error: board not found: '{id}'`                                  |
| `PriorityNotFound`       | Priority ID/name doesn't match                     | `error: priority not found: '{id}'`                               |
| `CannotDeleteLastColumn` | Trying to delete the only column                   | `error: cannot delete the last column on a board`                 |
| `DuplicateBoardName`     | Board name already exists                          | `error: board '{name}' already exists`                            |
| `PositionOutOfRange`     | Position arg exceeds valid range                   | `error: position {pos} is out of range (0..{max})`               |
| `IdNotFound`             | No ID matches prefix                               | `error: no match for ID prefix '{prefix}'`                        |
| `IdAmbiguous`            | Multiple IDs match prefix                          | `error: ambiguous ID prefix '{prefix}' matches: {id1} ({title1}), {id2} ({title2})` |
| `DatabaseError`          | SQLite error                                       | `error: database error: {details}`                                |
| `MigrationError`         | Migration failed                                   | `error: migration failed: {details}`                              |

---

## 9. Module / Crate Structure

Single crate, multiple modules.

```
personal-kanban/
├── Cargo.toml
├── flake.nix
├── flake.lock
├── SPEC.md
├── migrations/
│   └── 001_initial_schema.sql
└── src/
    ├── main.rs              # Entry point: arg dispatch, first-run init
    ├── core/
    │   ├── mod.rs           # Re-exports
    │   ├── types.rs         # Board, Column, Task, Priority, BoardState,
    │   │                    #   TaskChanges, ColumnChanges, TaskFilter,
    │   │                    #   OrphanAction, SqlParam, EntityTable
    │   ├── validation.rs    # validate_title, validate_column_name, etc.
    │   ├── position.rs      # next_position, recompute_positions, etc.
    │   ├── id.rs            # resolve_id, min_unique_prefixes
    │   ├── queries.rs       # SQL query constructors
    │   └── error.rs         # DomainError, IdError
    ├── shell/
    │   ├── mod.rs           # Re-exports
    │   ├── db.rs            # Db struct, open, migrations, load_board_state
    │   ├── ops.rs           # Shell operations (add_task, move_task, etc.)
    │   └── config.rs        # DB path resolution: CLI flag > env var > XDG
    ├── cli/
    │   ├── mod.rs           # Clap definitions, subcommand dispatch
    │   ├── interactive.rs   # Interactive prompting for missing args
    │   └── output.rs        # Text table formatting, error display
    └── tui/
        ├── mod.rs           # TUI entry point, event loop
        ├── app.rs           # AppState: focused column, selected task, mode
        ├── input.rs         # Keypress → Action mapping per mode
        ├── render.rs        # Top-level frame rendering
        └── widgets/
            ├── mod.rs
            ├── board.rs     # Board widget (column layout)
            ├── card.rs      # Task card widget (with short-ID rendering)
            ├── dialog.rs    # Confirmation / edit / choice dialogs
            └── help.rs      # Help overlay
```

### Why a single crate?

At this scale, a workspace adds ceremony without benefit. The module boundary enforces
the architectural constraint: `core/` has no `use` of anything from `shell/`, `cli/`, or
`tui/`. If the project grows (e.g., adding a REST API crate), extract `core/` into a
workspace member then.

---

## 10. Nix Flake

### What the flake provides

- A default package: the `kanban` binary (and `pk` symlink)
- A dev shell with Rust toolchain, SQLite headers, and development tools

### flake.nix outline

```nix
{
  description = "Personal kanban board — CLI and TUI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        craneLib = crane.mkLib pkgs;

        kanban = craneLib.buildPackage {
          src = craneLib.cleanCargoSource ./.;
          buildInputs = [ ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.libiconv
          ];
          postInstall = ''
            ln -s $out/bin/kanban $out/bin/pk
          '';
        };
      in {
        packages.default = kanban;
        apps.default = flake-utils.lib.mkApp { drv = kanban; };

        devShells.default = craneLib.devShell {
          packages = with pkgs; [
            rust-analyzer
            cargo-watch
          ];
        };
      }
    );
}
```

---

## 11. Dependencies

| Crate        | Purpose                                | Notes                                           |
|--------------|----------------------------------------|-------------------------------------------------|
| `clap`       | CLI argument parsing                   | Derive macros, subcommand support               |
| `ratatui`    | TUI rendering                          | Successor to `tui-rs`, actively maintained      |
| `crossterm`  | Terminal backend for ratatui           | Cross-platform, pure Rust                       |
| `rusqlite`   | SQLite bindings                        | `bundled` feature compiles SQLite in             |
| `nanoid`     | ID generation                          | Short, URL-safe random IDs                      |
| `serde`      | Serialization                          | For types that need serialization                |
| `chrono`     | Timestamp handling                     | DateTime types                                  |
| `dirs`       | XDG directory resolution               | Cross-platform                                  |
| `anyhow`     | Error handling in shell/UX layers      | Ergonomic error chaining for I/O code           |
| `thiserror`  | Error handling in core layer           | Derive macro for domain error enums             |
| `dialoguer`  | Interactive CLI prompts                | Input, confirm, select prompts                  |

---

## 12. Explicitly Out of Scope (MVP)

| Feature                  | Why excluded                                               |
|--------------------------|------------------------------------------------------------|
| REST API                 | Future UX layer — same shell operations, different I/O     |
| Multi-machine sync       | Requires network protocol design; SQLite file is portable  |
| Backup to home lab       | Simple file copy works today; automated backup is later    |
| Multiple boards (UI)     | Core models boards as instances; multi-board UX deferred   |
| Epics / parent tasks     | Personal kanban doesn't need hierarchy                     |
| Calendar view            | Not needed                                                 |
| Sprints                  | Not needed                                                 |
| Due dates                | Could be added as a task field later                       |
| Assignees                | Single user tool                                           |
| Comments on tasks        | Description field suffices                                 |
| Undo/redo                | Could be added via command history later                   |
| Task search / filter     | Stretch goal for TUI, not MVP                              |
| Custom themes            | ratatui supports it; not worth the config surface now      |
| `--json` output          | Can be a fast follow                                       |
| Custom priorities        | Table exists, but MVP only seeds low/medium/high           |

---

## 13. Implementation Order

Suggested build sequence for a coding agent:

1. **Project scaffolding** — `Cargo.toml`, `flake.nix`, directory structure, empty modules
2. **Core types** — `Board`, `Column`, `Task`, `Priority`, `BoardState`, `TaskChanges`,
   `ColumnChanges`, `DomainError`, `IdError`
3. **Core functions** — validation, position computation, ID resolution, SQL query
   constructors. Unit tests for all.
4. **Migrations** — `001_initial_schema.sql`, migration runner in `shell/db.rs`
5. **Shell operations** — `Db` struct, `load_board_state`, all mutation operations
6. **CLI** — clap definitions, interactive prompting (`dialoguer`), output formatting,
   first-run auto-init
7. **Integration tests** — CLI commands against a temp database
8. **TUI: rendering** — board layout, column panes, task cards with short-ID display
9. **TUI: navigation** — Normal mode (hjkl), mode transitions
10. **TUI: mutations** — Add/edit/move/delete via dialogs, column operations
11. **TUI: polish** — Help overlay, confirmation dialogs, error display, status bar
12. **Nix flake** — Finalize, test `nix build`, verify `pk` symlink
