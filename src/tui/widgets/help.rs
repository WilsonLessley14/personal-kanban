//! Help widget — renders the help overlay showing all keybindings.

use ratatui::layout::Rect;
use ratatui::text::{Line, Text};
use ratatui::widgets::Paragraph;

/// Render the help overlay showing all available keybindings.
pub fn render_help_overlay(frame: &mut ratatui::Frame<'_>, area: Rect) {
    let lines = vec![
        Line::raw("  Normal Mode                                   "),
        Line::raw("  h/l  prev/next column    j/k  prev/next task        "),
        Line::raw("  a  add task             Enter  view task             "),
        Line::raw("  m  move mode            d  delete task              "),
        Line::raw("  H/L  move task col      J/K  reorder task           "),
        Line::raw("  C  column mode          ?  toggle help              "),
        Line::raw("  q  quit                                                  "),
        Line::raw("                                                  "),
        Line::raw("  Move Mode                                   "),
        Line::raw("  h/l  highlight dest col  Enter  confirm   Esc cancel"),
        Line::raw("                                                  "),
        Line::raw("  Column Mode                                 "),
        Line::raw("  a  add  r  rename  d  delete  h/l  reorder  Esc exit"),
        Line::raw("                                                  "),
        Line::raw("  View Task                                   "),
        Line::raw("  Tab/j/k  cycle field  i  edit  Enter save  Esc cancel"),
        Line::raw("  Edit Field                                  "),
        Line::raw("  Enter  save  Esc  cancel  Tab/p cycle (priority)    "),
        Line::raw("                                                  "),
    ];

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, area);
}
