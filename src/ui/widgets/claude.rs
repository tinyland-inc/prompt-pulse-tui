use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table};

use crate::app::App;

pub fn draw_claude(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue));

    match &app.claude {
        Some(claude) => {
            // Aggregate token counts across all accounts.
            let total_in: i64 = claude.accounts.iter().map(|a| a.current_month.input_tokens).sum();
            let total_out: i64 = claude.accounts.iter().map(|a| a.current_month.output_tokens).sum();
            let token_tag = if total_in > 0 || total_out > 0 {
                format!(" {}in/{}out", format_tokens(total_in), format_tokens(total_out))
            } else {
                String::new()
            };
            let title = format!(" Claude (${:.2}{token_tag}) ", claude.total_cost_usd);

            if area.height >= 6 && !claude.accounts.is_empty() {
                let header = Row::new(vec!["Account", "Cost", "Tokens", "Models"])
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

                let rows: Vec<Row> = claude
                    .accounts
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        let models: String = a
                            .models
                            .iter()
                            .take(3)
                            .map(|m| m.model.clone())
                            .collect::<Vec<_>>()
                            .join(", ");
                        let color = if a.connected {
                            Color::Green
                        } else {
                            Color::Red
                        };
                        let bg = if i % 2 == 1 { Color::Rgb(30, 30, 40) } else { Color::Reset };
                        let acct_tokens = a.current_month.input_tokens + a.current_month.output_tokens;
                        Row::new(vec![
                            a.name.clone(),
                            format!("${:.2}", a.current_month.cost_usd),
                            format_tokens(acct_tokens),
                            models,
                        ])
                        .style(Style::default().fg(color).bg(bg))
                    })
                    .collect();

                let widths = [
                    Constraint::Min(12),
                    Constraint::Length(10),
                    Constraint::Length(8),
                    Constraint::Min(16),
                ];

                let table = Table::new(rows, widths)
                    .header(header)
                    .block(block.title(title));
                frame.render_widget(table, area);
            } else {
                let text = format!("Total: ${:.2}", claude.total_cost_usd);
                let paragraph = Paragraph::new(text)
                    .style(Style::default().fg(Color::Green))
                    .block(block.title(title));
                frame.render_widget(paragraph, area);
            }
        }
        None => {
            let paragraph = Paragraph::new("No Claude data")
                .style(Style::default().fg(Color::DarkGray))
                .block(block.title(" Claude "));
            frame.render_widget(paragraph, area);
        }
    }
}

fn format_tokens(tokens: i64) -> String {
    let t = tokens.unsigned_abs();
    if t >= 1_000_000 {
        format!("{:.1}M", t as f64 / 1_000_000.0)
    } else if t >= 1_000 {
        format!("{:.0}K", t as f64 / 1_000.0)
    } else {
        format!("{t}")
    }
}
