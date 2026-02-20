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
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // host
                Constraint::Length(4), // sparklines
                Constraint::Length(4), // memory
                Constraint::Length(4), // disks
                Constraint::Length(6), // tailscale
                Constraint::Min(3),    // billing
            ])
            .split(area);

        widgets::host::draw_host_info(frame, rows[0], app);

        // Sparklines in narrow mode too.
        let spark_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);
        widgets::sparkline::draw_cpu_sparkline(frame, spark_cols[0], app);
        widgets::sparkline::draw_mem_sparkline(frame, spark_cols[1], app);

        widgets::memory::draw_memory(frame, rows[2], app);
        widgets::disk::draw_disks(frame, rows[3], app);
        widgets::tailscale::draw_tailscale(frame, rows[4], app);
        widgets::billing_widget::draw_billing(frame, rows[5], app);
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
