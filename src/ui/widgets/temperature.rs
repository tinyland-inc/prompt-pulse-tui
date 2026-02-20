use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Row, Table};

use crate::app::App;

pub fn draw_temperatures(frame: &mut Frame, area: Rect, app: &App) {
    let snap = app.sys.snapshot();

    let max_temp = snap
        .temperatures
        .iter()
        .map(|t| t.temp_c)
        .fold(0.0f32, f32::max);
    let avg_temp = if snap.temperatures.is_empty() {
        0.0
    } else {
        snap.temperatures.iter().map(|t| t.temp_c).sum::<f32>() / snap.temperatures.len() as f32
    };
    let border_color = if max_temp >= 85.0 {
        Color::Red
    } else {
        Color::Blue
    };
    let title = if max_temp >= 85.0 {
        format!(" Temps ({}) [!{max_temp:.0}°C] ", snap.temperatures.len())
    } else if !snap.temperatures.is_empty() {
        format!(" Temps ({}) avg:{avg_temp:.0}°C ", snap.temperatures.len())
    } else {
        format!(" Temps ({}) ", snap.temperatures.len())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .border_style(Style::default().fg(border_color));

    if snap.temperatures.is_empty() {
        let p = ratatui::widgets::Paragraph::new("No sensors")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(p, area);
        return;
    }

    let header = Row::new(vec!["Sensor", "Temp", "Max"]).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = snap
        .temperatures
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let color = temp_gradient(t.temp_c);
            let bg = if i % 2 == 1 {
                Color::Rgb(30, 30, 40)
            } else {
                Color::Reset
            };
            Row::new(vec![
                truncate_label(&t.label, 22),
                format!("{:.0}°C", t.temp_c),
                if t.max_c > 0.0 {
                    format!("{:.0}°C", t.max_c)
                } else {
                    "-".into()
                },
            ])
            .style(Style::default().fg(color).bg(bg))
        })
        .collect();

    let widths = [
        Constraint::Min(12),
        Constraint::Length(7),
        Constraint::Length(7),
    ];

    let table = Table::new(rows, widths).header(header).block(block);
    frame.render_widget(table, area);
}

/// 5-step temperature gradient matching btm aesthetics.
fn temp_gradient(temp: f32) -> Color {
    if temp >= 90.0 {
        Color::Red
    } else if temp >= 80.0 {
        Color::Rgb(255, 100, 0)
    } else if temp >= 65.0 {
        Color::Yellow
    } else if temp >= 45.0 {
        Color::Rgb(150, 255, 0)
    } else {
        Color::Green
    }
}

fn truncate_label(label: &str, max: usize) -> String {
    if label.len() <= max {
        label.to_string()
    } else {
        format!("{}...", &label[..max - 3])
    }
}
