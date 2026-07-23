//! Dialog widget — renders popup overlays for insert, edit, and confirm dialogs.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::tui::app::{App, ConfirmContext, Mode};

/// Render the insert task popup overlay.
pub fn render_insert_overlay(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let overlay_width = 40u16.min(area.width.saturating_sub(2));
    let overlay_height = 5u16.min(area.height.saturating_sub(2));
    let popup_area = Rect {
        x: (area.width - overlay_width) / 2,
        y: (area.height - overlay_height) / 2,
        width: overlay_width,
        height: overlay_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" New Task Title ");

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("Title: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.input_buffer),
            Span::raw("█"),
        ]),
        Line::raw(""),
        Line::raw("  Enter save | Esc cancel"),
        Line::raw(""),
    ];

    let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);
}

/// Render the edit task popup overlay with field cycling.
pub fn render_edit_overlay(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let overlay_width = 50u16.min(area.width.saturating_sub(2));
    let overlay_height = 12u16.min(area.height.saturating_sub(2));
    let popup_area = Rect {
        x: (area.width - overlay_width) / 2,
        y: (area.height - overlay_height) / 2,
        width: overlay_width,
        height: overlay_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Edit Task ");

    let task = match &app.editing_task {
        Some(t) => t,
        None => {
            let paragraph = Paragraph::new(Text::from("No task to edit"));
            frame.render_widget(paragraph, popup_area);
            return;
        }
    };

    let state = match &app.state {
        Some(s) => s,
        None => {
            let paragraph = Paragraph::new(Text::from("No state"));
            frame.render_widget(paragraph, popup_area);
            return;
        }
    };

    let prio_name = state
        .priorities
        .iter()
        .find(|p| p.id == task.priority_id)
        .map(|p| p.name.as_str())
        .unwrap_or("?");

    let mut lines = vec![Line::raw("")];

    // Title
    let title_label = if app.edit_field == 0 {
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    lines.push(Line::from(vec![
        Span::styled("Title: ", title_label),
        Span::raw(if app.edit_field == 0 {
            &app.input_buffer
        } else {
            &task.title
        }),
        if app.mode == Mode::EditField && app.edit_field == 0 {
            Span::raw("█")
        } else {
            Span::raw("")
        },
    ]));

    // Description
    let desc_label = if app.edit_field == 1 {
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    lines.push(Line::from(vec![
        Span::styled("Description: ", desc_label),
        Span::raw(if app.edit_field == 1 {
            &app.input_buffer
        } else {
            &task.description
        }),
        if app.mode == Mode::EditField && app.edit_field == 1 {
            Span::raw("█")
        } else {
            Span::raw("")
        },
    ]));

    // Priority
    let prio_label = if app.edit_field == 2 {
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    lines.push(Line::from(vec![
        Span::styled("Priority: ", prio_label),
        Span::styled(
            format!("[{}]", prio_name),
            if app.edit_field == 2 {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            },
        ),
    ]));

    let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true });
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);
    frame.render_widget(paragraph, inner);
}

/// Render a confirmation dialog (y/N) for destructive actions.
pub fn render_confirm_overlay(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let overlay_width = 45u16.min(area.width.saturating_sub(2));
    let overlay_height = 5u16.min(area.height.saturating_sub(2));
    let popup_area = Rect {
        x: (area.width - overlay_width) / 2,
        y: (area.height - overlay_height) / 2,
        width: overlay_width,
        height: overlay_height,
    };

    let block = Block::default().borders(Borders::ALL).title(" Confirm ");

    let task = app.focused_task();
    let col = app.focused_column();
    let prompt = match app.confirm_context {
        Some(ConfirmContext::TaskDelete) => {
            if let Some(task) = task {
                format!("Delete task '{}' (y/N)?", task.title)
            } else {
                "Delete task (y/N)?".to_string()
            }
        }
        Some(ConfirmContext::ColumnDelete) => {
            if let Some(col) = col {
                let state = app.state.as_ref();
                let task_count = state
                    .map(|s| s.tasks.iter().filter(|t| t.column_id == col.id).count())
                    .unwrap_or(0);
                format!(
                    "Delete column '{}' ({} tasks)?\n  m=move-to-first  d=delete-all  n=cancel",
                    col.name, task_count
                )
            } else {
                "Delete column?\n  m=move-to-first  d=delete-all  n=cancel".to_string()
            }
        }
        None => {
            if let Some(task) = task {
                format!("Delete task '{}' (y/N)?", task.title)
            } else if let Some(_col) = col {
                "Delete column? (y/N)".to_string()
            } else {
                "Delete (y/N)?".to_string()
            }
        }
    };

    let lines: Vec<Line> = prompt
        .lines()
        .map(Line::from)
        .chain(std::iter::once(Line::raw("")))
        .collect();

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}
