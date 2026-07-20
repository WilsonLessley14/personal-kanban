//! TUI widget modules for rendering the kanban board.
//!
//! Organized into focused modules:
//! - [`board`] — full board layout, title/status bars, column panels
//! - [`card`] — individual task card rendering with jj-style IDs
//! - [`dialog`] — popup overlays for insert, edit, and confirm
//! - [`help`] — help overlay showing all keybindings

pub mod board;
pub mod card;
pub mod dialog;
pub mod help;

pub use board::render_board;

#[cfg(test)]
mod tests {
    use crate::tui::render::render_task_id_spans;

    #[test]
    fn render_task_id_spans_produces_two_spans() {
        let spans = render_task_id_spans("abc123", 2);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "ab");
        assert_eq!(spans[1].content, "c123");
    }
}
