use ratatui::prelude::*;
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, BorderType, Borders, Gauge};

use crate::app::App;

pub fn draw_cpu_bars(frame: &mut Frame, area: Rect, app: &App) {
    let snap = app.sys.snapshot();

    let freq_tag = if snap.cpu_freq_mhz > 0 {
        let ghz = snap.cpu_freq_mhz as f64 / 1000.0;
        format!(" @ {ghz:.1}GHz")
    } else {
        String::new()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(
            " CPU ({:.1}% avg, {} cores{freq_tag}) ",
            snap.cpu_total, snap.cpu_count
        ))
        .border_style(Style::default().fg(Color::Blue));

    // If enough height, show per-core bar chart. Otherwise, show aggregate gauge.
    if area.height >= 6 && snap.cpu_usage.len() > 1 {
        let bars: Vec<Bar> = snap
            .cpu_usage
            .iter()
            .enumerate()
            .map(|(i, &usage)| {
                let color = usage_color(usage);
                Bar::default()
                    .label(Line::from(format!("{i}")))
                    .value(usage as u64)
                    .style(Style::default().fg(color))
            })
            .collect();

        let chart = BarChart::default()
            .block(block)
            .data(BarGroup::default().bars(&bars))
            .bar_width(3)
            .bar_gap(1)
            .max(100);

        frame.render_widget(chart, area);
    } else {
        let gauge = Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(usage_color(snap.cpu_total)))
            .ratio((snap.cpu_total as f64 / 100.0).clamp(0.0, 1.0))
            .label(format!("{:.1}%", snap.cpu_total));

        frame.render_widget(gauge, area);
    }
}

fn usage_color(pct: f32) -> Color {
    if pct >= 90.0 {
        Color::Red
    } else if pct >= 80.0 {
        Color::Rgb(255, 100, 0) // orange-red
    } else if pct >= 65.0 {
        Color::Yellow
    } else if pct >= 40.0 {
        Color::Rgb(150, 255, 0) // yellow-green
    } else {
        Color::Green
    }
}
