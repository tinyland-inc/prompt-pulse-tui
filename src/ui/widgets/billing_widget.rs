use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Gauge, Paragraph, Row, Table};

use crate::app::App;

pub fn draw_billing(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue));

    match &app.billing {
        Some(billing) => {
            let title = format!(
                " Cloud Billing (${:.2}/mo) ",
                billing.total_monthly_usd
            );

            let inner = block.clone().title(title.clone());

            if billing.budget_usd > 0.0 && area.height >= 5 {
                // Show budget gauge + provider table.
                let inner_area = inner.inner(area);
                frame.render_widget(inner, area);

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(2), Constraint::Min(2)])
                    .split(inner_area);

                let budget_ratio = (billing.budget_percent / 100.0).clamp(0.0, 1.0);
                let budget_color = if billing.budget_percent >= 90.0 {
                    Color::Red
                } else if billing.budget_percent >= 70.0 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                let gauge = Gauge::default()
                    .gauge_style(Style::default().fg(budget_color))
                    .ratio(budget_ratio)
                    .label(format!(
                        "${:.2} / ${:.2} ({:.0}%)",
                        billing.total_monthly_usd, billing.budget_usd, billing.budget_percent
                    ));
                frame.render_widget(gauge, chunks[0]);

                draw_providers(frame, chunks[1], billing);
            } else {
                let inner_area = inner.inner(area);
                frame.render_widget(inner, area);
                draw_providers(frame, inner_area, billing);
            }
        }
        None => {
            let paragraph = Paragraph::new("No billing data")
                .style(Style::default().fg(Color::DarkGray))
                .block(block.title(" Cloud Billing "));
            frame.render_widget(paragraph, area);
        }
    }
}

fn draw_providers(
    frame: &mut Frame,
    area: Rect,
    billing: &crate::data::BillingReport,
) {
    let rows: Vec<Row> = billing
        .providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let color = if p.connected {
                Color::Green
            } else {
                Color::Red
            };
            let bg = if i % 2 == 1 { Color::Rgb(30, 30, 40) } else { Color::Reset };
            Row::new(vec![
                p.name.clone(),
                format!("${:.2}", p.month_to_date),
                format!("{} resources", p.resources.len()),
            ])
            .style(Style::default().fg(color).bg(bg))
        })
        .collect();

    if !rows.is_empty() {
        let widths = [
            Constraint::Min(12),
            Constraint::Length(12),
            Constraint::Length(14),
        ];
        let table = Table::new(rows, widths);
        frame.render_widget(table, area);
    }
}
