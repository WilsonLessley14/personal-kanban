use thiserror::Error;

/// Domain-level errors for kanban operations.
#[derive(Debug, Error, PartialEq)]
pub enum DomainError {
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

/// Errors from short-ID resolution.
#[derive(Debug, Error, PartialEq)]
pub enum IdError {
    #[error("no match for ID prefix '{prefix}'")]
    NotFound { prefix: String },

    #[error("ambiguous ID prefix '{prefix}' matches: {}", matches.join(", "))]
    Ambiguous {
        prefix: String,
        matches: Vec<String>,
    },
}
