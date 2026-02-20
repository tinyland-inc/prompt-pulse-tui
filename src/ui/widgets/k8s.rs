use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table};

use crate::app::App;

pub fn draw_k8s(frame: &mut Frame, area: Rect, app: &App) {
    match &app.k8s {
        Some(k8s) if !k8s.clusters.is_empty() => {
            // Aggregate health summary for title.
            let total_nodes: usize = k8s.clusters.iter().map(|c| c.nodes.len()).sum();
            let total_pods: i32 = k8s.clusters.iter().map(|c| c.total_pods).sum();
            let total_failed: i32 = k8s.clusters.iter().map(|c| c.failed_pods).sum();
            let health_tag = if total_failed > 0 {
                format!(" [{total_failed} failed]")
            } else {
                String::new()
            };
            let title_color = if total_failed > 0 { Color::Yellow } else { Color::Blue };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(" Kubernetes ({} clusters, {total_nodes}n/{total_pods}p{health_tag}) ", k8s.clusters.len()))
                .border_style(Style::default().fg(title_color));

            let header = Row::new(vec!["Cluster", "Nodes", "Pods", "Status"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

            let rows: Vec<Row> = k8s
                .clusters
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let status_color = if !c.connected {
                        Color::Red
                    } else if c.failed_pods > 0 {
                        Color::Yellow
                    } else {
                        Color::Green
                    };
                    let status = if !c.connected {
                        "disconnected".to_string()
                    } else if c.failed_pods > 0 {
                        format!("{} failed", c.failed_pods)
                    } else {
                        "healthy".to_string()
                    };
                    let bg = if i % 2 == 1 { Color::Rgb(30, 30, 40) } else { Color::Reset };
                    Row::new(vec![
                        c.context.clone(),
                        format!("{}", c.nodes.len()),
                        format!("{}/{}", c.running_pods, c.total_pods),
                        status,
                    ])
                    .style(Style::default().fg(status_color).bg(bg))
                })
                .collect();

            let widths = [
                Constraint::Min(20),
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Length(14),
            ];

            let table = Table::new(rows, widths).header(header).block(block);
            frame.render_widget(table, area);
        }
        _ => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Kubernetes ")
                .border_style(Style::default().fg(Color::Blue));
            let paragraph = Paragraph::new("No cluster data")
                .style(Style::default().fg(Color::DarkGray))
                .block(block);
            frame.render_widget(paragraph, area);
        }
    }
}
