use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::App;

pub fn draw_host_info(frame: &mut Frame, area: Rect, app: &App) {
    let snap = app.sys.snapshot();

    let uptime = format_uptime(snap.uptime_secs);
    let cpu_count = snap.cpu_count.max(1) as f64;
    let load_ratio = snap.load_avg[0] / cpu_count;
    let load_color = if load_ratio >= 1.5 {
        Color::Red
    } else if load_ratio >= 1.0 {
        Color::Rgb(255, 100, 0)
    } else if load_ratio >= 0.7 {
        Color::Yellow
    } else {
        Color::Gray
    };
    let load = format!(
        "Load: {:.2} / {:.2} / {:.2}",
        snap.load_avg[0], snap.load_avg[1], snap.load_avg[2]
    );

    let shell = std::env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(String::from))
        .unwrap_or_else(|| "unknown".into());

    let term = std::env::var("TERM_PROGRAM")
        .or_else(|_| std::env::var("TERM"))
        .unwrap_or_else(|_| "unknown".into());

    let total_ram = format_bytes_gib(snap.mem_total);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                &snap.hostname,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - "),
            Span::styled(&snap.os_name, Style::default().fg(Color::White)),
            Span::raw(" "),
            Span::styled(&snap.arch, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::raw("Kernel: "),
            Span::styled(&snap.kernel_version, Style::default().fg(Color::Gray)),
            Span::raw("  CPUs: "),
            Span::styled(
                format!("{}", snap.cpu_count),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  RAM: "),
            Span::styled(total_ram, Style::default().fg(Color::Yellow)),
        ]),
    ];

    if !snap.cpu_brand.is_empty() {
        let mut cpu_spans = vec![
            Span::raw("CPU: "),
            Span::styled(&snap.cpu_brand, Style::default().fg(Color::White)),
        ];
        if snap.cpu_freq_mhz > 0 {
            let ghz = snap.cpu_freq_mhz as f64 / 1000.0;
            cpu_spans.push(Span::raw(" @ "));
            cpu_spans.push(Span::styled(
                format!("{ghz:.1} GHz"),
                Style::default().fg(Color::Yellow),
            ));
        }
        // Show max temperature if available.
        if !snap.temperatures.is_empty() {
            let max_temp = snap
                .temperatures
                .iter()
                .map(|t| t.temp_c)
                .fold(0.0f32, f32::max);
            if max_temp > 0.0 {
                let temp_color = if max_temp >= 90.0 {
                    Color::Red
                } else if max_temp >= 75.0 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                cpu_spans.push(Span::raw("  "));
                cpu_spans.push(Span::styled(
                    format!("{max_temp:.0}°C"),
                    Style::default().fg(temp_color),
                ));
            }
        }
        lines.push(Line::from(cpu_spans));
    }

    // Uptime color: green (fresh) -> cyan (days) -> yellow (weeks) -> gray (months).
    let uptime_color = if snap.uptime_secs < 86400 {
        Color::Green
    } else if snap.uptime_secs < 7 * 86400 {
        Color::Cyan
    } else if snap.uptime_secs < 30 * 86400 {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    lines.push(Line::from(vec![
        Span::raw("Uptime: "),
        Span::styled(uptime, Style::default().fg(uptime_color)),
        Span::raw("  "),
        Span::styled(load, Style::default().fg(load_color)),
    ]));

    // IP + process count + memory pressure.
    let mut ip_spans = vec![
        Span::raw("IP: "),
        Span::styled(&snap.local_ip, Style::default().fg(Color::Cyan)),
        Span::raw("  Procs: "),
        Span::styled(
            format!("{}", snap.process_count),
            Style::default().fg(Color::Yellow),
        ),
    ];
    // Memory pressure tag.
    if snap.mem_percent >= 90.0 {
        ip_spans.push(Span::styled(
            "  MEM!",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    } else if snap.mem_percent >= 80.0 {
        ip_spans.push(Span::styled("  MEM", Style::default().fg(Color::Yellow)));
    }
    lines.push(Line::from(ip_spans));

    // Shell + terminal + nix packages + swap.
    let mut env_spans = vec![
        Span::raw("Shell: "),
        Span::styled(&shell, Style::default().fg(Color::Magenta)),
        Span::raw("  Term: "),
        Span::styled(&term, Style::default().fg(Color::Magenta)),
    ];
    // Swap tag when active.
    if snap.swap_used > 0 && snap.swap_total > 0 {
        let swap_pct = (snap.swap_used as f64 / snap.swap_total as f64) * 100.0;
        let swap_color = if swap_pct >= 70.0 {
            Color::Red
        } else if swap_pct >= 30.0 {
            Color::Yellow
        } else {
            Color::DarkGray
        };
        env_spans.push(Span::raw("  Swap: "));
        env_spans.push(Span::styled(
            format!("{:.0}%", swap_pct),
            Style::default().fg(swap_color),
        ));
    }
    if snap.nix_packages > 0 {
        env_spans.push(Span::raw("  Pkgs: "));
        env_spans.push(Span::styled(
            format!("{}", snap.nix_packages),
            Style::default().fg(Color::Cyan),
        ));
    }
    lines.push(Line::from(env_spans));

    // Battery info (laptops only).
    if let Some(batt) = &snap.battery {
        let batt_color = if batt.percent >= 50.0 {
            Color::Green
        } else if batt.percent >= 20.0 {
            Color::Yellow
        } else {
            Color::Red
        };
        let charge_icon = if batt.charging { " +" } else { "" };
        let mut batt_spans = vec![
            Span::raw("Battery: "),
            Span::styled(
                format!("{:.0}%{charge_icon}", batt.percent),
                Style::default().fg(batt_color),
            ),
        ];
        // Time remaining estimate.
        if let Some(time) = &batt.time_remaining {
            batt_spans.push(Span::raw("  "));
            batt_spans.push(Span::styled(
                format!("{time} left"),
                Style::default().fg(Color::Gray),
            ));
        }
        batt_spans.push(Span::raw("  "));
        batt_spans.push(Span::styled(
            &batt.source,
            Style::default().fg(Color::DarkGray),
        ));
        lines.push(Line::from(batt_spans));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Host ")
        .border_style(Style::default().fg(Color::Blue));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn format_bytes_gib(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    format!("{:.0} GiB", bytes as f64 / GIB as f64)
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}
