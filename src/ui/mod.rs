pub mod layout;
pub mod widgets;

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, Tab};

/// Top-level draw: tab bar + active tab content + help bar + optional help overlay.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Split into tab bar (3 lines) + content + help bar (1 line).
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    widgets::tabs::draw_tabs(frame, chunks[0], app);

    match app.active_tab {
        Tab::Dashboard => layout::dashboard(frame, chunks[1], app),
        Tab::System => layout::system(frame, chunks[1], app),
        Tab::Network => layout::network(frame, chunks[1], app),
        Tab::Billing => layout::billing(frame, chunks[1], app),
    }

    widgets::help::draw_help_bar(frame, chunks[2], app);

    // Help overlay (centered popup).
    if app.show_help {
        draw_help_overlay(frame, area);
    }
}

fn draw_help_overlay(frame: &mut Frame, area: Rect) {
    let popup_width = 52u16.min(area.width.saturating_sub(4));
    let popup_height = 32u16.min(area.height.saturating_sub(4));

    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled("Keyboard Shortcuts", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  q / Esc / Ctrl-C", Style::default().fg(Color::Yellow)),
            Span::raw("  Quit"),
        ]),
        Line::from(vec![
            Span::styled("  Tab / Right     ", Style::default().fg(Color::Yellow)),
            Span::raw("  Next tab"),
        ]),
        Line::from(vec![
            Span::styled("  Shift-Tab / Left", Style::default().fg(Color::Yellow)),
            Span::raw("  Previous tab"),
        ]),
        Line::from(vec![
            Span::styled("  1-4             ", Style::default().fg(Color::Yellow)),
            Span::raw("  Jump to tab"),
        ]),
        Line::from(vec![
            Span::styled("  Space           ", Style::default().fg(Color::Yellow)),
            Span::raw("  Freeze/resume data"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Process Table (System tab)", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j/k / Up/Down   ", Style::default().fg(Color::Yellow)),
            Span::raw("  Scroll processes"),
        ]),
        Line::from(vec![
            Span::styled("  g / Home        ", Style::default().fg(Color::Yellow)),
            Span::raw("  Jump to top"),
        ]),
        Line::from(vec![
            Span::styled("  G / End         ", Style::default().fg(Color::Yellow)),
            Span::raw("  Jump to bottom"),
        ]),
        Line::from(vec![
            Span::styled("  /               ", Style::default().fg(Color::Yellow)),
            Span::raw("  Filter by name/PID"),
        ]),
        Line::from(vec![
            Span::styled("  c / m / p / n   ", Style::default().fg(Color::Yellow)),
            Span::raw("  Sort: CPU/Mem/PID/Name"),
        ]),
        Line::from(vec![
            Span::styled("  r               ", Style::default().fg(Color::Yellow)),
            Span::raw("  Reverse sort order"),
        ]),
        Line::from(vec![
            Span::styled("  e               ", Style::default().fg(Color::Yellow)),
            Span::raw("  Toggle full command"),
        ]),
        Line::from(vec![
            Span::styled("  t               ", Style::default().fg(Color::Yellow)),
            Span::raw("  Toggle tree view"),
        ]),
        Line::from(vec![
            Span::styled("  PgUp / PgDn     ", Style::default().fg(Color::Yellow)),
            Span::raw("  Jump 10 processes"),
        ]),
        Line::from(vec![
            Span::styled("  dd              ", Style::default().fg(Color::Yellow)),
            Span::raw("  Kill process (TERM)"),
        ]),
        Line::from(vec![
            Span::styled("  D               ", Style::default().fg(Color::Yellow)),
            Span::raw("  Force kill (KILL)"),
        ]),
        Line::from(vec![
            Span::styled("  Mouse scroll    ", Style::default().fg(Color::Yellow)),
            Span::raw("  Scroll processes"),
        ]),
        Line::from(""),
        Line::from(Span::styled("General", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  + / -           ", Style::default().fg(Color::Yellow)),
            Span::raw("  Adjust refresh rate"),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Press any key to close", Style::default().fg(Color::DarkGray))),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Help (?) ")
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, popup_area);
}
