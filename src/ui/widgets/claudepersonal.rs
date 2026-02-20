use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Gauge, Paragraph};

use crate::app::App;

pub fn draw_claude_personal(frame: &mut Frame, area: Rect, app: &App) {
    let (title, gauge_ratio, gauge_color, status_text) = match &app.claude_personal {
        Some(report) => {
            let title = format!(
                " Claude Pro [{}/{}] ",
                report.messages_in_window, report.message_limit
            );
            let ratio = if report.message_limit > 0 {
                (report.messages_in_window as f64 / report.message_limit as f64).min(1.0)
            } else {
                0.0
            };
            let color = if ratio >= 0.90 {
                Color::Red
            } else if ratio >= 0.70 {
                Color::Yellow
            } else {
                Color::Green
            };
            let remaining = (report.message_limit - report.messages_in_window).max(0);
            let mut status = format!(
                "{} remaining in {}h window",
                remaining, report.window_hours
            );
            if report.next_slot_secs > 0 {
                let hours = report.next_slot_secs / 3600;
                let mins = (report.next_slot_secs % 3600) / 60;
                if hours > 0 {
                    status.push_str(&format!("  Reset: {}h{:02}m", hours, mins));
                } else {
                    status.push_str(&format!("  Reset: {}m", mins));
                }
            }
            (title, ratio, color, status)
        }
        None => {
            let title = " Claude Pro ".to_string();
            (title, 0.0, Color::DarkGray, "Scanning sessions...".to_string())
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .border_style(Style::default().fg(Color::Rgb(124, 58, 237))); // purple

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    if app.claude_personal.is_some() {
        // Split inner into gauge + status line.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);

        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(gauge_color).bg(Color::DarkGray))
            .ratio(gauge_ratio);
        frame.render_widget(gauge, chunks[0]);

        if chunks[1].height > 0 {
            let status = Paragraph::new(status_text)
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(status, chunks[1]);
        }
    } else {
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(paragraph, inner);
    }
}
