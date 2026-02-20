use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Cell, Row, Table};

use crate::app::App;

pub fn draw_network(frame: &mut Frame, area: Rect, app: &App) {
    let snap = app.sys.snapshot();

    let header = Row::new(vec![
        Cell::from("Interface").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("RX/s").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("TX/s").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Total RX").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Total TX").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]);

    let mut rows: Vec<Row> = snap
        .networks
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let rx_color = rate_color(n.rx_rate);
            let tx_color = rate_color(n.tx_rate);
            let bg = if i % 2 == 1 { Color::Rgb(30, 30, 40) } else { Color::Reset };
            let kind_color = match n.kind {
                crate::data::sysmetrics::NetKind::Wifi => Color::Cyan,
                crate::data::sysmetrics::NetKind::Ethernet => Color::Green,
                crate::data::sysmetrics::NetKind::Virtual => Color::DarkGray,
                crate::data::sysmetrics::NetKind::Unknown => Color::DarkGray,
            };
            // Trend arrows based on activity level.
            let rx_arrow = if n.rx_rate >= 1024 * 1024 { "\u{25b2}" }
                else if n.rx_rate >= 1024 { "\u{25b3}" }
                else { "" };
            let tx_arrow = if n.tx_rate >= 1024 * 1024 { "\u{25b2}" }
                else if n.tx_rate >= 1024 { "\u{25b3}" }
                else { "" };
            Row::new(vec![
                Cell::from(format!("{} {}", n.kind.icon(), n.name)).style(Style::default().fg(kind_color)),
                Cell::from(format!("{}{rx_arrow}", format_rate(n.rx_rate))).style(Style::default().fg(rx_color)),
                Cell::from(format!("{}{tx_arrow}", format_rate(n.tx_rate))).style(Style::default().fg(tx_color)),
                Cell::from(format_bytes(n.rx_bytes)).style(Style::default().fg(Color::DarkGray)),
                Cell::from(format_bytes(n.tx_bytes)).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(Style::default().bg(bg))
        })
        .collect();

    // Totals row.
    if snap.networks.len() > 1 {
        let total_rx_rate: u64 = snap.networks.iter().map(|n| n.rx_rate).sum();
        let total_tx_rate: u64 = snap.networks.iter().map(|n| n.tx_rate).sum();
        let total_rx: u64 = snap.networks.iter().map(|n| n.rx_bytes).sum();
        let total_tx: u64 = snap.networks.iter().map(|n| n.tx_bytes).sum();
        rows.push(
            Row::new(vec![
                Cell::from("TOTAL").style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from(format_rate(total_rx_rate)).style(Style::default().fg(rate_color(total_rx_rate)).add_modifier(Modifier::BOLD)),
                Cell::from(format_rate(total_tx_rate)).style(Style::default().fg(rate_color(total_tx_rate)).add_modifier(Modifier::BOLD)),
                Cell::from(format_bytes(total_rx)).style(Style::default().fg(Color::Gray)),
                Cell::from(format_bytes(total_tx)).style(Style::default().fg(Color::Gray)),
            ])
            .style(Style::default().bg(Color::Rgb(40, 40, 50)))
        );
    }

    let widths = [
        Constraint::Min(12),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
    ];

    // Show aggregate bandwidth in title.
    let total_rx_rate: u64 = snap.networks.iter().map(|n| n.rx_rate).sum();
    let total_tx_rate: u64 = snap.networks.iter().map(|n| n.tx_rate).sum();
    let max_rate = total_rx_rate.max(total_tx_rate);
    let net_title = if total_rx_rate > 0 || total_tx_rate > 0 {
        format!(" Network [rx:{} tx:{}] ", format_rate(total_rx_rate), format_rate(total_tx_rate))
    } else {
        format!(" Network ({}) ", snap.networks.len())
    };
    let border_color = if max_rate >= 10 * 1024 * 1024 {
        Color::Magenta
    } else if max_rate >= 1024 * 1024 {
        Color::Yellow
    } else {
        Color::Blue
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(net_title)
                .border_style(Style::default().fg(border_color)),
        );

    frame.render_widget(table, area);
}

fn rate_color(bytes_per_sec: u64) -> Color {
    const MIB: u64 = 1024 * 1024;
    if bytes_per_sec >= 10 * MIB {
        Color::Red
    } else if bytes_per_sec >= MIB {
        Color::Yellow
    } else if bytes_per_sec > 0 {
        Color::Green
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

fn format_bytes(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    const MIB: u64 = 1024 * 1024;
    const KIB: u64 = 1024;
    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.0} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}
