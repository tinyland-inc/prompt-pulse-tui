use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::{App, Tab};

pub fn draw_help_bar(frame: &mut Frame, area: Rect, app: &App) {
    // Filter mode: show filter input prompt.
    if app.filter_mode {
        let line = Line::from(vec![
            Span::styled(" /", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(&app.process_filter, Style::default().fg(Color::White)),
            Span::styled("|", Style::default().fg(Color::Yellow)),
            Span::styled("  Enter", Style::default().fg(Color::DarkGray)),
            Span::styled(" confirm ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::styled(" clear", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    let mut keys = vec![
        Span::styled(" q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" Quit ", Style::default().fg(Color::DarkGray)),
        Span::styled("Tab", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" Next ", Style::default().fg(Color::DarkGray)),
        Span::styled("1-4", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" Jump ", Style::default().fg(Color::DarkGray)),
    ];

    // Context-sensitive hints for System tab.
    if app.active_tab == Tab::System {
        keys.extend([
            Span::styled("j/k", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Scroll ", Style::default().fg(Color::DarkGray)),
            Span::styled("/", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Filter ", Style::default().fg(Color::DarkGray)),
            Span::styled("c/m/p/n", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Sort ", Style::default().fg(Color::DarkGray)),
            Span::styled("r", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Rev ", Style::default().fg(Color::DarkGray)),
            Span::styled("e", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Expand ", Style::default().fg(Color::DarkGray)),
            Span::styled("t", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Tree ", Style::default().fg(Color::DarkGray)),
            Span::styled("dd", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Kill ", Style::default().fg(Color::DarkGray)),
        ]);
    }

    keys.extend([
        Span::styled("+/-", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" Speed ", Style::default().fg(Color::DarkGray)),
        Span::styled("Space", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" Freeze ", Style::default().fg(Color::DarkGray)),
        Span::styled("?", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" Help", Style::default().fg(Color::DarkGray)),
    ]);

    // Right-aligned status indicators.
    // Refresh rate indicator.
    let rate_color = if app.refresh_ms <= 250 {
        Color::Green
    } else if app.refresh_ms <= 1000 {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let rate_label = if app.refresh_ms >= 1000 {
        format!("{:.1}s", app.refresh_ms as f64 / 1000.0)
    } else {
        format!("{}ms", app.refresh_ms)
    };
    keys.push(Span::styled(
        format!("  [{rate_label}]"),
        Style::default().fg(rate_color),
    ));

    // Mode indicators.
    if app.tree_mode {
        keys.push(Span::styled(
            " [TREE]",
            Style::default().fg(Color::Cyan),
        ));
    }
    if app.show_cmd {
        keys.push(Span::styled(
            " [CMD]",
            Style::default().fg(Color::Cyan),
        ));
    }

    // Show frozen indicator.
    if app.frozen {
        keys.push(Span::styled(
            " [FROZEN]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }

    // Show pending kill indicator.
    if app.pending_kill.is_some() {
        keys.push(Span::styled(
            " [d?]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }

    let line = Line::from(keys);
    let help = Paragraph::new(line);
    frame.render_widget(help, area);
}
