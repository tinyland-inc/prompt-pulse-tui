use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Sparkline as RatatuiSparkline};

use crate::app::App;

/// Draw CPU usage sparkline (last 60 seconds).
pub fn draw_cpu_sparkline(frame: &mut Frame, area: Rect, app: &App) {
    let data: Vec<u64> = app.cpu_history.iter().map(|v| *v as u64).collect();
    let current = data.last().copied().unwrap_or(0);
    let avg: u64 = if data.is_empty() {
        0
    } else {
        data.iter().sum::<u64>() / data.len() as u64
    };
    let peak = data.iter().copied().max().unwrap_or(0);
    let lo = data.iter().copied().min().unwrap_or(0);

    let color = pct_gradient(current);
    let title = format!(" CPU {current}% (avg:{avg} pk:{peak} lo:{lo}) ");

    let sparkline = RatatuiSparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .data(&data)
        .max(100)
        .style(Style::default().fg(color));

    frame.render_widget(sparkline, area);
}

/// Draw memory usage sparkline (last 60 seconds).
pub fn draw_mem_sparkline(frame: &mut Frame, area: Rect, app: &App) {
    let data: Vec<u64> = app.mem_history.iter().map(|v| *v as u64).collect();
    let current = data.last().copied().unwrap_or(0);
    let avg: u64 = if data.is_empty() {
        0
    } else {
        data.iter().sum::<u64>() / data.len() as u64
    };
    let peak = data.iter().copied().max().unwrap_or(0);
    let lo = data.iter().copied().min().unwrap_or(0);

    let color = pct_gradient(current);
    let title = format!(" MEM {current}% (avg:{avg} pk:{peak} lo:{lo}) ");

    let sparkline = RatatuiSparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .data(&data)
        .max(100)
        .style(Style::default().fg(color));

    frame.render_widget(sparkline, area);
}

/// Draw swap usage sparkline (last 60 seconds).
pub fn draw_swap_sparkline(frame: &mut Frame, area: Rect, app: &App) {
    let data: Vec<u64> = app.swap_history.iter().map(|v| *v as u64).collect();
    let current = data.last().copied().unwrap_or(0);
    let peak = data.iter().copied().max().unwrap_or(0);
    let lo = data.iter().copied().min().unwrap_or(0);

    let color = pct_gradient(current);
    let title = format!(" Swap {current}% (pk:{peak} lo:{lo}) ");

    let sparkline = RatatuiSparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .data(&data)
        .max(100)
        .style(Style::default().fg(color));

    frame.render_widget(sparkline, area);
}

/// Draw network RX rate sparkline (last 60 seconds).
pub fn draw_net_rx_sparkline(frame: &mut Frame, area: Rect, app: &App) {
    let data: Vec<u64> = app.net_rx_history.iter().map(|v| *v as u64).collect();
    let current = data.last().copied().unwrap_or(0);
    let peak = data.iter().copied().max().unwrap_or(0);
    let label = format_rate(current);
    let peak_label = format_rate(peak);
    let color = net_rate_color(current);

    let sparkline = RatatuiSparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(" RX {label} (pk:{peak_label}) "))
                .border_style(Style::default().fg(Color::Blue)),
        )
        .data(&data)
        .style(Style::default().fg(color));

    frame.render_widget(sparkline, area);
}

/// Draw network TX rate sparkline (last 60 seconds).
pub fn draw_net_tx_sparkline(frame: &mut Frame, area: Rect, app: &App) {
    let data: Vec<u64> = app.net_tx_history.iter().map(|v| *v as u64).collect();
    let current = data.last().copied().unwrap_or(0);
    let peak = data.iter().copied().max().unwrap_or(0);
    let label = format_rate(current);
    let peak_label = format_rate(peak);
    let color = net_rate_color(current);

    let sparkline = RatatuiSparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(" TX {label} (pk:{peak_label}) "))
                .border_style(Style::default().fg(Color::Blue)),
        )
        .data(&data)
        .style(Style::default().fg(color));

    frame.render_widget(sparkline, area);
}

/// Draw load average (1-minute) sparkline.
pub fn draw_load_sparkline(frame: &mut Frame, area: Rect, app: &App) {
    let snap = app.sys.snapshot();
    let cpu_count = snap.cpu_count.max(1) as f64;

    // Scale load as percentage of core count (load 1.0 on 8-core = 12.5%).
    let data: Vec<u64> = app
        .load_history
        .iter()
        .map(|v| ((v / cpu_count) * 100.0).clamp(0.0, 200.0) as u64)
        .collect();
    let current = app.load_history.back().copied().unwrap_or(0.0);
    let peak = app.load_history.iter().cloned().fold(0.0f64, f64::max);
    let load_pct = ((current / cpu_count) * 100.0) as u64;

    let color = pct_gradient(load_pct.min(100));
    let title = format!(" Load {current:.2} (pk:{peak:.2}) ");

    let sparkline = RatatuiSparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .data(&data)
        .max(100)
        .style(Style::default().fg(color));

    frame.render_widget(sparkline, area);
}

/// Draw max temperature sparkline (last 60 seconds).
pub fn draw_temp_sparkline(frame: &mut Frame, area: Rect, app: &App) {
    let data: Vec<u64> = app.temp_history.iter().map(|v| *v as u64).collect();
    let current = data.last().copied().unwrap_or(0);
    let peak = data.iter().copied().max().unwrap_or(0);
    let lo = data.iter().copied().min().unwrap_or(0);

    let color = temp_color(current);
    let title = format!(" Temp {current}°C (pk:{peak} lo:{lo}) ");

    let sparkline = RatatuiSparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .data(&data)
        .max(110) // max reasonable temp
        .style(Style::default().fg(color));

    frame.render_widget(sparkline, area);
}

/// Draw per-core CPU mini sparklines in a compact grid (4 per row).
pub fn draw_cpu_per_core(frame: &mut Frame, area: Rect, app: &App) {
    let cores = app.cpu_per_core_history.len();
    if cores == 0 {
        return;
    }

    let snap = app.sys.snapshot();

    let cols_per_row = if area.width >= 120 { 4 } else { 2 };
    let num_rows = (cores + cols_per_row - 1) / cols_per_row;
    let row_height = (area.height as usize / num_rows).max(3) as u16;

    let row_constraints: Vec<Constraint> = (0..num_rows)
        .map(|i| {
            if i == num_rows - 1 {
                Constraint::Min(3)
            } else {
                Constraint::Length(row_height)
            }
        })
        .collect();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    let col_constraints: Vec<Constraint> = (0..cols_per_row)
        .map(|_| Constraint::Ratio(1, cols_per_row as u32))
        .collect();

    for (core_idx, history) in app.cpu_per_core_history.iter().enumerate() {
        let row_idx = core_idx / cols_per_row;
        let col_idx = core_idx % cols_per_row;

        if row_idx >= rows.len() {
            break;
        }

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints.clone())
            .split(rows[row_idx]);

        if col_idx >= cols.len() {
            break;
        }

        let data: Vec<u64> = history.iter().map(|v| *v as u64).collect();
        let current = data.last().copied().unwrap_or(0);
        let color = pct_gradient(current);

        // Show per-core frequency if available.
        let freq_tag = snap
            .cpu_freqs
            .get(core_idx)
            .filter(|&&f| f > 0)
            .map(|&f| {
                let ghz = f as f64 / 1000.0;
                format!(" {ghz:.1}G")
            })
            .unwrap_or_default();

        let sparkline = RatatuiSparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(format!(" C{core_idx} {current}%{freq_tag} "))
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .data(&data)
            .max(100)
            .style(Style::default().fg(color));

        frame.render_widget(sparkline, cols[col_idx]);
    }
}

/// btm-style color gradient: green -> yellow -> red based on percentage.
fn pct_gradient(pct: u64) -> Color {
    if pct >= 90 {
        Color::Red
    } else if pct >= 80 {
        Color::Rgb(255, 100, 0) // orange-red
    } else if pct >= 65 {
        Color::Yellow
    } else if pct >= 40 {
        Color::Rgb(150, 255, 0) // yellow-green
    } else {
        Color::Green
    }
}

/// Temperature color: green (cool) -> yellow (warm) -> red (hot).
fn temp_color(temp_c: u64) -> Color {
    if temp_c >= 90 {
        Color::Red
    } else if temp_c >= 80 {
        Color::Rgb(255, 100, 0)
    } else if temp_c >= 65 {
        Color::Yellow
    } else if temp_c >= 45 {
        Color::Rgb(150, 255, 0)
    } else {
        Color::Green
    }
}

/// Network rate color: idle=DarkGray, low=Green, medium=Cyan, high=Yellow, very high=Magenta.
fn net_rate_color(bytes_per_sec: u64) -> Color {
    const MIB: u64 = 1024 * 1024;
    const KIB: u64 = 1024;
    if bytes_per_sec >= 10 * MIB {
        Color::Magenta
    } else if bytes_per_sec >= MIB {
        Color::Yellow
    } else if bytes_per_sec >= 100 * KIB {
        Color::Cyan
    } else if bytes_per_sec >= KIB {
        Color::Green
    } else if bytes_per_sec > 0 {
        Color::DarkGray
    } else {
        Color::DarkGray
    }
}

fn format_rate(bytes_per_sec: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    const KIB: u64 = 1024;
    if bytes_per_sec >= MIB {
        format!("{:.1} MB/s", bytes_per_sec as f64 / MIB as f64)
    } else if bytes_per_sec >= KIB {
        format!("{:.0} KB/s", bytes_per_sec as f64 / KIB as f64)
    } else if bytes_per_sec > 0 {
        format!("{} B/s", bytes_per_sec)
    } else {
        "idle".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pct_gradient_thresholds() {
        // <40% should be Green
        let low = pct_gradient(20);
        assert_eq!(low, Color::Green);
        // 90%+ should be Red
        let high = pct_gradient(95);
        assert_eq!(high, Color::Red);
        // 65-79 should be Yellow
        let mid = pct_gradient(70);
        assert_eq!(mid, Color::Yellow);
    }

    #[test]
    fn test_format_rate_units() {
        assert!(format_rate(500).contains("B/s"));
        assert!(format_rate(2048).contains("KB/s"));
        assert!(format_rate(2 * 1024 * 1024).contains("MB/s"));
        assert_eq!(format_rate(0), "idle");
    }
}
