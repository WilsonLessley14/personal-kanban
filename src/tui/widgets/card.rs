//! Card widget — renders individual task cards with jj-style short IDs.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};

use crate::tui::app::App;
use crate::tui::render::{get_min_prefix_lengths, render_task_id_spans};

/// Render a single task card in the given area.
pub fn render_task_card(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    task: &crate::core::Task,
    is_selected: bool,
    area: Rect,
) {
    let card_style = if is_selected {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default()
    };

    let border_style = if is_selected {
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Magenta)
    } else {
        Style::default()
    };

    let prio_name = app
        .state
        .as_ref()
        .and_then(|s| {
            s.priorities
                .iter()
                .find(|p| p.id == task.priority_id)
                .map(|p| p.name.as_str())
        })
        .unwrap_or("?");

    let prefix_lengths = get_min_prefix_lengths(app);
    let min_len = prefix_lengths.get(&task.id).copied().unwrap_or(1);
    let id_spans = render_task_id_spans(&task.id, min_len);

    let title_line = Line::from(Span::styled(format!("{}  ", task.title), card_style));

    let id_line = Line::from(
        id_spans
            .iter()
            .map(|s| Span::styled(s.content.clone(), s.style))
            .collect::<Vec<_>>(),
    )
    .right_aligned();

    let text = Text::from(vec![title_line]);
    let card = Paragraph::new(text)
        .block(
            Block::bordered()
                .title(Span::styled(format!("[{}]", prio_name), card_style))
                .border_type(BorderType::Rounded)
                .border_style(border_style)
                .title_bottom(id_line),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(card, area);
}
