# Spec: personal-kanban

## Context

A personal kanban board tool with a CLI and TUI interface, built in Rust. The binary is
named `kanban`, with a `pk` alias installed alongside it.

The architecture follows a strict three-layer pattern, and the checkpoints below are
organized along those layers:

1. **Core** — Pure types, enums, parsing logic, SQL query constructors, validation. No side
   effects, no I/O.
2. **Imperative Shell** — Executes operations against SQLite using core types and queries.
   Owns all I/O: database reads/writes, filesystem operations, interactive prompts.
3. **UX Layer** — User-facing frontends (CLI and TUI) that translate user intent into shell
   operations and render results. Future: REST API.

Both the CLI and TUI invoke the same shell operations. Every action available in the TUI is
a CLI subcommand and vice versa.

**Dependency Direction**

```
UX (cli/, tui/)
    ↓ calls
Shell (shell/)
    ↓ uses
Core (core/)
```

Core has zero dependencies on shell or UX. Shell depends on core. UX depends on shell (and
transitively on core for types). The module boundary enforces this: `core/` has no `use` of
anything from `shell/`, `cli/`, or `tui/`.

**Data Model**

**Board** — a self-contained unit. MVP ships with a single default board created on first
run. The core models a board as an isolated instance so multi-board support later is just
instantiation + UX glue.

| Field        | Type       | Notes                     |
|--------------|------------|---------------------------|
| `id`         | `TEXT PK`  | nanoid                    |
| `name`       | `TEXT`     | Human-readable board name |
| `created_at` | `DATETIME` | Auto-set on creation      |
| `updated_at` | `DATETIME` | Auto-set on mutation      |

**Column** — ordered containers for tasks within a board.

| Field        | Type      | Notes                        |
|--------------|-----------|------------------------------|
| `id`         | `TEXT PK` | nanoid                       |
| `board_id`   | `TEXT FK` | References `board.id`        |
| `name`       | `TEXT`    | Display name (e.g. "Doing")  |
| `position`   | `INTEGER` | Explicit ordering, 0-indexed |
| `created_at` | `DATETIME`| Auto-set                     |
| `updated_at` | `DATETIME`| Auto-set                     |

**Priority** — a first-class entity stored in its own table, not a hardcoded enum. Allows
users to define custom priority levels in future versions.

| Field  | Type      | Notes                      |
|--------|-----------|----------------------------|
| `id`   | `TEXT PK` | nanoid                     |
| `name` | `TEXT`    | Display name (e.g. "high") |

**Task**

| Field         | Type      | Notes                             |
|---------------|-----------|-----------------------------------|
| `id`          | `TEXT PK` | nanoid                            |
| `column_id`   | `TEXT FK` | References `column_.id`           |
| `title`       | `TEXT`    | Required, non-empty               |
| `description` | `TEXT`    | Optional, can be empty            |
| `priority_id` | `TEXT FK` | References `priority.id`          |
| `position`    | `INTEGER` | Ordering within column, 0-indexed |
| `created_at`  | `DATETIME`| Auto-set                          |
| `updated_at`  | `DATETIME`| Auto-set                          |

**Default board state:** four columns — Backlog (0), Todo (1), Doing (2), Done (3) — and
three priorities: `low`, `medium`, `high`. Default priority for new tasks: `medium`.

**ID Generation and Short-ID Resolution**

All entity IDs use nanoid (short random strings, e.g. `V1StGXR8_Z5jdHi6B`).

**Short-ID matching:** Users can reference any entity by a prefix of its ID — only enough
characters to be unique within the selectable set for that operation. For example, given
task IDs `a3x9k2`, `a3bQ7f`, and `m8rT2p`:

- `m` uniquely matches `m8rT2p`
- `a3x` uniquely matches `a3x9k2`
- `a3` is ambiguous → error listing the matching candidates

**TUI display:** task IDs render with the minimum-unique prefix in bold (or bright color)
and the remaining characters dimmed (jj-style). The minimum-unique length is computed per
render frame against the visible set.

**Storage**

Single-file SQLite via `rusqlite` with the `bundled` feature (SQLite amalgamation compiled
into the binary — no external C dependency at runtime).

Default location: `$XDG_DATA_HOME/kanban/kanban.db` (typically
`~/.local/share/kanban/kanban.db`). Overridable via `--db <path>` (CLI flag) or `KANBAN_DB`
(env var). Priority: CLI flag > env var > XDG default.

Schema:

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

Notes: `column_` has a trailing underscore because `column` is a SQL reserved word.
`task.column_id` deliberately does NOT use `ON DELETE CASCADE` — column deletion requires an
explicit user decision about orphaned tasks.

Migrations are numbered SQL files (`NNN_description.sql`) under `migrations/`, embedded into
the binary at compile time (via `include_str!` or `rust-embed`). On startup the shell creates
`_migrations` if absent, reads applied migration ids, runs any unapplied migrations in order
within a transaction, and records each one.

This database serves a single user's personal kanban; read/write volume is negligible. The
schema prioritizes clarity and correctness over optimization.

**Command Interface**

Every command maps to a shell operation; every shell operation is reachable from both CLI
and TUI.

| Command                                                         | Shell operation                        |
|-----------------------------------------------------------------|----------------------------------------|
| `kanban init`                                                   | `init_board(name)`                     |
| `board list` (alias `kanban ls`)                                | `list_boards()`                        |
| `board rename`                                                  | `rename_board(id, name)`               |
| `board show`                                                    | `load_board_state(id)`                 |
| `column add <name>`                                             | `add_column(board_id, name)`           |
| `column rename <id> <new-name>`                                 | `rename_column(id, new_name)`          |
| `column remove <id>`                                            | `remove_column(id, orphan_action)`     |
| `column move <id> <position>`                                   | `move_column(id, position)`            |
| `column list`                                                   | `list_columns(board_id)`               |
| `task add <title> --column <col> [--desc <d>] [--priority <p>]` | `add_task(col, title, desc, priority)` |
| `task edit <id> [--title <t>] [--desc <d>] [--priority <p>]`    | `edit_task(id, changes)`               |
| `task move <id> <column> [--position <pos>]`                    | `move_task(id, column, position)`      |
| `task remove <id>`                                              | `remove_task(id)`                      |
| `task show <id>`                                                | `show_task(id)`                        |
| `task list [--column <col>] [--priority <p>]`                   | `list_tasks(board_id, filter)`         |

All `<id>` arguments accept full IDs or short prefixes, resolved via `core::resolve_id()`.
Columns may also be referenced by name (case-insensitive); the shell tries ID resolution
first, then falls back to name matching (duplicate column names are disallowed, so name
matching is unambiguous).

**Out of Scope (MVP)**

REST API, multi-machine sync, automated backup, multi-board UI, epics/parent tasks, calendar
view, sprints, due dates, assignees, comments, undo/redo, task search/filter, custom themes,
`--json` output, and user-defined custom priorities (the table exists but MVP seeds only
low/medium/high).

**Dependencies**

`clap`, `ratatui`, `crossterm`, `rusqlite` (bundled), `nanoid`, `serde`, `chrono`, `dirs`,
`anyhow`, `thiserror`, `dialoguer`. These are already declared in `Cargo.toml`.

**Module Structure**

Single crate, multiple modules:

```
src/
├── main.rs              # Entry point: arg dispatch, first-run init
├── core/  { mod, types, validation, position, id, queries, error }
├── shell/ { mod, db, ops, config }
├── cli/   { mod, interactive, output }
└── tui/   { mod, app, input, render, widgets/{mod,board,card,dialog,help} }
```

A workspace is deferred; if the project grows (e.g. a REST API crate), extract `core/` into
a workspace member then.

## Checkpoint: core

Goal: Implement the pure core library (types, validation, position math, ID resolution, SQL query constructors) with full unit-test coverage and no I/O.

### Requirements

The core (`src/core/`) is pure library code: no I/O, no side effects, no `std::fs`, no
database access. It must not `use` anything from `shell/`, `cli/`, or `tui/`.

**Types** (`types.rs`): `Board`, `Column`, `Priority`, `Task` as described in the Context
data model, plus:

```rust
struct BoardState {
    board: Board,
    columns: Vec<Column>,       // sorted by position
    tasks: Vec<Task>,           // sorted by column_id, then position
    priorities: Vec<Priority>,
}

struct TaskChanges {
    title: Option<String>,
    description: Option<String>,
    priority_id: Option<String>,
    column_id: Option<String>,
    position: Option<i32>,
}

struct ColumnChanges { name: Option<String>, position: Option<i32> }

struct TaskFilter { column_id: Option<String>, priority_id: Option<String> }

enum OrphanAction { MoveToFirst, Delete }

enum SqlParam { Text(String), Int(i32) }

enum EntityTable { Column, Task }
```

**Validation** (`validation.rs`):

```rust
fn validate_title(title: &str) -> Result<(), DomainError>
fn validate_column_name(name: &str, existing: &[Column]) -> Result<(), DomainError>
fn validate_priority(priority_id: &str, priorities: &[Priority]) -> Result<(), DomainError>
fn validate_column_exists(column_id: &str, columns: &[Column]) -> Result<(), DomainError>
```

**Position computation** (`position.rs`):

```rust
fn next_position(existing_positions: &[i32]) -> i32
fn recompute_positions(items: &[(String, i32)]) -> Vec<(String, i32)>
fn positions_after_insert(existing: &[(String, i32)], insert_at: i32) -> Vec<(String, i32)>
fn positions_after_move(existing: &[(String, i32)], from: i32, to: i32) -> Vec<(String, i32)>
```

`recompute_positions` must yield gap-free 0,1,2,… ordering after deletes/moves.

**ID resolution** (`id.rs`):

```rust
fn resolve_id(prefix: &str, candidates: &[&str]) -> Result<String, IdError>
fn min_unique_prefixes(ids: &[&str]) -> Vec<(String, usize)>
```

`resolve_id` returns the full ID on exactly one match, `IdError::Ambiguous { prefix, matches }`
on multiple, `IdError::NotFound { prefix }` on none. `min_unique_prefixes` returns each ID
paired with the minimum prefix length needed for uniqueness within the set.

**SQL query constructors** (`queries.rs`) — construct SQL strings and param lists but never
execute:

```rust
fn query_board_by_id() -> &'static str
fn query_columns_by_board() -> &'static str
fn query_tasks_by_board() -> &'static str
fn query_priorities() -> &'static str
fn query_all_boards() -> &'static str

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
```

**Error types** (`error.rs`): `DomainError` (thiserror) with variants `EmptyTitle`,
`TitleTooLong { max }`, `EmptyColumnName`, `DuplicateColumnName { name }`,
`ColumnNotFound { id }`, `TaskNotFound { id }`, `BoardNotFound { id }`,
`PriorityNotFound { id }`, `CannotDeleteLastColumn`, `DuplicateBoardName { name }`,
`PositionOutOfRange { position, max }`; and `IdError` with `NotFound { prefix }` and
`Ambiguous { prefix, matches }`. Error message strings match the Error Catalog in the Notes.

Every core function has unit tests, including the short-ID examples above
(`m`→`m8rT2p`, `a3x`→`a3x9k2`, `a3`→ambiguous).

### Validation

```validation
cargo test core:: -- --nocapture
cargo clippy -- -D warnings
```

### Notes

Error catalog (message strings the tests assert on):

| Error                    | CLI message                                                       |
|--------------------------|------------------------------------------------------------------|
| `EmptyTitle`             | `title cannot be empty`                                           |
| `TitleTooLong`           | `title exceeds maximum length of {max} characters`               |
| `EmptyColumnName`        | `column name cannot be empty`                                     |
| `DuplicateColumnName`    | `column '{name}' already exists on this board`                    |
| `ColumnNotFound`         | `column not found: '{id}'`                                        |
| `TaskNotFound`           | `task not found: '{id}'`                                          |
| `BoardNotFound`          | `board not found: '{id}'`                                         |
| `PriorityNotFound`       | `priority not found: '{id}'`                                      |
| `CannotDeleteLastColumn` | `cannot delete the last column on a board`                        |
| `DuplicateBoardName`     | `board '{name}' already exists`                                   |
| `PositionOutOfRange`     | `position {position} is out of range (0..{max})`                 |
| `IdError::NotFound`      | `no match for ID prefix '{prefix}'`                              |
| `IdError::Ambiguous`     | `ambiguous ID prefix '{prefix}' matches: {matches}`             |

## Checkpoint: shell

Goal: Implement the imperative shell (SQLite-backed operations API and migration runner) so every user-facing action has a working, transactional shell function.

### Requirements

The shell (`src/shell/`) owns all I/O. Each operation: (1) loads relevant state via core
query constructors, (2) validates via core validation functions, (3) constructs mutation
queries via core SQL constructors, (4) executes within a transaction, (5) returns a
render-ready result type. The shell never contains business rules the core should own; it
orchestrates.

**DB path resolution** (`config.rs`): CLI flag > `KANBAN_DB` env var > XDG default
(`$XDG_DATA_HOME/kanban/kanban.db`).

**Db handle and migrations** (`db.rs`):

```rust
struct Db { conn: rusqlite::Connection }

impl Db {
    fn open(path: &Path) -> Result<Self>            // creates file+dirs, runs migrations
    fn load_board_state(&self, board_id: &str) -> Result<BoardState>
    fn list_boards(&self) -> Result<Vec<Board>>
}
```

`open` must create parent directories and the DB file if missing, then run all pending
migrations in order inside a transaction, recording each in `_migrations`. Migration `001`
lives at `migrations/001_initial_schema.sql` and is embedded at compile time.

**Operations** (`ops.rs`) — each resolves ID/prefix args via `core::resolve_id()` before
proceeding:

```rust
fn init_board(db: &Db, name: &str) -> Result<Board>
fn list_boards(db: &Db) -> Result<Vec<Board>>
fn rename_board(db: &Db, board_id: &str, new_name: &str) -> Result<()>

fn add_column(db: &Db, board_id: &str, name: &str) -> Result<Column>
fn rename_column(db: &Db, column_id_prefix: &str, new_name: &str) -> Result<()>
fn remove_column(db: &Db, column_id_prefix: &str, orphan_action: OrphanAction) -> Result<()>
fn move_column(db: &Db, column_id_prefix: &str, new_position: i32) -> Result<()>
fn list_columns(db: &Db, board_id: &str) -> Result<Vec<Column>>

fn add_task(db: &Db, column_id_prefix: &str, title: &str, desc: &str, priority_id_prefix: &str) -> Result<Task>
fn edit_task(db: &Db, task_id_prefix: &str, changes: &TaskChanges) -> Result<()>
fn move_task(db: &Db, task_id_prefix: &str, target_column_prefix: &str, position: Option<i32>) -> Result<()>
fn remove_task(db: &Db, task_id_prefix: &str) -> Result<()>
fn show_task(db: &Db, task_id_prefix: &str) -> Result<Task>
fn list_tasks(db: &Db, board_id: &str, filter: &TaskFilter) -> Result<Vec<Task>>
```

`init_board` seeds the four default columns (Backlog/Todo/Doing/Done) and three priorities
(low/medium/high).

**Column deletion behavior** (`remove_column`): if the column is empty, delete it. If it has
tasks, the caller supplies an `OrphanAction`: `MoveToFirst` reassigns tasks to the column
that will have position 0 after recomputation (appended at its end), `Delete` deletes the
tasks then the column. After deletion, remaining column positions are recomputed gap-free.
Deleting the last column returns `DomainError::CannotDeleteLastColumn`.

Shell functions wrap `DomainError`/`IdError` and I/O errors in `anyhow::Error`.

Integration tests exercise operations against a temp database: init seeds defaults; add/edit/
move/remove round-trips; short-ID resolution against real rows; the three `remove_column`
paths (empty, MoveToFirst, Delete, last-column error).

### Validation

```validation
cargo test shell:: -- --nocapture
cargo test --test '*' -- --nocapture
cargo clippy -- -D warnings
```

### Notes

Migrations are plain SQL, one file per numbered change; the runner is idempotent (re-running
`open` on an up-to-date DB applies nothing). At this scale SQLite reads are sub-millisecond,
so no in-memory caching is warranted.

## Checkpoint: cli

Goal: Implement the clap-based CLI so every command runs end-to-end against a real database and produces the specified output, with first-run auto-init and interactive prompting for missing required args.

### Requirements

The CLI (`src/cli/`) parses arguments (clap derive), prompts for missing required fields
(`dialoguer`), formats output, and displays errors. It calls only shell operations — never
SQLite directly.

**Top-level:** `kanban [--db <path>] [-y | --yes] <subcommand>` with subcommands `init`,
`ls` (alias for `board list`), `board`, `column`, `task`, and `tui`. Invoked with no
subcommand, the default action is the TUI (implemented in the `tui` checkpoint; until then a
stub is acceptable but the arg wiring must exist). The `pk`/`kanban` distinction is purely
`argv[0]`; behavior is identical.

**First run:** if the database doesn't exist on any command, create the file + parent dirs,
run migrations, and run `init_board("Personal")`, so `kanban task add "My first task"` works
with zero setup.

**Interactive prompting (mixed input modes):** required args may be given as flags OR entered
interactively when omitted; optional args (`--desc`, `--priority`) are never prompted and use
defaults. After gathering inputs, show a confirmation summary; `--yes`/`-y` skips it. In a
non-interactive terminal (piped stdin), a missing required arg is an error rather than a
hang.

**Output** (`output.rs`): human-readable text tables by default. Errors print to stderr with
an `error: ` prefix. Exit codes: `0` success, `1` domain error, `2` I/O error.

### Validation

```validation
# Exercise the real binary against a throwaway DB; every imperative touchpoint is run and its output inspected.
cargo build
export KANBAN_DB="$(mktemp -d)/kanban.db"

# First-run auto-init + add (non-interactive: all required args as flags, -y to skip confirm)
cargo run -- task add "Fix login bug" --column backlog --priority high -y | tee /tmp/pk_add.out
grep -q "Fix login bug" /tmp/pk_add.out || { echo "FAIL: add did not confirm task"; exit 1; }

# board show reflects the task
cargo run -- board show | tee /tmp/pk_show.out
grep -q "Fix login bug" /tmp/pk_show.out || { echo "FAIL: task not on board"; exit 1; }

# column list shows the four seeded columns
cargo run -- column list | tee /tmp/pk_cols.out
for c in Backlog Todo Doing Done; do
  grep -qi "$c" /tmp/pk_cols.out || { echo "FAIL: missing column $c"; exit 1; }
done

# task list is filterable and shows the task
cargo run -- task list --column backlog | grep -q "Fix login bug" || { echo "FAIL: task list"; exit 1; }

# error path: empty title -> exit 1, message on stderr
if cargo run -- task add "" --column backlog -y 2>/tmp/pk_err.out; then
  echo "FAIL: empty title should error"; exit 1; fi
grep -q "title cannot be empty" /tmp/pk_err.out || { echo "FAIL: wrong error message"; exit 1; }

# error path: ambiguous / not-found id -> exit 1
if cargo run -- task show zzzzz 2>/dev/null; then echo "FAIL: bad id should error"; exit 1; fi

echo "CLI e2e OK"
```

### Notes

The confirmation summary and interactive prompts are for TTY use; the validation above drives
the non-interactive path (flags + `-y`) precisely because piped stdin must not hang. If a
command's exact stdout wording differs from these greps during implementation, update the
greps to match the real output rather than weakening the observe-the-output requirement.

## Checkpoint: tui

Goal: Implement the ratatui/crossterm TUI so the board renders with short-ID cards and all mode interactions (Normal/Move/Column/Insert/Edit/Confirm/Help) drive the same shell operations as the CLI.

### Requirements

The TUI (`src/tui/`) renders with `ratatui` and handles input with `crossterm`. It calls only
shell operations. `kanban tui` (and bare `kanban`) launches it.

**Layout:** equal-width vertical column panes with a title bar and a bottom status bar showing
keybindings for the current mode; columns are horizontally scrollable if they exceed terminal
width. Each task card shows title, `[priority]`, and its short ID rendered jj-style — the
minimum-unique prefix (from `core::min_unique_prefixes()` over visible IDs) in bold/bright,
the remainder dimmed. The selected task is highlighted.

**Event loop:** synchronous, tick-based — draw, read a key, map it to an `Action`
(`Quit` / `ShellOp` / `ModeChange` / `Navigate` / `None`), execute, and after any mutation
reload board state fresh from SQLite (no cross-mutation in-memory caching).

**Modes:** Normal, Insert, Edit, Column, Confirm, Help.

Normal-mode keys: `h`/`l` prev/next column, `j`/`k` prev/next task, `a` add task to focused
column, `e` edit focused task, `m` enter move mode, `d` delete focused task (confirm),
`H`/`L` move focused task to prev/next column, `J`/`K` move task down/up within column,
`C` enter column mode, `?` toggle help, `q` quit.

Move mode: `h`/`l` highlight destination column, `Enter` confirm, `Esc` cancel.
Column mode: `a` add, `r` rename, `d` delete, `h`/`l` reorder, `Esc` exit.
Insert/Edit: `Enter` save, `Esc` cancel, `Tab` cycle fields (edit), standard text input.

**Widgets** (`widgets/`): `board` (column layout), `card` (task card with short-ID render),
`dialog` (confirm / edit / choice — column deletion offers Move-to-first / Delete-all /
Cancel; task deletion is a y/N confirm), `help` (overlay listing all keybindings).

**Task edit view:** centered overlay with Title (text), Description (multi-line text), and
Priority (cycle available priorities).

**Error display:** status bar at the bottom, in red, for ~3 seconds or until the next
keypress; destructive-operation errors use a dialog popup instead.

Because the TUI needs a live terminal, its automated validation covers the pure pieces
(input→Action mapping and short-ID/card rendering helpers) via unit tests; interactive
behavior is verified manually per the Notes.

**Testability:** factor input handling as a pure `handle_input(key, mode) -> Action` function
and card/short-ID rendering as pure helpers so they can be unit-tested without a terminal.

### Validation

```validation
cargo test tui:: -- --nocapture
cargo clippy -- -D warnings
cargo build
```

### Notes

Manual smoke test (requires a TTY, not run in CI): `KANBAN_DB=$(mktemp -d)/k.db cargo run --
tui`, then verify hjkl navigation, `a` to add a task, `d` + confirm to delete, `C` then `a`
to add a column, `?` for help, `q` to quit — and confirm each mutation persists by reloading.
The `handle_input` and short-ID rendering unit tests are the enforceable gate; the smoke test
is the human check.

## Checkpoint: packaging

Goal: Finalize the Nix flake so `nix build` produces the `kanban` binary with a working `pk` alias, and the dev shell provides the full toolchain.

### Requirements

The flake (`flake.nix`) provides a default package building the `kanban` binary (with a `pk`
symlink) via `crane`, and a dev shell with the Rust toolchain, SQLite headers, and dev tools
(`rust-analyzer`, `cargo-watch`). On Darwin it includes the required build inputs
(`libiconv`, and the Security framework if needed by the toolchain).

`postInstall` creates `pk` as a symlink to `kanban` in the package output. Behavior is
identical regardless of `argv[0]`, and `kanban ls` remains a built-in alias for
`kanban board list`.

### Validation

```validation
# Build via Nix and observe both binaries work end-to-end.
nix build .#default -L
test -x ./result/bin/kanban || { echo "FAIL: kanban binary missing"; exit 1; }
test -L ./result/bin/pk || { echo "FAIL: pk is not a symlink"; exit 1; }

export KANBAN_DB="$(mktemp -d)/kanban.db"
./result/bin/kanban task add "packaged task" --column backlog -y | grep -q "packaged task" \
  || { echo "FAIL: kanban binary did not create task"; exit 1; }
./result/bin/pk board show | grep -q "packaged task" \
  || { echo "FAIL: pk alias did not read the same board"; exit 1; }
echo "packaging OK"
```

### Notes

`nix build` compiles the SQLite amalgamation via the `bundled` rusqlite feature, so no system
SQLite is required at runtime. If `nix` is unavailable in the execution environment, the
equivalent fallback is `cargo build --release` plus a manual `ln -s kanban pk`, but the
flake build is the authoritative gate.
