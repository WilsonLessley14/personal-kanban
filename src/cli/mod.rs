use std::process;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

pub mod interactive;
pub mod output;

use crate::core::{OrphanAction, TaskChanges, TaskFilter};
use crate::shell;

// ── Top-level parser ───────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "kanban", about = "Personal kanban board CLI", version)]
struct Cli {
    /// Path to the database file (overrides KANBAN_DB and XDG default)
    #[arg(global = true, long)]
    db: Option<String>,

    /// Skip confirmation prompts
    #[arg(global = true, short = 'y', long = "yes")]
    yes: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new board
    Init {
        /// Board name (default: "Personal")
        #[arg(short, long, default_value = "Personal")]
        name: String,
    },
    /// List boards (alias for `board list`)
    Ls,
    /// Board management
    Board {
        #[command(subcommand)]
        command: BoardCommands,
    },
    /// Column management
    Column {
        #[command(subcommand)]
        command: ColumnCommands,
    },
    /// Task management
    Task {
        #[command(subcommand)]
        command: TaskCommands,
    },
    /// Launch the TUI
    Tui,
}

#[derive(Subcommand)]
enum BoardCommands {
    /// List all boards
    List,
    /// Rename a board
    Rename {
        /// Board ID or short prefix
        id: String,
        /// New name
        name: String,
    },
    /// Show board state (columns and tasks)
    Show,
}

#[derive(Subcommand)]
enum ColumnCommands {
    /// Add a column to the board
    Add {
        /// Column name
        name: String,
    },
    /// Rename a column
    Rename {
        /// Column ID or short prefix
        id: String,
        /// New name
        name: String,
    },
    /// Remove a column
    Remove {
        /// Column ID or short prefix
        id: String,
        /// What to do with orphaned tasks: move-first or delete
        #[arg(long, default_value = "move-first")]
        orphans: OrphanActionArg,
    },
    /// Move a column to a new position
    Move {
        /// Column ID or short prefix
        id: String,
        /// New 0-based position
        position: i32,
    },
    /// List columns on the board
    List,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
enum OrphanActionArg {
    #[default]
    #[value(name = "move-first")]
    MoveFirst,
    #[value(name = "delete")]
    Delete,
}

impl From<OrphanActionArg> for OrphanAction {
    fn from(arg: OrphanActionArg) -> Self {
        match arg {
            OrphanActionArg::MoveFirst => OrphanAction::MoveToFirst,
            OrphanActionArg::Delete => OrphanAction::Delete,
        }
    }
}

#[derive(Subcommand)]
enum TaskCommands {
    /// Add a new task
    Add {
        /// Task title (prompted if omitted)
        title: Option<String>,
        /// Column ID, name, or short prefix (prompted if omitted)
        #[arg(long, short)]
        column: Option<String>,
        /// Task description
        #[arg(long, short = 'd')]
        desc: Option<String>,
        /// Priority ID or name (low/medium/high)
        #[arg(long, short, default_value = "medium")]
        priority: Option<String>,
    },
    /// Edit an existing task
    Edit {
        /// Task ID or short prefix
        id: String,
        /// New title
        #[arg(long, short)]
        title: Option<String>,
        /// New description
        #[arg(long, short = 'd')]
        desc: Option<String>,
        /// New priority ID or name
        #[arg(long, short)]
        priority: Option<String>,
        /// Move to column (ID, name, or prefix)
        #[arg(long)]
        column: Option<String>,
    },
    /// Move a task to another column
    Move {
        /// Task ID or short prefix
        id: String,
        /// Target column ID, name, or prefix
        column: String,
        /// Position within target column (default: append)
        #[arg(long, short)]
        position: Option<i32>,
    },
    /// Remove a task
    Remove {
        /// Task ID or short prefix
        id: String,
    },
    /// Show a single task
    Show {
        /// Task ID or short prefix
        id: String,
    },
    /// List tasks
    List {
        /// Filter by column ID, name, or prefix
        #[arg(long, short)]
        column: Option<String>,
        /// Filter by priority ID or name
        #[arg(long, short)]
        priority: Option<String>,
    },
}

// ── Main dispatch ──────────────────────────────────────────────────────────

/// Run the CLI with the parsed arguments.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let db_path = shell::resolve_db_path(cli.db.as_deref());

    // Open or create DB (creates dirs + runs migrations)
    let db = ensure_db(&db_path)?;

    let skip_confirm = cli.yes;
    let is_tty = atty::is(atty::Stream::Stdin);

    match cli.command {
        Some(Commands::Init { name }) => cmd_init(&db, &name),
        Some(Commands::Ls) => cmd_board_list(&db),
        Some(Commands::Board { command }) => cmd_board(&db, command),
        Some(Commands::Column { command }) => cmd_column(&db, command, skip_confirm, is_tty),
        Some(Commands::Task { command }) => cmd_task(&db, command, skip_confirm, is_tty),
        Some(Commands::Tui) => cmd_tui(&db_path),
        None => cmd_tui(&db_path), // Default action: TUI
    }
}

/// Ensure the database exists; on first run, auto-init with "Personal".
fn ensure_db(path: &std::path::Path) -> Result<shell::Db> {
    let db = shell::Db::open(path)?;

    // If no boards exist, auto-init
    let boards = shell::list_boards(&db).context("failed to list boards")?;
    if boards.is_empty() {
        shell::init_board(&db, "Personal").context("failed to initialize default board")?;
    }

    Ok(db)
}

// ── Init command ───────────────────────────────────────────────────────────

fn cmd_init(db: &shell::Db, name: &str) -> Result<()> {
    let board = shell::init_board(db, name)?;
    println!("Board '{}' initialized (id: {})", board.name, board.id);
    Ok(())
}

// ── Board commands ─────────────────────────────────────────────────────────

fn cmd_board(_db: &shell::Db, command: BoardCommands) -> Result<()> {
    match command {
        BoardCommands::List => cmd_board_list(_db),
        BoardCommands::Rename { id, name } => cmd_board_rename(_db, &id, &name),
        BoardCommands::Show => cmd_board_show(_db),
    }
}

fn cmd_board_list(db: &shell::Db) -> Result<()> {
    let boards = shell::list_boards(db)?;
    if boards.is_empty() {
        println!("No boards found. Run 'kanban init' to create one.");
        return Ok(());
    }
    for board in &boards {
        println!("  {}  {}", board.id, board.name);
    }
    Ok(())
}

fn cmd_board_rename(db: &shell::Db, board_id: &str, new_name: &str) -> Result<()> {
    shell::rename_board(db, board_id, new_name)?;
    println!("Board renamed to '{}'", new_name);
    Ok(())
}

fn cmd_board_show(db: &shell::Db) -> Result<()> {
    let state = load_default_board_state(db)?;
    println!("Board: {} (id: {})", state.board.name, state.board.id);
    println!();

    for col in &state.columns {
        let tasks: Vec<&crate::core::Task> = state
            .tasks
            .iter()
            .filter(|t| t.column_id == col.id)
            .collect();
        println!("  [{}] ({})", col.name, tasks.len());
        for task in tasks {
            let prio_name = state
                .priorities
                .iter()
                .find(|p| p.id == task.priority_id)
                .map(|p| p.name.as_str())
                .unwrap_or("?");
            println!("    {}  {} [{}]", task.id, task.title, prio_name);
        }
        println!();
    }
    Ok(())
}

// ── Column commands ────────────────────────────────────────────────────────

fn cmd_column(
    db: &shell::Db,
    command: ColumnCommands,
    _skip_confirm: bool,
    is_tty: bool,
) -> Result<()> {
    match command {
        ColumnCommands::Add { name } => {
            let name =
                crate::cli::interactive::prompt_required(Some(name), "Column name", is_tty, &[])?;
            let board_id = default_board_id(db)?;
            let col = shell::add_column(db, &board_id, &name)?;
            println!("Column '{}' added (id: {})", col.name, col.id);
        }
        ColumnCommands::Rename { id, name } => {
            shell::rename_column(db, &id, &name)?;
            println!("Column renamed to '{}'", name);
        }
        ColumnCommands::Remove { id, orphans } => {
            shell::remove_column(db, &id, orphans.into())?;
            println!("Column removed");
        }
        ColumnCommands::Move { id, position } => {
            shell::move_column(db, &id, position)?;
            println!("Column moved to position {}", position);
        }
        ColumnCommands::List => cmd_column_list(db)?,
    }
    Ok(())
}

fn cmd_column_list(db: &shell::Db) -> Result<()> {
    let board_id = default_board_id(db)?;
    let columns = shell::list_columns(db, &board_id)?;
    if columns.is_empty() {
        println!("No columns found.");
        return Ok(());
    }
    for col in &columns {
        println!("  {}  {}  (pos: {})", col.id, col.name, col.position);
    }
    Ok(())
}

// ── Task commands ──────────────────────────────────────────────────────────

fn cmd_task(db: &shell::Db, command: TaskCommands, skip_confirm: bool, is_tty: bool) -> Result<()> {
    match command {
        TaskCommands::Add {
            title,
            column,
            desc,
            priority,
        } => {
            let title = crate::cli::interactive::prompt_required(title, "Task title", is_tty, &[])?;
            let column = crate::cli::interactive::prompt_required(
                column,
                "Column (name or id)",
                is_tty,
                &load_column_names(db)?,
            )?;
            let desc_val = desc.unwrap_or_default();
            let prio = priority.unwrap_or_else(|| "medium".to_string());

            // Confirmation summary
            if !skip_confirm {
                let confirmed = crate::cli::interactive::confirm_task_add(
                    &title, &column, &desc_val, &prio, is_tty,
                )?;
                if !confirmed {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            let task = shell::add_task(db, &column, &title, &desc_val, &prio)?;
            println!("Task '{}' added (id: {})", task.title, task.id);
        }
        TaskCommands::Edit {
            id,
            title,
            desc,
            priority,
            column,
        } => {
            let mut changes = TaskChanges::default();
            if let Some(t) = title {
                changes.title = Some(t);
            }
            if let Some(d) = desc {
                changes.description = Some(d);
            }
            if let Some(p) = priority {
                changes.priority_id = Some(p);
            }
            if let Some(c) = column {
                changes.column_id = Some(c);
            }
            // Don't apply empty changes
            if changes == TaskChanges::default() {
                println!("No changes specified.");
                return Ok(());
            }
            shell::edit_task(db, &id, &changes)?;
            println!("Task updated");
        }
        TaskCommands::Move {
            id,
            column,
            position,
        } => {
            shell::move_task(db, &id, &column, position)?;
            println!("Task moved to column '{}'", column);
        }
        TaskCommands::Remove { id } => {
            shell::remove_task(db, &id)?;
            println!("Task removed");
        }
        TaskCommands::Show { id } => cmd_task_show(db, &id)?,
        TaskCommands::List { column, priority } => {
            let board_id = default_board_id(db)?;
            let filter = TaskFilter {
                column_id: column,
                priority_id: priority,
            };
            let tasks = shell::list_tasks(db, &board_id, &filter)?;
            if tasks.is_empty() {
                println!("No tasks found.");
                return Ok(());
            }
            for task in &tasks {
                println!("  {}  {}", task.id, task.title);
            }
        }
    }
    Ok(())
}

fn cmd_task_show(db: &shell::Db, task_id: &str) -> Result<()> {
    let task = shell::show_task(db, task_id)?;
    let state = load_default_board_state(db)?;

    let prio_name = state
        .priorities
        .iter()
        .find(|p| p.id == task.priority_id)
        .map(|p| p.name.as_str())
        .unwrap_or("?");
    let col_name = state
        .columns
        .iter()
        .find(|c| c.id == task.column_id)
        .map(|c| c.name.as_str())
        .unwrap_or("?");

    println!("ID:          {}", task.id);
    println!("Title:       {}", task.title);
    println!("Description: {}", task.description);
    println!("Priority:    {}", prio_name);
    println!("Column:      {}", col_name);
    println!("Position:    {}", task.position);
    println!("Created:     {}", task.created_at);
    println!("Updated:     {}", task.updated_at);
    Ok(())
}

// ── TUI ────────────────────────────────────────────────────────────────────

fn cmd_tui(db_path: &std::path::Path) -> Result<()> {
    crate::tui::run(db_path)
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn default_board_id(db: &shell::Db) -> Result<String> {
    let boards = shell::list_boards(db)?;
    boards
        .first()
        .map(|b| b.id.clone())
        .ok_or_else(|| anyhow!("no boards found; run 'kanban init' first"))
}

fn load_default_board_state(db: &shell::Db) -> Result<crate::core::BoardState> {
    let board_id = default_board_id(db)?;
    db.load_board_state(&board_id)
        .map_err(|_| anyhow!("board not found"))
}

/// Load column names from the default board for interactive prompting.
fn load_column_names(db: &shell::Db) -> Result<Vec<String>> {
    let state = load_default_board_state(db)?;
    Ok(state.columns.iter().map(|c| c.name.clone()).collect())
}

// ── Exit code wrapper ──────────────────────────────────────────────────────

/// Run the CLI, handling exit codes per the spec.
pub fn run_with_exit() {
    match run() {
        Ok(()) => process::exit(0),
        Err(err) => {
            eprintln!("error: {}", err);
            let exit_code = if err.root_cause().is::<std::io::Error>() {
                2
            } else {
                1
            };
            process::exit(exit_code);
        }
    }
}
