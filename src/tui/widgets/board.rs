//! Board widget — renders the full board layout with title bar, columns, and status bar.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::core::Task;
use crate::tui::app::{App, Mode};

use super::card::render_task_card;
use super::dialog::{render_confirm_overlay, render_edit_overlay, render_insert_overlay};
use super::help::render_help_overlay;

/// Render the full board view with columns, task cards, title bar, and status bar.
pub fn render_board(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    let area = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_title_bar(frame, vertical[0], app);

    let content = vertical[1];
    match app.mode {
        Mode::Help => render_help_overlay(frame, content),
        Mode::Insert => render_insert_overlay(frame, content, app),
        Mode::ViewTask | Mode::EditField => render_edit_overlay(frame, content, app),
        Mode::Confirm => render_confirm_overlay(frame, content, app),
        _ => render_column_panels(frame, content, app),
    }

    render_status_bar(frame, vertical[2], app);
}

fn render_title_bar(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let board_name = app
        .state
        .as_ref()
        .map(|s| s.board.name.as_str())
        .unwrap_or("Kanban");
    let text = format!(" {} ", board_name);
    let paragraph = Paragraph::new(Text::from(text)).style(
        Style::default()
            .fg(Color::Magenta)
            .bg(Color::Reset)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}

fn render_status_bar(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let text = if let Some(ref err) = app.error {
        err.clone()
    } else {
        match app.mode {
            Mode::Normal => "h/l col | j/k task | a add | enter view | m move | d del | H/L move-task | J/K reorder | C col-mode | ? help | q quit".to_string(),
            Mode::Move => "MOVE MODE — h/l target col | Enter confirm | Esc cancel".to_string(),
            Mode::Column => "COLUMN MODE — a add | r rename | d delete | h/l reorder | Esc exit".to_string(),
            Mode::Insert => "INSERT — Enter save | Esc cancel".to_string(),
            Mode::ViewTask => "VIEW — Tab/j/k cycle | i edit | Enter save | Esc cancel".to_string(),
            Mode::EditField => match app.edit_field {
                0 | 1 => "EDIT FIELD — Enter save | Esc cancel".to_string(),
                2 => "EDIT PRIORITY — Tab/p cycle | Enter save | Esc cancel".to_string(),
                _ => "EDIT — Enter save | Esc cancel".to_string(),
            },
            Mode::Confirm => "CONFIRM — y/N".to_string(),
            Mode::Help => "?/Esc/q close".to_string(),
        }
    };

    let style = if app.error.is_some() {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Gray)
    };

    let paragraph = Paragraph::new(Text::from(text)).style(style);
    frame.render_widget(paragraph, area);
}

/// Render the column panels (normal/move/column mode).
pub fn render_column_panels(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let state = match &app.state {
        Some(s) => s,
        None => {
            let text = "No board loaded";
            let paragraph = Paragraph::new(Text::from(text));
            frame.render_widget(paragraph, area);
            return;
        }
    };

    let num_cols = state.columns.len();
    if num_cols == 0 {
        let text = "No columns";
        let paragraph = Paragraph::new(Text::from(text));
        frame.render_widget(paragraph, area);
        return;
    }

    let constraints: Vec<Constraint> =
        std::iter::repeat_n(Constraint::Percentage(100 / num_cols as u16), num_cols).collect();
    let col_areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (col_idx, (col, col_area)) in state.columns.iter().zip(col_areas.iter()).enumerate() {
        let tasks: Vec<&Task> = state
            .tasks
            .iter()
            .filter(|t| t.column_id == col.id)
            .collect();

        let is_focused_col = col_idx == app.focused_col_idx;
        let is_move_target = app.mode == Mode::Move
            && col_idx == app.move_target_col_idx
            && col_idx != app.focused_col_idx;

        let col_block_style = if is_move_target {
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Reset)
                .add_modifier(Modifier::BOLD)
        } else if is_focused_col {
            Style::default()
                .fg(Color::White)
                .bg(Color::Reset)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray).bg(Color::Reset)
        };

        let col_border_style = if is_move_target {
            Style::default().fg(Color::Yellow).bg(Color::Reset)
        } else if is_focused_col {
            Style::default()
                .fg(Color::Magenta)
                .bg(Color::Reset)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray).bg(Color::Reset)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::raw(&col.name))
            .style(col_block_style)
            .border_type(BorderType::Rounded)
            .border_style(col_border_style);
        frame.render_widget(&block, *col_area);

        let inner = block.inner(*col_area);

        if tasks.is_empty() {
            let empty_text = Paragraph::new(Text::from("  (empty)"));
            frame.render_widget(empty_text, inner);
            continue;
        }

        let card_height: u16 = 5;
        let available_height = inner.height as usize;
        let card_height_usize = card_height as usize;
        let max_visible = available_height.saturating_sub(1) / card_height_usize;

        let focused_idx = if is_focused_col {
            app.focused_task_idx
        } else {
            0
        };
        let visible_tasks = tasks
            .iter()
            .cloned()
            .enumerate()
            .skip_while(|(i, _)| *i + max_visible <= focused_idx)
            .take(max_visible)
            .map(|(_, t)| t)
            .collect::<Vec<_>>();

        let mut y_offset: u16 = 0;
        for (task_idx, task) in visible_tasks.iter().enumerate() {
            let actual_idx = if focused_idx >= max_visible {
                focused_idx - max_visible + task_idx
            } else {
                task_idx
            };
            let is_selected = is_focused_col && actual_idx == app.focused_task_idx;

            let remaining_height = inner.height.saturating_sub(y_offset);
            let card_area = Rect {
                x: inner.x,
                y: inner.y + y_offset,
                width: inner.width,
                height: card_height.min(remaining_height),
            };

            if card_area.height < 2 || card_area.width < 1 {
                break;
            }

            render_task_card(frame, app, task, is_selected, card_area);

            y_offset += card_height;
            if y_offset >= inner.height {
                break;
            }
        }
    }
}
