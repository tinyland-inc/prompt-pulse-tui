use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Tabs as RatatuiTabs};

use crate::app::{App, Tab};

pub fn draw_tabs(frame: &mut Frame, area: Rect, app: &mut App) {
    let hostname = app.sys.snapshot().hostname.clone();
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let num = format!("{}", i + 1);
            let style = if *t == app.active_tab {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(vec![
                Span::styled(num, Style::default().fg(Color::Cyan)),
                Span::raw(":"),
                Span::styled(t.title(), style),
            ])
        })
        .collect();

    let selected = Tab::ALL
        .iter()
        .position(|t| *t == app.active_tab)
        .unwrap_or(0);

    // Show clock, refresh rate, and frozen indicator on the right side.
    let now = chrono::Local::now();
    let clock = now.format("%H:%M:%S").to_string();

    let tabs = RatatuiTabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .title(format!(" prompt-pulse v3 :: {hostname} "))
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .title_bottom(Line::from(vec![]).right_aligned())
                .title(
                    Line::from({
                        let mut spans = vec![Span::styled(
                            format!(" {clock} "),
                            Style::default().fg(Color::DarkGray),
                        )];
                        if app.refresh_ms != 1000 {
                            spans.push(Span::styled(
                                format!("{:.1}s ", app.refresh_ms as f64 / 1000.0),
                                Style::default().fg(Color::Cyan),
                            ));
                        }
                        if app.frozen {
                            spans.push(Span::styled(
                                "[FROZEN] ",
                                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                            ));
                        }
                        spans
                    })
                    .right_aligned(),
                ),
        )
        .select(selected)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .divider(Span::styled(" | ", Style::default().fg(Color::DarkGray)));

    frame.render_widget(tabs, area);
}
