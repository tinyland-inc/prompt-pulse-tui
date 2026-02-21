use ratatui::prelude::*;

use super::widgets;
use crate::app::App;

/// Dashboard tab: overview of everything.
/// Adaptive layout based on terminal width.
pub fn dashboard(frame: &mut Frame, area: Rect, app: &mut App) {
    let wide = area.width >= 120;

    if wide {
        // Wide: [left column: sys+host] [right column: tailscale+billing+claude]
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        // Left column: host info + sparklines + CPU/RAM gauges + disks.
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(9), // host info (hostname, kernel, cpu, uptime, IP/procs, shell, battery)
                Constraint::Length(5), // sparklines (CPU + MEM side by side)
                Constraint::Length(8), // CPU bars
                Constraint::Length(6), // memory
                Constraint::Min(4),    // disks
            ])
            .split(cols[0]);

        widgets::host::draw_host_info(frame, left[0], app);

        // Sparklines row: CPU, MEM, Swap.
        let spark_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35),
                Constraint::Percentage(35),
                Constraint::Percentage(30),
            ])
            .split(left[1]);
        widgets::sparkline::draw_cpu_sparkline(frame, spark_cols[0], app);
        widgets::sparkline::draw_mem_sparkline(frame, spark_cols[1], app);
        widgets::sparkline::draw_swap_sparkline(frame, spark_cols[2], app);

        widgets::cpu::draw_cpu_bars(frame, left[2], app);
        widgets::memory::draw_memory(frame, left[3], app);
        widgets::disk::draw_disks(frame, left[4], app);

        // Right column.
        let right_has_waifu = app.wants_waifu();
        if right_has_waifu {
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(50), // waifu
                    Constraint::Length(8),      // tailscale
                    Constraint::Min(5),         // billing/claude
                ])
                .split(cols[1]);

            widgets::waifu::draw_waifu(frame, right[0], app);
            widgets::tailscale::draw_tailscale(frame, right[1], app);

            let bottom = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(right[2]);

            widgets::claude::draw_claude(frame, bottom[0], app);
            widgets::billing_widget::draw_billing(frame, bottom[1], app);
        } else {
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(10), // tailscale
                    Constraint::Length(8),  // k8s
                    Constraint::Min(5),     // billing/claude
                ])
                .split(cols[1]);

            widgets::tailscale::draw_tailscale(frame, right[0], app);
            widgets::k8s::draw_k8s(frame, right[1], app);

            let bottom = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(right[2]);

            widgets::claude::draw_claude(frame, bottom[0], app);
            widgets::billing_widget::draw_billing(frame, bottom[1], app);
        }
    } else {
        // Narrow: single-column stack.
        let narrow_waifu = app.wants_waifu();
        let mut constraints = vec![
            Constraint::Length(8), // host
            Constraint::Length(4), // sparklines
            Constraint::Length(4), // memory
            Constraint::Length(4), // disks
        ];
        if narrow_waifu {
            constraints.push(Constraint::Length(10)); // waifu
        }
        constraints.push(Constraint::Length(6)); // tailscale
        constraints.push(Constraint::Min(3)); // billing

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let mut idx = 0;
        widgets::host::draw_host_info(frame, rows[idx], app);
        idx += 1;

        // Sparklines in narrow mode too.
        let spark_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[idx]);
        widgets::sparkline::draw_cpu_sparkline(frame, spark_cols[0], app);
        widgets::sparkline::draw_mem_sparkline(frame, spark_cols[1], app);
        idx += 1;

        widgets::memory::draw_memory(frame, rows[idx], app);
        idx += 1;
        widgets::disk::draw_disks(frame, rows[idx], app);
        idx += 1;

        if narrow_waifu {
            widgets::waifu::draw_waifu(frame, rows[idx], app);
            idx += 1;
        }

        widgets::tailscale::draw_tailscale(frame, rows[idx], app);
        idx += 1;
        widgets::billing_widget::draw_billing(frame, rows[idx], app);
    }
}

/// System tab: detailed CPU per-core, memory, disks, temps, network, processes.
pub fn system(frame: &mut Frame, area: Rect, app: &mut App) {
    let wide = area.width >= 120;

    if wide {
        // Wide: left column (sparklines, CPU, memory, disks+temps) | right column (net sparklines, processes, network)
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),  // CPU+MEM sparklines
                Constraint::Length(14), // CPU per-core mini sparklines
                Constraint::Length(6),  // memory + swap
                Constraint::Min(4),     // disks + temps split
            ])
            .split(cols[0]);

        // CPU + MEM + Swap + Load + Temp sparklines.
        let spark_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ])
            .split(left[0]);
        widgets::sparkline::draw_cpu_sparkline(frame, spark_cols[0], app);
        widgets::sparkline::draw_mem_sparkline(frame, spark_cols[1], app);
        widgets::sparkline::draw_swap_sparkline(frame, spark_cols[2], app);
        widgets::sparkline::draw_load_sparkline(frame, spark_cols[3], app);
        widgets::sparkline::draw_temp_sparkline(frame, spark_cols[4], app);
        widgets::sparkline::draw_cpu_per_core(frame, left[1], app);
        widgets::memory::draw_memory(frame, left[2], app);

        // Disks and temps side by side.
        let disk_temp = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(left[3]);
        widgets::disk::draw_disks(frame, disk_temp[0], app);
        widgets::temperature::draw_temperatures(frame, disk_temp[1], app);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),      // RX+TX sparklines
                Constraint::Percentage(55), // processes (scrollable)
                Constraint::Min(5),         // network table
            ])
            .split(cols[1]);

        // Network RX + TX sparklines.
        let net_spark_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(right[0]);
        widgets::sparkline::draw_net_rx_sparkline(frame, net_spark_cols[0], app);
        widgets::sparkline::draw_net_tx_sparkline(frame, net_spark_cols[1], app);

        widgets::processes::draw_processes(frame, right[1], app);
        widgets::network::draw_network(frame, right[2], app);
    } else {
        // Narrow: single stack
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),  // CPU+MEM+Temp sparklines
                Constraint::Length(10), // CPU per-core
                Constraint::Length(6),  // memory + swap
                Constraint::Length(4),  // net sparklines
                Constraint::Length(10), // processes
                Constraint::Length(6),  // disks
                Constraint::Length(6),  // temperatures
                Constraint::Min(4),     // network
            ])
            .split(area);

        let spark_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35),
                Constraint::Percentage(35),
                Constraint::Percentage(30),
            ])
            .split(chunks[0]);
        widgets::sparkline::draw_cpu_sparkline(frame, spark_cols[0], app);
        widgets::sparkline::draw_mem_sparkline(frame, spark_cols[1], app);
        widgets::sparkline::draw_temp_sparkline(frame, spark_cols[2], app);
        widgets::cpu::draw_cpu_bars(frame, chunks[1], app);
        widgets::memory::draw_memory(frame, chunks[2], app);

        // Net sparklines in narrow mode too.
        let net_spark_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[3]);
        widgets::sparkline::draw_net_rx_sparkline(frame, net_spark_cols[0], app);
        widgets::sparkline::draw_net_tx_sparkline(frame, net_spark_cols[1], app);

        widgets::processes::draw_processes(frame, chunks[4], app);
        widgets::disk::draw_disks(frame, chunks[5], app);
        widgets::temperature::draw_temperatures(frame, chunks[6], app);
        widgets::network::draw_network(frame, chunks[7], app);
    }
}

/// Network tab: net sparklines + interface table + Tailscale peers + K8s clusters.
pub fn network(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),      // net sparklines
            Constraint::Length(10),     // interface table
            Constraint::Percentage(40), // tailscale
            Constraint::Min(6),         // k8s
        ])
        .split(area);

    let net_spark_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);
    widgets::sparkline::draw_net_rx_sparkline(frame, net_spark_cols[0], app);
    widgets::sparkline::draw_net_tx_sparkline(frame, net_spark_cols[1], app);

    widgets::network::draw_network(frame, chunks[1], app);
    widgets::tailscale::draw_tailscale(frame, chunks[2], app);
    widgets::k8s::draw_k8s(frame, chunks[3], app);
}

/// Build tab: component SHAs, versions, and flake input revisions.
pub fn build(frame: &mut Frame, area: Rect, app: &mut App) {
    widgets::buildinfo::draw_build_info(frame, area, app);
}

/// Billing tab: Claude personal gauge + Claude API usage + cloud billing.
pub fn billing(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),      // Claude Personal gauge
            Constraint::Percentage(45), // Claude API usage
            Constraint::Percentage(45), // Cloud billing
        ])
        .split(area);

    widgets::claudepersonal::draw_claude_personal(frame, chunks[0], app);
    widgets::claude::draw_claude(frame, chunks[1], app);
    widgets::billing_widget::draw_billing(frame, chunks[2], app);
}

#[cfg(test)]
mod tests {
    use crate::app::{App, Tab};
    use crate::config::TuiConfig;
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};
    use std::path::PathBuf;

    /// Check if a rendered Buffer contains a substring.
    fn buffer_contains(buf: &Buffer, needle: &str) -> bool {
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        content.contains(needle)
    }

    fn render_app(width: u16, height: u16, app: &mut App) -> Buffer {
        app.on_resize(width, height);
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| crate::ui::draw(frame, app)).unwrap();
        terminal.backend().buffer().clone()
    }

    // --- Dashboard tab: wide layout ---

    #[test]
    fn wide_no_waifu_shows_kubernetes() {
        let mut app = App::test_new(TuiConfig::default());
        app.active_tab = Tab::Dashboard;
        let buf = render_app(160, 50, &mut app);
        assert!(
            buffer_contains(&buf, "Kubernetes"),
            "Kubernetes widget should appear when waifu is off"
        );
        assert!(
            !buffer_contains(&buf, "Waifu"),
            "Waifu should not appear when disabled"
        );
    }

    #[test]
    fn wide_with_waifu_shows_waifu() {
        let mut app = App::test_new(TuiConfig::default())
            .with_waifu_enabled()
            .with_waifu_images(vec![PathBuf::from("/fake/a.png")]);
        app.active_tab = Tab::Dashboard;
        let buf = render_app(160, 50, &mut app);
        assert!(
            buffer_contains(&buf, "Waifu"),
            "Waifu widget should appear when enabled with images"
        );
    }

    #[test]
    fn wide_with_waifu_hides_kubernetes() {
        let mut app = App::test_new(TuiConfig::default())
            .with_waifu_enabled()
            .with_waifu_images(vec![PathBuf::from("/fake/a.png")]);
        app.active_tab = Tab::Dashboard;
        let buf = render_app(160, 50, &mut app);
        assert!(
            !buffer_contains(&buf, "Kubernetes"),
            "Kubernetes should be hidden when waifu is shown"
        );
    }

    // --- Dashboard tab: narrow layout ---

    #[test]
    fn narrow_shows_host() {
        let mut app = App::test_new(TuiConfig::default());
        app.active_tab = Tab::Dashboard;
        let buf = render_app(80, 40, &mut app);
        assert!(
            buffer_contains(&buf, "Host"),
            "Host widget should appear in narrow layout"
        );
    }

    // --- System tab ---

    #[test]
    fn system_tab_shows_memory() {
        let mut app = App::test_new(TuiConfig::default());
        app.active_tab = Tab::System;
        let buf = render_app(160, 50, &mut app);
        assert!(
            buffer_contains(&buf, "Memory"),
            "System tab should show Memory widget"
        );
    }

    // --- Network tab ---

    #[test]
    fn network_tab_shows_tailscale() {
        let mut app = App::test_new(TuiConfig::default());
        app.active_tab = Tab::Network;
        let buf = render_app(160, 50, &mut app);
        assert!(
            buffer_contains(&buf, "Tailscale"),
            "Network tab should show Tailscale widget"
        );
    }

    #[test]
    fn network_tab_shows_kubernetes() {
        let mut app = App::test_new(TuiConfig::default());
        app.active_tab = Tab::Network;
        let buf = render_app(160, 50, &mut app);
        assert!(
            buffer_contains(&buf, "Kubernetes"),
            "Network tab should show Kubernetes widget"
        );
    }

    // --- Expanded mode ---

    #[test]
    fn expanded_mode_no_tab_bar() {
        let mut app = App::test_new(TuiConfig::default())
            .with_waifu_enabled()
            .with_waifu_images(vec![PathBuf::from("/fake/a.png")]);
        app.expanded = true;
        let buf = render_app(160, 50, &mut app);
        // In expanded mode, only waifu renders -- no tab bar with Dashboard/System/etc.
        assert!(
            !buffer_contains(&buf, "Dashboard"),
            "Tab bar should not appear in expanded mode"
        );
        assert!(
            buffer_contains(&buf, "Waifu"),
            "Waifu should render fullscreen in expanded mode"
        );
    }

    // --- All tabs render without panic ---

    #[test]
    fn all_tabs_render_wide() {
        for tab in Tab::ALL {
            let mut app = App::test_new(TuiConfig::default());
            app.active_tab = *tab;
            let _buf = render_app(160, 50, &mut app);
        }
    }

    #[test]
    fn all_tabs_render_narrow() {
        for tab in Tab::ALL {
            let mut app = App::test_new(TuiConfig::default());
            app.active_tab = *tab;
            let _buf = render_app(80, 30, &mut app);
        }
    }
}
