use crate::core::BoardState;

/// Current mode of the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Edit,
    Column,
    Confirm,
    Help,
    Move,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Normal => write!(f, "NORMAL"),
            Mode::Insert => write!(f, "INSERT"),
            Mode::Edit => write!(f, "EDIT"),
            Mode::Column => write!(f, "COLUMN"),
            Mode::Confirm => write!(f, "CONFIRM"),
            Mode::Help => write!(f, "HELP"),
            Mode::Move => write!(f, "MOVE"),
        }
    }
}

/// Context for a confirmation dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmContext {
    /// Confirming deletion of a task (y/N)
    TaskDelete,
    /// Confirming deletion of a column (m=move-to-first, d=delete-all, n=cancel)
    ColumnDelete,
}

/// Possible actions resulting from input handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
    ModeChange(Mode),
    NavigatePrevColumn,
    NavigateNextColumn,
    NavigatePrevTask,
    NavigateNextTask,
    AddTask,
    EditTask,
    DeleteTask,
    EnterMoveMode,
    MoveTaskPrevColumn,
    MoveTaskNextColumn,
    MoveTaskDown,
    MoveTaskUp,
    EnterColumnMode,
    ToggleHelp,
    // Move mode actions — adjust the target highlight, not the focus
    MoveTargetPrev,
    MoveTargetNext,
    MoveConfirm,
    MoveCancel,
    // Column mode actions
    ColumnAdd,
    ColumnRename,
    ColumnDelete,
    ColumnMoveLeft,
    ColumnMoveRight,
    ColumnExit,
    // Insert/Edit actions
    Save,
    Cancel,
    CycleField,
    CyclePriority,
    InsertText(String),
    DeleteChar,
    ConfirmYes,
    ConfirmNo,
    // Column-delete three-way choices
    ConfirmMoveToFirst,
    ConfirmDeleteAll,
}

/// The TUI application state.
pub struct App {
    pub state: Option<BoardState>,
    pub mode: Mode,
    /// Index into the current column's tasks.
    pub focused_col_idx: usize,
    pub focused_task_idx: usize,
    pub error: Option<String>,
    pub error_tick: u64,
    pub tick: u64,
    /// Text buffer for insert/edit modes.
    pub input_buffer: String,
    pub cursor_pos: usize,
    /// For edit mode: which field is being edited (0=title, 1=description, 2=priority).
    pub edit_field: usize,
    /// The task being edited (if any).
    pub editing_task: Option<crate::core::Task>,
    /// Pending delete task ID.
    pub delete_task_id: Option<String>,
    /// For move mode: the highlighted destination column index.
    pub move_target_col_idx: usize,
    /// What we are confirming (task delete vs column delete).
    pub confirm_context: Option<ConfirmContext>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            state: None,
            mode: Mode::Normal,
            focused_col_idx: 0,
            focused_task_idx: 0,
            error: None,
            error_tick: 0,
            tick: 0,
            input_buffer: String::new(),
            cursor_pos: 0,
            edit_field: 0,
            editing_task: None,
            delete_task_id: None,
            move_target_col_idx: 0,
            confirm_context: None,
        }
    }

    pub fn with_state(state: BoardState) -> Self {
        let mut app = Self::new();
        app.state = Some(state);
        app
    }

    pub fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
        self.error_tick = self.tick;
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// Get the focused column, if any.
    pub fn focused_column(&self) -> Option<&crate::core::Column> {
        self.state
            .as_ref()
            .and_then(|s| s.columns.get(self.focused_col_idx))
    }

    /// Get the tasks in the focused column.
    pub fn focused_column_tasks(&self) -> Vec<&crate::core::Task> {
        let col = match self.focused_column() {
            Some(c) => &c.id,
            None => return vec![],
        };
        self.state
            .as_ref()
            .map(|s| s.tasks.iter().filter(|t| t.column_id == *col).collect())
            .unwrap_or_default()
    }

    /// Get the currently focused task.
    pub fn focused_task(&self) -> Option<&crate::core::Task> {
        let tasks = self.focused_column_tasks();
        tasks.get(self.focused_task_idx).map(|v| &**v)
    }

    pub fn clamp_focus(&mut self) {
        if let Some(ref state) = self.state {
            self.focused_col_idx = self
                .focused_col_idx
                .min(state.columns.len().saturating_sub(1));
            let tasks = self.focused_column_tasks();
            self.focused_task_idx = self.focused_task_idx.min(tasks.len().saturating_sub(1));
        }
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        // Clear error after ~3 seconds (assuming 20 ticks/sec)
        if self.error.is_some() && (self.tick - self.error_tick) > 60 {
            self.clear_error();
        }
    }
}
