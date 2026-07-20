use crate::core::{Board, BoardState, Column, Task};

/// Render a board state as a text board view.
pub fn render_board(state: &BoardState) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Board: {} (id: {})\n\n",
        state.board.name, state.board.id
    ));

    for col in &state.columns {
        let tasks: Vec<&Task> = state
            .tasks
            .iter()
            .filter(|t| t.column_id == col.id)
            .collect();
        out.push_str(&format!("  [{}] ({})\n", col.name, tasks.len()));
        for task in tasks {
            let prio_name = state
                .priorities
                .iter()
                .find(|p| p.id == task.priority_id)
                .map(|p| p.name.as_str())
                .unwrap_or("?");
            out.push_str(&format!(
                "    {}  {} [{}]\n",
                task.id, task.title, prio_name
            ));
        }
        out.push('\n');
    }

    out
}

/// Render a list of boards as text lines.
pub fn render_boards(boards: &[Board]) -> String {
    let mut out = String::new();
    for board in boards {
        out.push_str(&format!("  {}  {}\n", board.id, board.name));
    }
    out
}

/// Render a list of columns as text lines.
pub fn render_columns(columns: &[Column]) -> String {
    let mut out = String::new();
    for col in columns {
        out.push_str(&format!(
            "  {}  {}  (pos: {})\n",
            col.id, col.name, col.position
        ));
    }
    out
}

/// Render a list of tasks as text lines.
pub fn render_tasks(tasks: &[Task]) -> String {
    let mut out = String::new();
    for task in tasks {
        out.push_str(&format!("  {}  {}\n", task.id, task.title));
    }
    out
}

/// Render a single task as text lines.
pub fn render_task(task: &Task, state: &BoardState) -> String {
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

    format!(
        "ID:          {}\n\
         Title:       {}\n\
         Description: {}\n\
         Priority:    {}\n\
         Column:      {}\n\
         Position:    {}\n\
         Created:     {}\n\
         Updated:     {}\n",
        task.id,
        task.title,
        task.description,
        prio_name,
        col_name,
        task.position,
        task.created_at,
        task.updated_at
    )
}
