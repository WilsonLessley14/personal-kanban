use ratatui::prelude::Stylize;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use crate::core::min_unique_prefixes;

use super::app::App;

/// Compute the styled spans for a task's short ID (jj-style).
///
/// The minimum-unique prefix is rendered in bold/bright, the remainder in dim.
pub fn render_task_id_spans(task_id: &str, min_len: usize) -> Vec<Span<'_>> {
    let (prefix, rest) = task_id.split_at(min_len);
    let parts = [prefix, rest];
    vec![
        Span::styled(
            parts[0].to_string(),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::styled(parts[1].to_string(), Style::default().dim()),
    ]
}

/// Build the set of visible task IDs for computing min-unique prefixes.
pub fn get_visible_task_ids(app: &App) -> Vec<&str> {
    app.state
        .as_ref()
        .map(|s| s.tasks.iter().map(|t| t.id.as_str()).collect())
        .unwrap_or_default()
}

/// Get the minimum prefix lengths for all visible tasks, as a map.
pub fn get_min_prefix_lengths(app: &App) -> std::collections::HashMap<String, usize> {
    let ids = get_visible_task_ids(app);
    let prefixes = min_unique_prefixes(&ids);
    prefixes.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_task_id_spans_single_char() {
        let spans = render_task_id_spans("abc123", 1);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "a");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[1].content, "bc123");
    }

    #[test]
    fn render_task_id_spans_full_id() {
        let spans = render_task_id_spans("abc", 3);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "abc");
        assert_eq!(spans[1].content, "");
    }

    #[test]
    fn render_task_id_spans_various_lengths() {
        let spans = render_task_id_spans("V1StGXR8_Z5jdHi6B", 5);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "V1StG");
        assert_eq!(spans[1].content, "XR8_Z5jdHi6B");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn get_min_prefix_lengths_integration() {
        use crate::core::{Board, BoardState, Column, Priority, Task};

        let board = Board {
            id: "b1".into(),
            name: "Test".into(),
            created_at: "".into(),
            updated_at: "".into(),
        };
        let columns = vec![Column {
            id: "c1".into(),
            board_id: "b1".into(),
            name: "Backlog".into(),
            position: 0,
            created_at: "".into(),
            updated_at: "".into(),
        }];
        let tasks = vec![
            Task {
                id: "a3x9k2".into(),
                column_id: "c1".into(),
                title: "A".into(),
                description: "".into(),
                priority_id: "p1".into(),
                position: 0,
                created_at: "".into(),
                updated_at: "".into(),
            },
            Task {
                id: "a3bQ7f".into(),
                column_id: "c1".into(),
                title: "B".into(),
                description: "".into(),
                priority_id: "p1".into(),
                position: 1,
                created_at: "".into(),
                updated_at: "".into(),
            },
            Task {
                id: "m8rT2p".into(),
                column_id: "c1".into(),
                title: "C".into(),
                description: "".into(),
                priority_id: "p1".into(),
                position: 2,
                created_at: "".into(),
                updated_at: "".into(),
            },
        ];
        let priorities = vec![Priority {
            id: "p1".into(),
            name: "medium".into(),
        }];

        let state = BoardState {
            board,
            columns,
            tasks,
            priorities,
        };
        let app = App::with_state(state);

        let lengths = get_min_prefix_lengths(&app);
        assert_eq!(*lengths.get("m8rT2p").unwrap(), 1);
        assert_eq!(*lengths.get("a3x9k2").unwrap(), 3);
        assert_eq!(*lengths.get("a3bQ7f").unwrap(), 3);
    }
}
