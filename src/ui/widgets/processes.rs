use ratatui::prelude::*;
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
    TableState,
};

use crate::app::{App, ProcessSort};

pub fn draw_processes(frame: &mut Frame, area: Rect, app: &mut App) {
    let sort_indicator = |col: ProcessSort| -> &str {
        if app.process_sort == col {
            if app.sort_reverse {
                " \u{25b2}"
            } else {
                " \u{25bc}"
            }
        } else {
            ""
        }
    };

    let name_header = if app.show_cmd { "Cmd" } else { "Name" };
    let header_style = |col: Option<ProcessSort>| -> Style {
        let is_active = col.is_some_and(|c| c == app.process_sort);
        let fg = if is_active {
            Color::Yellow
        } else {
            Color::Cyan
        };
        Style::default().fg(fg).add_modifier(Modifier::BOLD)
    };
    let header = Row::new(vec![
        Cell::from("S").style(header_style(None)),
        Cell::from(format!("PID{}", sort_indicator(ProcessSort::Pid)))
            .style(header_style(Some(ProcessSort::Pid))),
        Cell::from("User").style(header_style(None)),
        Cell::from(format!(
            "{name_header}{}",
            sort_indicator(ProcessSort::Name)
        ))
        .style(header_style(Some(ProcessSort::Name))),
        Cell::from(format!("CPU%{}", sort_indicator(ProcessSort::Cpu)))
            .style(header_style(Some(ProcessSort::Cpu))),
        Cell::from(format!("Mem{}", sort_indicator(ProcessSort::Memory)))
            .style(header_style(Some(ProcessSort::Memory))),
        Cell::from("Time").style(header_style(None)),
    ]);

    let name_max: usize = if app.show_cmd { 40 } else { 20 };
    let total_mem = app.sys.snapshot().mem_total;
    let rows: Vec<Row> = app
        .processes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let cpu_color = if p.cpu_usage >= 90.0 {
                Color::Red
            } else if p.cpu_usage >= 70.0 {
                Color::Rgb(255, 100, 0)
            } else if p.cpu_usage >= 50.0 {
                Color::Yellow
            } else if p.cpu_usage >= 20.0 {
                Color::Rgb(150, 255, 0)
            } else {
                Color::Green
            };
            let bg = if i % 2 == 1 {
                Color::Rgb(30, 30, 40) // subtle alternating row
            } else {
                Color::Reset
            };
            let state_color = match p.state {
                crate::app::ProcessState::Run => Color::Green,
                crate::app::ProcessState::Zombie => Color::Red,
                _ => Color::DarkGray,
            };
            let display_name = if app.show_cmd { &p.cmd } else { &p.name };
            let user_display = truncate_name(&p.user, 8);
            // Tree indentation prefix.
            let tree_prefix = if app.tree_mode && p.tree_depth > 0 {
                let indent = "  ".repeat(p.tree_depth.min(4));
                format!("{indent}|- ")
            } else {
                String::new()
            };
            let name_with_tree = format!(
                "{tree_prefix}{}",
                truncate_name(display_name, name_max.saturating_sub(tree_prefix.len()))
            );
            // Highlight filter match in name.
            let name_cell = if !app.process_filter.is_empty() {
                let lower = name_with_tree.to_lowercase();
                let filter = app.process_filter.to_lowercase();
                if let Some(pos) = lower.find(&filter) {
                    let before = &name_with_tree[..pos];
                    let matched = &name_with_tree[pos..pos + filter.len()];
                    let after = &name_with_tree[pos + filter.len()..];
                    Cell::from(Line::from(vec![
                        Span::raw(before.to_string()),
                        Span::styled(
                            matched.to_string(),
                            Style::default().fg(Color::Black).bg(Color::Yellow),
                        ),
                        Span::raw(after.to_string()),
                    ]))
                } else {
                    Cell::from(name_with_tree)
                }
            } else {
                Cell::from(name_with_tree)
            };
            Row::new(vec![
                Cell::from(p.state.label()).style(Style::default().fg(state_color)),
                Cell::from(format!("{}", p.pid)),
                Cell::from(user_display).style(Style::default().fg(Color::DarkGray)),
                name_cell,
                Cell::from(format!("{:.1}", p.cpu_usage)).style(Style::default().fg(cpu_color)),
                Cell::from(format_mem(p.memory_bytes, total_mem)),
                Cell::from(format_duration(p.run_time_secs))
                    .style(Style::default().fg(Color::DarkGray)),
            ])
            .style(Style::default().bg(bg))
        })
        .collect();

    let row_count = rows.len();

    let widths = [
        Constraint::Length(1),
        Constraint::Length(7),
        Constraint::Length(8),
        Constraint::Min(12),
        Constraint::Length(7),
        Constraint::Length(12),
        Constraint::Length(8),
    ];

    let sort_arrow = if app.sort_reverse {
        "\u{25b2}"
    } else {
        "\u{25bc}"
    };
    let sort_name = match app.process_sort {
        ProcessSort::Cpu => "CPU",
        ProcessSort::Memory => "Mem",
        ProcessSort::Pid => "PID",
        ProcessSort::Name => "Name",
    };

    let count_label = if !app.process_filter.is_empty() || app.filter_mode {
        format!("{}/{}", app.processes.len(), app.total_process_count)
    } else {
        format!("{}", app.processes.len())
    };
    let tree_tag = if app.tree_mode { " tree" } else { "" };
    let visible_cpu: f32 = app.processes.iter().map(|p| p.cpu_usage).sum();
    let cpu_tag = if visible_cpu >= 1.0 {
        format!(" {visible_cpu:.0}%")
    } else {
        String::new()
    };
    // Process state counters.
    let running = app
        .processes
        .iter()
        .filter(|p| matches!(p.state, crate::app::ProcessState::Run))
        .count();
    let sleeping = app
        .processes
        .iter()
        .filter(|p| matches!(p.state, crate::app::ProcessState::Sleep))
        .count();
    let zombie = app
        .processes
        .iter()
        .filter(|p| matches!(p.state, crate::app::ProcessState::Zombie))
        .count();
    let state_tag = if running > 0 || zombie > 0 {
        let mut parts = Vec::new();
        if running > 0 {
            parts.push(format!("R:{running}"));
        }
        if sleeping > 0 {
            parts.push(format!("S:{sleeping}"));
        }
        if zombie > 0 {
            parts.push(format!("Z:{zombie}"));
        }
        format!(" {}", parts.join(" "))
    } else {
        String::new()
    };
    let title = if app.filter_mode {
        format!(" Processes ({count_label}) [/{}|] ", app.process_filter)
    } else if !app.process_filter.is_empty() {
        format!(
            " Processes ({count_label}{cpu_tag}{state_tag}) [filter: {}] ",
            app.process_filter
        )
    } else {
        format!(" Processes ({count_label}{cpu_tag}{state_tag}) [sort: {sort_name}{sort_arrow}{tree_tag}] ")
    };

    // Scroll position indicator.
    let scroll_tag = if row_count > 0 {
        format!(" {}/{} ", app.process_scroll + 1, row_count)
    } else {
        String::new()
    };

    let border_color = if app.filter_mode {
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
                .title(title)
                .title_bottom(
                    Line::from(Span::styled(
                        scroll_tag,
                        Style::default().fg(Color::DarkGray),
                    ))
                    .right_aligned(),
                )
                .border_style(Style::default().fg(border_color)),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(60, 60, 80))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut state = TableState::default().with_selected(Some(app.process_scroll));
    frame.render_stateful_widget(table, area, &mut state);

    // Scrollbar.
    if row_count > 0 {
        let mut scrollbar_state = ScrollbarState::new(row_count).position(app.process_scroll);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn format_duration(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    if days > 0 {
        format!("{days}d{hours:02}h")
    } else if hours > 0 {
        format!("{hours}:{mins:02}:{s:02}")
    } else {
        format!("{mins}:{s:02}")
    }
}

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}...", &name[..max_len - 3])
    }
}

fn format_mem(bytes: u64, total: u64) -> String {
    let pct = if total > 0 {
        (bytes as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    if pct >= 1.0 {
        format!("{} {:.0}%", format_bytes(bytes), pct)
    } else {
        format_bytes(bytes)
    }
}

fn format_bytes(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    const MIB: u64 = 1024 * 1024;
    const KIB: u64 = 1024;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.0} MiB", bytes as f64 / MIB as f64)
    } else {
        format!("{:.0} KiB", bytes as f64 / KIB as f64)
    }
}
