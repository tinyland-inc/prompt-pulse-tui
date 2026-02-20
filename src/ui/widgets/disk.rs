use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Gauge};

use crate::app::App;

pub fn draw_disks(frame: &mut Frame, area: Rect, app: &App) {
    let snap = app.sys.snapshot();

    // Disk space warning: if any disk > 90% or available < 5GB, highlight border.
    let max_pct = snap.disks.iter().map(|d| d.percent).fold(0.0f64, f64::max);
    let min_avail_gib = snap
        .disks
        .iter()
        .map(|d| (d.total.saturating_sub(d.used)) as f64 / (1024.0 * 1024.0 * 1024.0))
        .fold(f64::MAX, f64::min);
    let (border_color, title) = if max_pct >= 95.0 {
        (
            Color::Red,
            format!(" Disks ({}) [!{max_pct:.0}%] ", snap.disks.len()),
        )
    } else if max_pct >= 85.0 || min_avail_gib < 5.0 {
        (
            Color::Yellow,
            format!(" Disks ({}) [{min_avail_gib:.0}G free] ", snap.disks.len()),
        )
    } else {
        (Color::Blue, format!(" Disks ({}) ", snap.disks.len()))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .border_style(Style::default().fg(border_color));

    if snap.disks.is_empty() {
        frame.render_widget(block, area);
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // One gauge row per disk (2 lines each: 1 for gauge, 1 spacing).
    let constraints: Vec<Constraint> = snap
        .disks
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if i == snap.disks.len() - 1 {
                Constraint::Min(1)
            } else {
                Constraint::Length(2)
            }
        })
        .collect();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, disk) in snap.disks.iter().enumerate() {
        if i >= rows.len() {
            break;
        }

        let color = pct_gradient(disk.percent);
        let icon = if disk.is_removable { "\u{23cf} " } else { "" };

        let avail = disk.total.saturating_sub(disk.used);
        let fs_tag = if disk.fs_type.is_empty() {
            String::new()
        } else {
            format!(" [{}]", disk.fs_type)
        };
        let label = format!(
            "{}{}{}: {} / {} ({:.0}%) {} free",
            icon,
            truncate_mount(&disk.mount, 18),
            fs_tag,
            format_bytes(disk.used),
            format_bytes(disk.total),
            disk.percent,
            format_bytes(avail),
        );

        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(color))
            .ratio((disk.percent / 100.0).clamp(0.0, 1.0))
            .label(label);

        frame.render_widget(gauge, rows[i]);
    }
}

fn pct_gradient(pct: f64) -> Color {
    if pct >= 90.0 {
        Color::Red
    } else if pct >= 80.0 {
        Color::Rgb(255, 100, 0)
    } else if pct >= 65.0 {
        Color::Yellow
    } else if pct >= 40.0 {
        Color::Rgb(150, 255, 0)
    } else {
        Color::Green
    }
}

fn truncate_mount(mount: &str, max_len: usize) -> &str {
    if mount.len() <= max_len {
        mount
    } else {
        &mount[mount.len() - max_len..]
    }
}

fn format_bytes(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    const TIB: u64 = 1024 * GIB;
    if bytes >= TIB {
        format!("{:.1}T", bytes as f64 / TIB as f64)
    } else {
        format!("{:.1}G", bytes as f64 / GIB as f64)
    }
}
