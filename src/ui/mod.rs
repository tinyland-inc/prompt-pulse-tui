pub mod layout;
pub mod widgets;

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, Tab};

/// Top-level draw: tab bar + active tab content + help bar + optional help overlay.
/// In expand mode, renders the waifu widget fullscreen (no tab bar or help bar).
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Expand mode: fullscreen waifu.
    if app.expanded {
        widgets::waifu::draw_waifu(frame, area, app);
        return;
    }

    // Split into tab bar (3 lines) + content + help bar (1 line).
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    widgets::tabs::draw_tabs(frame, chunks[0], app);

    match app.active_tab {
        Tab::Dashboard => layout::dashboard(frame, chunks[1], app),
        Tab::System => layout::system(frame, chunks[1], app),
        Tab::Network => layout::network(frame, chunks[1], app),
        Tab::Billing => layout::billing(frame, chunks[1], app),
    }

    widgets::help::draw_help_bar(frame, chunks[2], app);

    // Help overlay (centered popup).
    if app.show_help {
        draw_help_overlay(frame, area, app.help_tab);
    }
}

/// Render a keybinding line: fixed-width key + description.
fn help_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {:<18}", key), Style::default().fg(Color::Yellow)),
        Span::raw(format!("  {}", desc)),
    ])
}

/// Section header in cyan bold.
fn help_section(title: &str) -> Line<'_> {
    Line::from(Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
}

fn help_tab_tui<'a>() -> Vec<Line<'a>> {
    vec![
        help_section("Navigation"),
        Line::from(""),
        help_line("Tab / Right", "Next tab"),
        help_line("Shift-Tab / Left", "Previous tab"),
        help_line("1-4", "Jump to tab"),
        help_line("Space", "Freeze/resume data"),
        Line::from(""),
        help_section("Process Table (System tab)"),
        Line::from(""),
        help_line("j/k / Up/Down", "Scroll processes"),
        help_line("g / Home", "Jump to top"),
        help_line("G / End", "Jump to bottom"),
        help_line("/", "Filter by name/PID"),
        help_line("c / m / p / n", "Sort: CPU/Mem/PID/Name"),
        help_line("r", "Reverse sort order"),
        help_line("e", "Toggle full command"),
        help_line("t", "Toggle tree view"),
        help_line("PgUp / PgDn", "Jump 10 processes"),
        help_line("dd", "Kill process (TERM)"),
        help_line("D", "Force kill (KILL)"),
        Line::from(""),
        help_section("Waifu (Dashboard tab)"),
        Line::from(""),
        help_line("n / p", "Next / previous image"),
        help_line("r", "Random image"),
        help_line("i", "Toggle info overlay"),
        Line::from(""),
        help_section("Display"),
        Line::from(""),
        help_line("+ / -", "Adjust refresh (250ms-5s)"),
        help_line("?", "This help"),
        help_line("q / Esc", "Quit"),
    ]
}

fn help_tab_shell<'a>() -> Vec<Line<'a>> {
    vec![
        help_section("Shell Keybindings"),
        Line::from(""),
        help_line("Ctrl+P", "Launch TUI dashboard"),
        help_line("Ctrl+W", "Launch waifu viewer"),
        help_line("pp", "prompt-pulse alias"),
        help_line("pp-tui", "prompt-pulse-tui alias"),
        help_line("pp-status", "Daemon health check"),
        help_line("pp-start", "Start daemon"),
        help_line("pp-stop", "Stop daemon"),
        help_line("pp-banner", "Show text banner"),
        Line::from(""),
        help_section("Starship Prompt"),
        Line::from(""),
        help_line("Claude segment", "Purple - API usage & burn rate"),
        help_line("Billing segment", "Cyan - CIVO + DO costs"),
        help_line("Infra segment", "Green - Tailscale + K8s"),
    ]
}

fn help_tab_lab<'a>() -> Vec<Line<'a>> {
    vec![
        help_section("Deployment"),
        Line::from(""),
        help_line("just deploy <host>", "Full deployment"),
        help_line("just nix-switch", "Nix config only"),
        help_line("just check <host>", "Dry-run with diff"),
        Line::from(""),
        help_section("Diagnostics"),
        Line::from(""),
        help_line("just doctor", "Run diagnostic checks"),
        help_line("lab_status", "Show API key status"),
        help_line("tinyland_build", "Show build info"),
        Line::from(""),
        help_section("Development"),
        Line::from(""),
        help_line("just test", "Run all tests"),
        help_line("just molecule <role>", "Molecule test role"),
        help_line("just test-pbt", "Property-based tests"),
        help_line("just nix-check", "Nix flake check"),
        help_line("jb-dev", "DevContainer launcher"),
    ]
}

fn help_tab_starship<'a>() -> Vec<Line<'a>> {
    vec![
        help_section("Starship Modules"),
        Line::from(""),
        help_line("custom.claude", "Claude API usage (purple)"),
        help_line("custom.billing", "Cloud billing (cyan)"),
        help_line("custom.infra", "Infra status (green)"),
        Line::from(""),
        help_section("Themes"),
        Line::from(""),
        help_line("ultra-minimal", "Directory only, fastest"),
        help_line("minimal", "Dir + git, clean"),
        help_line("full", "Languages, duration, etc."),
        help_line("plain", "No special chars"),
        help_line("monitoring", "With prompt-pulse modules"),
        Line::from(""),
        help_section("Configuration"),
        Line::from(""),
        help_line("~/.config/starship", "Managed by Nix"),
        help_line("nix/hosts/base.nix", "Theme selection"),
        help_line("starship.nix", "Module definitions"),
    ]
}

fn draw_help_overlay(frame: &mut Frame, area: Rect, help_tab: usize) {
    let popup_width = 56u16.min(area.width.saturating_sub(4));
    let popup_height = 34u16.min(area.height.saturating_sub(4));

    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    // Tab selector line.
    let tab_names = ["TUI", "Shell", "Lab", "Starship"];
    let tab_spans: Vec<Span> = tab_names.iter().enumerate().map(|(i, name)| {
        if i == help_tab {
            Span::styled(format!(" [{}] {} ", i + 1, name), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(format!("  {}  {} ", i + 1, name), Style::default().fg(Color::DarkGray))
        }
    }).collect();

    let mut lines = vec![
        Line::from(tab_spans),
        Line::from(""),
    ];

    // Tab content.
    let content = match help_tab {
        0 => help_tab_tui(),
        1 => help_tab_shell(),
        2 => help_tab_lab(),
        3 => help_tab_starship(),
        _ => help_tab_tui(),
    };
    lines.extend(content);

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Left/Right or 1-4 to switch tabs. Any other key to close.",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Keymap Reference (?) ")
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, popup_area);
}
