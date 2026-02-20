use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Gauge};

use crate::app::App;

pub fn draw_memory(frame: &mut Frame, area: Rect, app: &App) {
    let snap = app.sys.snapshot();

    // Memory pressure warning: change border + title when under pressure.
    let avail_gib = snap.mem_available as f64 / (1024.0 * 1024.0 * 1024.0);
    let (border_color, title) = if snap.mem_percent >= 90.0 {
        (Color::Red, format!(" Memory [!{:.0}%] ", snap.mem_percent))
    } else if snap.mem_percent >= 80.0 || avail_gib < 2.0 {
        (Color::Yellow, format!(" Memory [{:.1}G free] ", avail_gib))
    } else {
        (Color::Blue, " Memory ".to_string())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .border_style(Style::default().fg(border_color));

    // Split into RAM gauge and swap gauge.
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    // RAM gauge.
    let ram_ratio = (snap.mem_percent / 100.0).clamp(0.0, 1.0);
    let ram_label = format!(
        "RAM: {} / {} ({:.1}%)  avail: {}",
        format_bytes(snap.mem_used),
        format_bytes(snap.mem_total),
        snap.mem_percent,
        format_bytes(snap.mem_available),
    );
    let ram_color = if snap.mem_percent >= 90.0 {
        Color::Red
    } else if snap.mem_percent >= 80.0 {
        Color::Rgb(255, 100, 0)
    } else if snap.mem_percent >= 65.0 {
        Color::Yellow
    } else if snap.mem_percent >= 40.0 {
        Color::Rgb(150, 255, 0)
    } else {
        Color::Green
    };

    let ram = Gauge::default()
        .gauge_style(Style::default().fg(ram_color))
        .ratio(ram_ratio)
        .label(ram_label);
    frame.render_widget(ram, chunks[0]);

    // Swap gauge.
    if snap.swap_total > 0 {
        let swap_pct = (snap.swap_used as f64 / snap.swap_total as f64) * 100.0;
        let swap_ratio = (swap_pct / 100.0).clamp(0.0, 1.0);
        let swap_label = format!(
            "Swap: {} / {} ({:.1}%)",
            format_bytes(snap.swap_used),
            format_bytes(snap.swap_total),
            swap_pct,
        );
        let swap_color = if swap_pct >= 90.0 {
            Color::Red
        } else if swap_pct >= 70.0 {
            Color::Rgb(255, 100, 0)
        } else if swap_pct >= 40.0 {
            Color::Yellow
        } else {
            Color::Magenta
        };
        let swap = Gauge::default()
            .gauge_style(Style::default().fg(swap_color))
            .ratio(swap_ratio)
            .label(swap_label);
        frame.render_widget(swap, chunks[1]);
    }
}

fn format_bytes(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    const MIB: u64 = 1024 * 1024;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else {
        format!("{:.0} MiB", bytes as f64 / MIB as f64)
    }
}
