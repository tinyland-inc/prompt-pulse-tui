use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Cell, Row, Table};

use crate::app::App;

pub fn draw_tailscale(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue));

    match &app.tailscale {
        Some(ts) => {
            let online = ts.online_peers_sorted();
            // Aggregate bandwidth across all peers.
            let total_rx: i64 = online.iter().map(|p| p.rx_bytes).sum();
            let total_tx: i64 = online.iter().map(|p| p.tx_bytes).sum();
            let bw_tag = if total_rx > 0 || total_tx > 0 {
                format!(" rx:{} tx:{}", format_bytes(total_rx), format_bytes(total_tx))
            } else {
                String::new()
            };
            let title = format!(
                " Tailscale - {} ({}/{} online{bw_tag}) ",
                ts.tailnet_name, online.len(), ts.total_peers
            );

            let hdr_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
            let header = Row::new(vec![
                Cell::from("Host").style(hdr_style),
                Cell::from("OS").style(hdr_style),
                Cell::from("IP").style(hdr_style),
                Cell::from("Seen").style(hdr_style),
                Cell::from("RX").style(hdr_style),
                Cell::from("TX").style(hdr_style),
            ]);

            let rows: Vec<Row> = online
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let ip = p.tailscale_ips.first().cloned().unwrap_or_default();
                    let bg = if i % 2 == 1 { Color::Rgb(30, 30, 40) } else { Color::Reset };
                    let seen = p.last_seen
                        .map(|t| format_relative_time(t))
                        .unwrap_or_else(|| "now".into());
                    let seen_color = if seen == "now" || seen.ends_with('s') {
                        Color::Green
                    } else if seen.ends_with('m') {
                        Color::Cyan
                    } else {
                        Color::DarkGray
                    };
                    Row::new(vec![
                        Cell::from(p.hostname.clone()).style(Style::default().fg(Color::Green)),
                        Cell::from(p.os.clone()).style(Style::default().fg(Color::Gray)),
                        Cell::from(ip).style(Style::default().fg(Color::Cyan)),
                        Cell::from(seen).style(Style::default().fg(seen_color)),
                        Cell::from(format_bytes(p.rx_bytes)).style(Style::default().fg(Color::DarkGray)),
                        Cell::from(format_bytes(p.tx_bytes)).style(Style::default().fg(Color::DarkGray)),
                    ])
                    .style(Style::default().bg(bg))
                })
                .collect();

            let widths = [
                Constraint::Min(14),
                Constraint::Length(8),
                Constraint::Length(16),
                Constraint::Length(6),
                Constraint::Length(9),
                Constraint::Length(9),
            ];

            let table = Table::new(rows, widths)
                .header(header)
                .block(block.title(title));

            frame.render_widget(table, area);
        }
        None => {
            let paragraph = ratatui::widgets::Paragraph::new("Waiting for daemon data...")
                .style(Style::default().fg(Color::DarkGray))
                .block(block.title(" Tailscale "));
            frame.render_widget(paragraph, area);
        }
    }
}

fn format_relative_time(t: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let delta = now.signed_duration_since(t);
    let secs = delta.num_seconds();
    if secs < 0 || secs < 60 {
        "now".into()
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

fn format_bytes(bytes: i64) -> String {
    let bytes = bytes as u64;
    const GIB: u64 = 1024 * 1024 * 1024;
    const MIB: u64 = 1024 * 1024;
    const KIB: u64 = 1024;
    if bytes >= GIB {
        format!("{:.1}G", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.0}M", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.0}K", bytes as f64 / KIB as f64)
    } else if bytes > 0 {
        format!("{bytes}B")
    } else {
        "-".into()
    }
}
