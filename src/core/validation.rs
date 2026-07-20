use crate::core::error::DomainError;
use crate::core::types::{Column, Priority};

/// Maximum allowed length for a task title.
pub const MAX_TITLE_LENGTH: usize = 200;

/// Validate a task title is non-empty and within length limits.
pub fn validate_title(title: &str) -> Result<(), DomainError> {
    if title.is_empty() {
        return Err(DomainError::EmptyTitle);
    }
    if title.len() > MAX_TITLE_LENGTH {
        return Err(DomainError::TitleTooLong {
            max: MAX_TITLE_LENGTH,
        });
    }
    Ok(())
}

/// Validate a column name is non-empty and unique within the existing columns.
pub fn validate_column_name(name: &str, existing: &[Column]) -> Result<(), DomainError> {
    if name.is_empty() {
        return Err(DomainError::EmptyColumnName);
    }
    if existing.iter().any(|c| c.name == name) {
        return Err(DomainError::DuplicateColumnName {
            name: name.to_string(),
        });
    }
    Ok(())
}

/// Validate that a priority with the given ID exists in the provided list.
pub fn validate_priority(priority_id: &str, priorities: &[Priority]) -> Result<(), DomainError> {
    if priorities.iter().any(|p| p.id == priority_id) {
        return Ok(());
    }
    Err(DomainError::PriorityNotFound {
        id: priority_id.to_string(),
    })
}

/// Validate that a column with the given ID exists in the provided list.
pub fn validate_column_exists(column_id: &str, columns: &[Column]) -> Result<(), DomainError> {
    if columns.iter().any(|c| c.id == column_id) {
        return Ok(());
    }
    Err(DomainError::ColumnNotFound {
        id: column_id.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_column(id: &str, name: &str) -> Column {
        Column {
            id: id.to_string(),
            board_id: "board1".to_string(),
            name: name.to_string(),
            position: 0,
            created_at: "".to_string(),
            updated_at: "".to_string(),
        }
    }

    fn sample_priority(id: &str, name: &str) -> Priority {
        Priority {
            id: id.to_string(),
            name: name.to_string(),
        }
    }

    #[test]
    fn validate_title_rejects_empty() {
        assert_eq!(validate_title(""), Err(DomainError::EmptyTitle));
    }

    #[test]
    fn validate_title_accepts_valid() {
        assert!(validate_title("Fix the bug").is_ok());
    }

    #[test]
    fn validate_title_rejects_too_long() {
        let long = "a".repeat(MAX_TITLE_LENGTH + 1);
        assert_eq!(
            validate_title(&long),
            Err(DomainError::TitleTooLong {
                max: MAX_TITLE_LENGTH
            })
        );
    }

    #[test]
    fn validate_title_accepts_exact_max() {
        let exact = "a".repeat(MAX_TITLE_LENGTH);
        assert!(validate_title(&exact).is_ok());
    }

    #[test]
    fn validate_column_name_rejects_empty() {
        assert_eq!(
            validate_column_name("", &[]),
            Err(DomainError::EmptyColumnName)
        );
    }

    #[test]
    fn validate_column_name_accepts_unique() {
        assert!(validate_column_name("New Column", &[]).is_ok());
    }

    #[test]
    fn validate_column_name_rejects_duplicate() {
        let cols = vec![sample_column("c1", "Backlog")];
        assert_eq!(
            validate_column_name("Backlog", &cols),
            Err(DomainError::DuplicateColumnName {
                name: "Backlog".to_string()
            })
        );
    }

    #[test]
    fn validate_column_name_accepts_similar_but_different() {
        let cols = vec![sample_column("c1", "Backlog")];
        assert!(validate_column_name("backlog", &cols).is_ok());
    }

    #[test]
    fn validate_priority_finds_existing() {
        let prios = vec![sample_priority("p1", "high")];
        assert!(validate_priority("p1", &prios).is_ok());
    }

    #[test]
    fn validate_priority_rejects_missing() {
        let prios = vec![sample_priority("p1", "high")];
        assert_eq!(
            validate_priority("p99", &prios),
            Err(DomainError::PriorityNotFound {
                id: "p99".to_string()
            })
        );
    }

    #[test]
    fn validate_priority_rejects_empty_list() {
        let prios: Vec<Priority> = vec![];
        assert_eq!(
            validate_priority("p1", &prios),
            Err(DomainError::PriorityNotFound {
                id: "p1".to_string()
            })
        );
    }

    #[test]
    fn validate_column_exists_finds_existing() {
        let cols = vec![sample_column("c1", "Backlog")];
        assert!(validate_column_exists("c1", &cols).is_ok());
    }

    #[test]
    fn validate_column_exists_rejects_missing() {
        let cols = vec![sample_column("c1", "Backlog")];
        assert_eq!(
            validate_column_exists("c99", &cols),
            Err(DomainError::ColumnNotFound {
                id: "c99".to_string()
            })
        );
    }

    #[test]
    fn validate_column_exists_rejects_empty_list() {
        let cols: Vec<Column> = vec![];
        assert_eq!(
            validate_column_exists("c1", &cols),
            Err(DomainError::ColumnNotFound {
                id: "c1".to_string()
            })
        );
    }

    #[test]
    fn error_display_messages() {
        assert_eq!(DomainError::EmptyTitle.to_string(), "title cannot be empty");
        assert_eq!(
            DomainError::TitleTooLong { max: 200 }.to_string(),
            "title exceeds maximum length of 200 characters"
        );
        assert_eq!(
            DomainError::EmptyColumnName.to_string(),
            "column name cannot be empty"
        );
        assert_eq!(
            DomainError::DuplicateColumnName {
                name: "Test".to_string()
            }
            .to_string(),
            "column 'Test' already exists on this board"
        );
        assert_eq!(
            DomainError::ColumnNotFound {
                id: "abc".to_string()
            }
            .to_string(),
            "column not found: 'abc'"
        );
        assert_eq!(
            DomainError::TaskNotFound {
                id: "xyz".to_string()
            }
            .to_string(),
            "task not found: 'xyz'"
        );
        assert_eq!(
            DomainError::BoardNotFound {
                id: "b1".to_string()
            }
            .to_string(),
            "board not found: 'b1'"
        );
        assert_eq!(
            DomainError::PriorityNotFound {
                id: "p1".to_string()
            }
            .to_string(),
            "priority not found: 'p1'"
        );
        assert_eq!(
            DomainError::CannotDeleteLastColumn.to_string(),
            "cannot delete the last column on a board"
        );
        assert_eq!(
            DomainError::DuplicateBoardName {
                name: "Main".to_string()
            }
            .to_string(),
            "board 'Main' already exists"
        );
        assert_eq!(
            DomainError::PositionOutOfRange {
                position: 5,
                max: 3
            }
            .to_string(),
            "position 5 is out of range (0..3)"
        );
    }
}
