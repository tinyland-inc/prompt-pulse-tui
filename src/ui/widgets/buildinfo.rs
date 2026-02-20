use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::App;
use crate::data::buildinfo::TuiBuildInfo;

pub fn draw_build_info(frame: &mut Frame, area: Rect, app: &App) {
    let build = TuiBuildInfo::current();
    let versions = &app.component_versions;

    let mut lines: Vec<Line> = Vec::new();

    // Section: TUI Binary
    lines.push(section_header("TUI Binary"));
    lines.push(kv_line("Version", format!("v{}", build.version)));
    lines.push(kv_line("Git SHA", build.sha_display()));

    lines.push(Line::from(""));

    // Section: Go Daemon
    lines.push(section_header("Go Daemon"));
    if let Some(ref daemon) = versions.daemon {
        lines.push(kv_line("Version", daemon.version.clone()));
        let sha = if daemon.git_sha.len() > 8 {
            daemon.git_sha[..8].to_string()
        } else {
            daemon.git_sha.clone()
        };
        lines.push(kv_line("Git SHA", sha));
        lines.push(kv_line("Go", daemon.go_version.clone()));
    } else {
        lines.push(dim_line("  daemon not detected"));
    }

    lines.push(Line::from(""));

    // Section: Nix Environment
    lines.push(section_header("Nix Environment"));
    if let Some(ref gen) = versions.hm_generation {
        lines.push(kv_line("HM Generation", gen.clone()));
    }
    if let Some(ref nix_ver) = versions.nix_version {
        lines.push(kv_line("Nix", nix_ver.clone()));
    }

    if !versions.flake_inputs.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Flake Inputs"));
        for input in &versions.flake_inputs {
            lines.push(kv_line(&input.name, input.rev.clone()));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Build Info ")
        .border_style(Style::default().fg(Color::Magenta));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {title}"),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

fn kv_line(key: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("    {:<18}", key),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(value, Style::default().fg(Color::White)),
    ])
}

fn dim_line(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default().fg(Color::DarkGray),
    ))
}
