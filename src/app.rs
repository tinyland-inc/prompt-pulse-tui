use std::collections::VecDeque;
use std::time::Instant;

use anyhow::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use image::imageops::FilterType;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

use crate::config::TuiConfig;
use crate::data::claudepersonal::ClaudePersonalReport;
use crate::data::waifu::WaifuEntry;
use crate::data::waifu_client::FetchResult;
use crate::data::{
    self, BillingReport, CacheReader, ClaudeUsage, K8sStatus, SysMetrics, TailscaleStatus,
};

use tokio::sync::mpsc;

/// Maximum number of historical data points for sparklines (~60s at 1s interval).
const HISTORY_LEN: usize = 60;

/// Active tab in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    System,
    Network,
    Billing,
    Build,
}

impl Tab {
    pub const ALL: &[Tab] = &[
        Tab::Dashboard,
        Tab::System,
        Tab::Network,
        Tab::Billing,
        Tab::Build,
    ];

    pub fn title(&self) -> &str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::System => "System",
            Tab::Network => "Network",
            Tab::Billing => "Billing",
            Tab::Build => "Build",
        }
    }
}

/// Process info for the process table widget.
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub cmd: String,
    pub user: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub state: ProcessState,
    pub run_time_secs: u64,
    pub tree_depth: usize, // 0 = root, 1+ = child depth
}

/// Process running state.
#[derive(Debug, Clone, Copy)]
pub enum ProcessState {
    Run,
    Sleep,
    Idle,
    Zombie,
    Unknown,
}

impl ProcessState {
    pub fn label(&self) -> &str {
        match self {
            Self::Run => "R",
            Self::Sleep => "S",
            Self::Idle => "I",
            Self::Zombie => "Z",
            Self::Unknown => "?",
        }
    }
}

/// Process sort column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessSort {
    Cpu,
    Memory,
    Pid,
    Name,
}

/// Application state.
pub struct App {
    pub cfg: TuiConfig,
    pub active_tab: Tab,
    pub term_width: u16,
    pub term_height: u16,
    pub show_help: bool,
    pub help_tab: usize, // 0=TUI, 1=Shell, 2=Lab, 3=Starship
    pub frozen: bool,

    // Process filter (btm-style '/' search).
    pub process_filter: String,
    pub filter_mode: bool,

    // Adjustable refresh interval (500ms to 5000ms).
    pub refresh_ms: u64,

    // Process expanded command toggle ('e' key).
    pub show_cmd: bool,

    // Process tree view toggle ('t' key).
    pub tree_mode: bool,

    // Live system data (collected in-process).
    pub sys: SysMetrics,

    // Historical data for sparklines (newest at back).
    pub cpu_history: VecDeque<f64>,
    pub cpu_per_core_history: Vec<VecDeque<f64>>,
    pub mem_history: VecDeque<f64>,
    pub swap_history: VecDeque<f64>,
    pub net_rx_history: VecDeque<f64>,
    pub net_tx_history: VecDeque<f64>,
    pub load_history: VecDeque<f64>,
    pub temp_history: VecDeque<f64>, // max temperature over last 60s

    // Process kill: double-d (btm-style) confirmation.
    pub pending_kill: Option<Instant>, // timestamp of first 'd' press

    // Top processes by CPU usage.
    pub processes: Vec<ProcessInfo>,
    pub process_sort: ProcessSort,
    pub sort_reverse: bool,
    pub process_scroll: usize,
    pub total_process_count: usize, // unfiltered count for title display

    // Cached data from Go daemon.
    pub tailscale: Option<TailscaleStatus>,
    pub claude: Option<ClaudeUsage>,
    pub billing: Option<BillingReport>,
    pub k8s: Option<K8sStatus>,

    // Waifu image rendering state (ratatui-image StatefulProtocol).
    pub waifu_state: Option<StatefulProtocol>,

    // Waifu in-memory gallery (live-fetched, no disk cache).
    pub waifu_gallery: Vec<WaifuEntry>,
    pub waifu_index: i32,
    pub waifu_show_info: bool,
    pub waifu_name: String,
    pub waifu_fetching: bool, // true while an async fetch is in flight

    // Claude personal plan usage (read from daemon state file).
    pub claude_personal: Option<ClaudePersonalReport>,

    // Expand mode: fullscreen single widget (e.g. --expand waifu).
    pub expanded: bool,

    // Image picker for protocol detection.
    pub picker: Picker,

    // Process list handle (refreshed separately from sys).
    proc_sys: sysinfo::System,
    users: sysinfo::Users,

    cache_reader: CacheReader,
    last_cache_read: Instant,
    last_sys_refresh: Instant,

    // Build/component version info (read once at startup).
    pub component_versions: data::buildinfo::ComponentVersions,

    // Channel for receiving live-fetched waifu results (None = fetch failed).
    waifu_fetch_rx: mpsc::Receiver<Option<FetchResult>>,
    waifu_fetch_tx: mpsc::Sender<Option<FetchResult>>,
}

impl App {
    pub async fn new(
        cfg: TuiConfig,
        picker: Picker,
        expand_widget: Option<String>,
    ) -> Result<Self> {
        let cache_reader = CacheReader::new(cfg.cache_dir());
        let sys = SysMetrics::collect();

        // Initial cache read.
        let tailscale = cache_reader.read_tailscale();
        let claude = cache_reader.read_claude();
        let billing = cache_reader.read_billing();
        let k8s = cache_reader.read_k8s();
        let claude_personal = cache_reader.read_claude_personal();

        // Waifu gallery starts empty — images are fetched live from the web service.
        let waifu_gallery: Vec<WaifuEntry> = Vec::new();
        let waifu_state: Option<StatefulProtocol> = None;
        let waifu_index: i32 = -1;
        let waifu_name = String::new();

        // Expand mode from CLI flag.
        let expanded = expand_widget.as_deref() == Some("waifu");

        // Collect build/component version info (once at startup).
        let component_versions = data::buildinfo::collect_versions(&cfg);

        // Channel for async waifu fetch results.
        let (waifu_fetch_tx, waifu_fetch_rx) = mpsc::channel(4);

        // Initialize process system with CPU refresh for usage tracking.
        let mut proc_sys = sysinfo::System::new();
        proc_sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        let users = sysinfo::Users::new_with_refreshed_list();

        let mut result = Ok(Self {
            cfg,
            active_tab: Tab::Dashboard,
            term_width: 0,
            term_height: 0,
            show_help: false,
            help_tab: 0,
            frozen: false,
            process_filter: String::new(),
            filter_mode: false,
            refresh_ms: 1000,
            show_cmd: false,
            tree_mode: false,
            sys,
            cpu_history: VecDeque::with_capacity(HISTORY_LEN),
            cpu_per_core_history: Vec::new(),
            mem_history: VecDeque::with_capacity(HISTORY_LEN),
            swap_history: VecDeque::with_capacity(HISTORY_LEN),
            net_rx_history: VecDeque::with_capacity(HISTORY_LEN),
            net_tx_history: VecDeque::with_capacity(HISTORY_LEN),
            load_history: VecDeque::with_capacity(HISTORY_LEN),
            temp_history: VecDeque::with_capacity(HISTORY_LEN),
            pending_kill: None,
            processes: Vec::new(),
            process_sort: ProcessSort::Cpu,
            sort_reverse: false,
            process_scroll: 0,
            total_process_count: 0,
            tailscale,
            claude,
            billing,
            k8s,
            waifu_state,
            waifu_gallery,
            waifu_index,
            waifu_show_info: false,
            waifu_name,
            waifu_fetching: false,
            claude_personal,
            expanded,
            picker,
            proc_sys,
            users,
            cache_reader,
            last_cache_read: Instant::now(),
            last_sys_refresh: Instant::now(),
            component_versions,
            waifu_fetch_rx,
            waifu_fetch_tx,
        });

        // Auto-fetch waifu from live service on launch.
        if let Ok(ref mut app) = result {
            if app.cfg.image.waifu_enabled && app.cfg.waifu_endpoint().is_some() {
                app.waifu_fetch_live();
            }
        }

        result
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        use crossterm::event::KeyCode;

        // Process filter input mode: capture typed characters.
        if self.filter_mode {
            match key.code {
                KeyCode::Esc => {
                    self.filter_mode = false;
                    self.process_filter.clear();
                }
                KeyCode::Enter => {
                    self.filter_mode = false;
                }
                KeyCode::Backspace => {
                    self.process_filter.pop();
                }
                KeyCode::Char(c) => {
                    self.process_filter.push(c);
                    self.process_scroll = 0;
                }
                _ => {}
            }
            return;
        }

        // Toggle help overlay.
        if key.code == KeyCode::Char('?') {
            self.show_help = !self.show_help;
            if self.show_help {
                self.help_tab = 0;
            }
            return;
        }
        // Navigate within help overlay if showing.
        if self.show_help {
            match key.code {
                KeyCode::Right | KeyCode::Tab => self.help_tab = (self.help_tab + 1) % 4,
                KeyCode::Left | KeyCode::BackTab => self.help_tab = (self.help_tab + 3) % 4,
                KeyCode::Char('1') => self.help_tab = 0,
                KeyCode::Char('2') => self.help_tab = 1,
                KeyCode::Char('3') => self.help_tab = 2,
                KeyCode::Char('4') => self.help_tab = 3,
                _ => self.show_help = false, // Any other key dismisses
            }
            return;
        }

        // Expand mode: Esc exits, waifu keys work, everything else ignored.
        if self.expanded {
            match key.code {
                KeyCode::Esc => self.expanded = false,
                KeyCode::Char('n') => self.waifu_navigate(1),
                KeyCode::Char('p') => self.waifu_navigate(-1),
                KeyCode::Char('r') => self.waifu_random(),
                KeyCode::Char('i') => self.waifu_show_info = !self.waifu_show_info,
                KeyCode::Char('f') => self.waifu_fetch_live(),
                _ => {}
            }
            return;
        }

        // Dashboard tab: waifu keys when waifu area is visible.
        if self.active_tab == Tab::Dashboard && self.wants_waifu() {
            match key.code {
                KeyCode::Char('n') if self.has_waifu() => {
                    self.waifu_navigate(1);
                    return;
                }
                KeyCode::Char('p') if self.has_waifu() => {
                    self.waifu_navigate(-1);
                    return;
                }
                KeyCode::Char('r') if self.has_waifu() => {
                    self.waifu_random();
                    return;
                }
                KeyCode::Char('i') if self.has_waifu() => {
                    self.waifu_show_info = !self.waifu_show_info;
                    return;
                }
                KeyCode::Char('f') => {
                    self.waifu_fetch_live();
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            // Freeze toggle (pause data collection).
            KeyCode::Char(' ') => self.frozen = !self.frozen,
            // Process filter (btm-style '/' search).
            KeyCode::Char('/') => {
                self.filter_mode = true;
                self.process_filter.clear();
            }
            KeyCode::Tab | KeyCode::Right => self.next_tab(),
            KeyCode::BackTab | KeyCode::Left => self.prev_tab(),
            KeyCode::Char('1') => self.active_tab = Tab::Dashboard,
            KeyCode::Char('2') => self.active_tab = Tab::System,
            KeyCode::Char('3') => self.active_tab = Tab::Network,
            KeyCode::Char('4') => self.active_tab = Tab::Billing,
            KeyCode::Char('5') => self.active_tab = Tab::Build,
            // Process table navigation (System tab).
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.processes.is_empty() {
                    self.process_scroll =
                        (self.process_scroll + 1).min(self.processes.len().saturating_sub(1));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.process_scroll = self.process_scroll.saturating_sub(1);
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.process_scroll = 0;
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.process_scroll = self.processes.len().saturating_sub(1);
            }
            // Sort toggle: c=CPU, m=Memory, p=PID, n=Name.
            // n/p/r are context-sensitive: on Dashboard with waifu they're handled above.
            KeyCode::Char('c') => self.process_sort = ProcessSort::Cpu,
            KeyCode::Char('m') => self.process_sort = ProcessSort::Memory,
            KeyCode::Char('p') => self.process_sort = ProcessSort::Pid,
            KeyCode::Char('n') => self.process_sort = ProcessSort::Name,
            KeyCode::Char('r') => self.sort_reverse = !self.sort_reverse,
            // Page up/down for process table.
            KeyCode::PageDown => {
                if !self.processes.is_empty() {
                    self.process_scroll =
                        (self.process_scroll + 10).min(self.processes.len().saturating_sub(1));
                }
            }
            KeyCode::PageUp => {
                self.process_scroll = self.process_scroll.saturating_sub(10);
            }
            // Toggle full command display for processes.
            KeyCode::Char('e') => self.show_cmd = !self.show_cmd,
            // Toggle tree view for processes.
            KeyCode::Char('t') => self.tree_mode = !self.tree_mode,
            // Process kill: 'dd' sends SIGTERM (btm-style double-key).
            KeyCode::Char('d') => {
                if let Some(first_press) = self.pending_kill {
                    if first_press.elapsed().as_millis() < 500 {
                        self.kill_selected_process(false);
                    }
                    self.pending_kill = None;
                } else {
                    self.pending_kill = Some(Instant::now());
                }
            }
            // 'D' (shift-d) sends SIGKILL immediately.
            KeyCode::Char('D') => {
                self.kill_selected_process(true);
            }
            // Adjustable refresh rate.
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.refresh_ms = (self.refresh_ms.saturating_sub(250)).max(250);
            }
            KeyCode::Char('-') => {
                self.refresh_ms = (self.refresh_ms + 250).min(5000);
            }
            _ => {
                // Any other key cancels pending kill.
                self.pending_kill = None;
            }
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        use crossterm::event::MouseEventKind;
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if self.active_tab == Tab::System && !self.processes.is_empty() {
                    self.process_scroll =
                        (self.process_scroll + 3).min(self.processes.len().saturating_sub(1));
                }
            }
            MouseEventKind::ScrollUp => {
                if self.active_tab == Tab::System {
                    self.process_scroll = self.process_scroll.saturating_sub(3);
                }
            }
            // Click in the top 3 rows = tab bar region.
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                if mouse.row < 3 {
                    // Rough tab hit detection: divide width evenly.
                    let tab_count = Tab::ALL.len() as u16;
                    let tab_width = self.term_width / tab_count.max(1);
                    let idx = (mouse.column / tab_width.max(1)) as usize;
                    if idx < Tab::ALL.len() {
                        self.active_tab = Tab::ALL[idx];
                    }
                }
            }
            _ => {}
        }
    }

    pub fn on_resize(&mut self, w: u16, h: u16) {
        let old_w = self.term_width;
        let old_h = self.term_height;
        self.term_width = w;
        self.term_height = h;

        // Re-create waifu protocol when terminal size changes substantially,
        // so the image is pre-scaled to fill the new widget area.
        if self.waifu_index >= 0 && (old_w != w || old_h != h) {
            self.waifu_load_at(self.waifu_index as usize);
        }
    }

    /// Called every tick (~250ms). Refresh real-time system data and
    /// periodically re-read daemon cache files.
    pub async fn tick(&mut self) {
        // Always poll for async fetch results, even when frozen.
        self.poll_waifu_fetch();

        // Skip all data collection when frozen.
        if self.frozen {
            return;
        }

        let now = Instant::now();

        // Refresh system metrics at adjustable rate.
        if now.duration_since(self.last_sys_refresh).as_millis() >= self.refresh_ms as u128 {
            self.sys.refresh();

            // Record history for sparklines.
            let snap = self.sys.snapshot();
            if self.cpu_history.len() >= HISTORY_LEN {
                self.cpu_history.pop_front();
            }
            self.cpu_history.push_back(snap.cpu_total as f64);

            // Per-core history.
            if self.cpu_per_core_history.len() != snap.cpu_usage.len() {
                self.cpu_per_core_history =
                    vec![VecDeque::with_capacity(HISTORY_LEN); snap.cpu_usage.len()];
            }
            for (i, &usage) in snap.cpu_usage.iter().enumerate() {
                if self.cpu_per_core_history[i].len() >= HISTORY_LEN {
                    self.cpu_per_core_history[i].pop_front();
                }
                self.cpu_per_core_history[i].push_back(usage as f64);
            }

            if self.mem_history.len() >= HISTORY_LEN {
                self.mem_history.pop_front();
            }
            self.mem_history.push_back(snap.mem_percent);

            // Swap history.
            let swap_pct = if snap.swap_total > 0 {
                (snap.swap_used as f64 / snap.swap_total as f64) * 100.0
            } else {
                0.0
            };
            if self.swap_history.len() >= HISTORY_LEN {
                self.swap_history.pop_front();
            }
            self.swap_history.push_back(swap_pct);

            // Load average (1-min) history.
            if self.load_history.len() >= HISTORY_LEN {
                self.load_history.pop_front();
            }
            self.load_history.push_back(snap.load_avg[0]);

            // Record max temperature for sparkline.
            let max_temp = snap
                .temperatures
                .iter()
                .map(|t| t.temp_c)
                .fold(0.0f32, f32::max);
            if self.temp_history.len() >= HISTORY_LEN {
                self.temp_history.pop_front();
            }
            self.temp_history.push_back(max_temp as f64);

            // Record aggregate network rate for sparklines.
            let total_rx: u64 = snap.networks.iter().map(|n| n.rx_rate).sum();
            let total_tx: u64 = snap.networks.iter().map(|n| n.tx_rate).sum();
            if self.net_rx_history.len() >= HISTORY_LEN {
                self.net_rx_history.pop_front();
            }
            self.net_rx_history.push_back(total_rx as f64);
            if self.net_tx_history.len() >= HISTORY_LEN {
                self.net_tx_history.pop_front();
            }
            self.net_tx_history.push_back(total_tx as f64);

            // Refresh process list and collect top 50 (scrollable).
            self.proc_sys
                .refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            self.total_process_count = self
                .proc_sys
                .processes()
                .values()
                .filter(|p| p.cpu_usage() > 0.0)
                .count();
            let filter_lower = self.process_filter.to_lowercase();
            let mut procs: Vec<ProcessInfo> = self
                .proc_sys
                .processes()
                .values()
                .filter(|p| p.cpu_usage() > 0.0)
                .filter(|p| {
                    if filter_lower.is_empty() {
                        true
                    } else {
                        p.name()
                            .to_string_lossy()
                            .to_lowercase()
                            .contains(&filter_lower)
                            || p.pid().as_u32().to_string().contains(&filter_lower)
                            || p.cmd()
                                .iter()
                                .any(|s| s.to_string_lossy().to_lowercase().contains(&filter_lower))
                    }
                })
                .map(|p| {
                    let cmd_parts: Vec<String> = p
                        .cmd()
                        .iter()
                        .map(|s| s.to_string_lossy().to_string())
                        .collect();
                    let cmd = if cmd_parts.is_empty() {
                        p.name().to_string_lossy().to_string()
                    } else {
                        cmd_parts.join(" ")
                    };
                    let state = match p.status() {
                        sysinfo::ProcessStatus::Run => ProcessState::Run,
                        sysinfo::ProcessStatus::Sleep => ProcessState::Sleep,
                        sysinfo::ProcessStatus::Idle => ProcessState::Idle,
                        sysinfo::ProcessStatus::Zombie => ProcessState::Zombie,
                        _ => ProcessState::Unknown,
                    };
                    let user = p
                        .user_id()
                        .and_then(|uid| {
                            self.users
                                .iter()
                                .find(|u| u.id() == uid)
                                .map(|u| u.name().to_string())
                        })
                        .unwrap_or_default();
                    ProcessInfo {
                        pid: p.pid().as_u32(),
                        ppid: p.parent().map(|p| p.as_u32()).unwrap_or(0),
                        name: p.name().to_string_lossy().to_string(),
                        cmd,
                        user,
                        cpu_usage: p.cpu_usage(),
                        memory_bytes: p.memory(),
                        state,
                        run_time_secs: p.run_time(),
                        tree_depth: 0,
                    }
                })
                .collect();
            match self.process_sort {
                ProcessSort::Cpu => procs.sort_by(|a, b| {
                    b.cpu_usage
                        .partial_cmp(&a.cpu_usage)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
                ProcessSort::Memory => procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes)),
                ProcessSort::Pid => procs.sort_by(|a, b| a.pid.cmp(&b.pid)),
                ProcessSort::Name => {
                    procs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                }
            }
            if self.sort_reverse {
                procs.reverse();
            }
            // Tree view: reorder by parent-child depth-first.
            if self.tree_mode {
                procs = Self::build_tree(procs);
            }
            procs.truncate(100);
            self.processes = procs;
            // Clamp scroll to valid range.
            if self.process_scroll >= self.processes.len() {
                self.process_scroll = self.processes.len().saturating_sub(1);
            }

            self.last_sys_refresh = now;
        }

        // Re-read daemon cache every 5 seconds.
        if now.duration_since(self.last_cache_read).as_secs() >= 5 {
            self.tailscale = self.cache_reader.read_tailscale();
            self.claude = self.cache_reader.read_claude();
            self.billing = self.cache_reader.read_billing();
            self.k8s = self.cache_reader.read_k8s();
            self.claude_personal = self.cache_reader.read_claude_personal();
            self.last_cache_read = now;
        }
    }

    /// Check if a waifu image is loaded (for layout decisions).
    pub fn has_waifu(&self) -> bool {
        self.waifu_state.is_some()
    }

    /// Whether the waifu widget area should be shown in the layout.
    /// True when waifu is enabled AND a live endpoint is configured.
    pub fn wants_waifu(&self) -> bool {
        self.cfg.image.waifu_enabled && self.cfg.waifu_endpoint().is_some()
    }

    /// Navigate to a waifu image by relative offset (1 = next, -1 = prev).
    /// Also triggers a background fetch to grow the gallery on demand.
    pub fn waifu_navigate(&mut self, delta: i32) {
        let n = self.waifu_gallery.len() as i32;
        if n == 0 {
            return;
        }
        let base = if self.waifu_index >= 0 {
            self.waifu_index
        } else {
            0
        };
        let new_idx = ((base + delta) % n + n) % n;
        self.waifu_load_at(new_idx as usize);

        // Auto-fetch more images as the user navigates.
        self.waifu_fetch_live();
    }

    /// Navigate to a random waifu image.
    /// Also triggers a background fetch to grow the gallery.
    pub fn waifu_random(&mut self) {
        let n = self.waifu_gallery.len();
        if n == 0 {
            return;
        }
        // Simple pseudo-random using system time nanos.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0) as usize;
        let idx = nanos % n;
        self.waifu_load_at(idx);

        // Auto-fetch more images as the user navigates.
        self.waifu_fetch_live();
    }

    /// Fetch a new random image from the live waifu mirror service.
    /// Non-blocking: spawns a tokio task, result arrives via channel.
    pub fn waifu_fetch_live(&mut self) {
        if self.waifu_fetching {
            return; // Already fetching.
        }
        let endpoint = match self.cfg.waifu_endpoint() {
            Some(ep) => ep.to_string(),
            None => return, // No endpoint configured.
        };
        let category = self.cfg.waifu_category().to_string();
        let tx = self.waifu_fetch_tx.clone();
        self.waifu_fetching = true;

        tokio::spawn(async move {
            match data::waifu_client::fetch_random(&endpoint, &category).await {
                Ok(result) => {
                    let _ = tx.send(Some(result)).await;
                }
                Err(e) => {
                    tracing::warn!("waifu fetch failed: {}", e);
                    let _ = tx.send(None).await;
                }
            }
        });
    }

    /// Minimum gallery size for auto-fetch on launch.
    const GALLERY_PREFETCH: usize = 3;

    /// Poll for completed live fetch results (called from tick).
    fn poll_waifu_fetch(&mut self) {
        while let Ok(msg) = self.waifu_fetch_rx.try_recv() {
            self.waifu_fetching = false;

            let result = match msg {
                Some(r) => r,
                None => {
                    // Fetch failed; chain next if gallery still small.
                    if self.waifu_gallery.len() < Self::GALLERY_PREFETCH {
                        self.waifu_fetch_live();
                    }
                    continue;
                }
            };

            // Decode image from raw bytes.
            let image = match data::waifu::decode_image_bytes(&result.data) {
                Ok(img) => img,
                Err(e) => {
                    tracing::warn!("waifu decode failed: {}", e);
                    continue;
                }
            };

            // Dedup by hash: skip if already in gallery.
            if self.waifu_gallery.iter().any(|e| e.hash == result.hash) {
                // Already have this image; just navigate to it.
                if let Some(idx) = self
                    .waifu_gallery
                    .iter()
                    .position(|e| e.hash == result.hash)
                {
                    self.waifu_load_at(idx);
                }
                // Still chain prefetch — the dupe doesn't count toward our target.
                if self.waifu_gallery.len() < Self::GALLERY_PREFETCH {
                    self.waifu_fetch_live();
                }
                continue;
            }

            // Add to gallery.
            let name = data::waifu::format_image_name(&result.name);
            let gallery_was_small = self.waifu_gallery.len() < Self::GALLERY_PREFETCH;
            let entry = WaifuEntry {
                image: image.clone(),
                name: name.clone(),
                hash: result.hash,
            };
            self.waifu_gallery.push(entry);

            // Auto-display during initial prefetch (gallery building up).
            // After prefetch, silently add to gallery — don't stomp user's navigation.
            if gallery_was_small || self.waifu_index < 0 {
                let idx = self.waifu_gallery.len() - 1;
                let scaled = self.prepare_waifu_image(&image);
                self.waifu_state = Some(self.picker.new_resize_protocol(scaled));
                self.waifu_index = idx as i32;
                self.waifu_name = name;
            }

            // Auto-fetch more until gallery reaches prefetch target.
            if self.waifu_gallery.len() < Self::GALLERY_PREFETCH {
                self.waifu_fetch_live();
            }
        }
    }

    /// Load the waifu image at the given gallery index.
    /// Pre-scales the image to fill the widget area (cover mode).
    pub(crate) fn waifu_load_at(&mut self, idx: usize) {
        if idx >= self.waifu_gallery.len() {
            return;
        }
        let entry = &self.waifu_gallery[idx];
        let scaled = self.prepare_waifu_image(&entry.image);
        self.waifu_state = Some(self.picker.new_resize_protocol(scaled));
        self.waifu_index = idx as i32;
        self.waifu_name = entry.name.clone();
    }

    /// Pre-scale image to fill the widget area (CSS object-fit: cover).
    /// Scales the image so its cell dimensions >= the widget area,
    /// ensuring Resize::Crop fills the widget with no empty space.
    fn prepare_waifu_image(&self, image: &image::DynamicImage) -> image::DynamicImage {
        let (fw, fh) = self.picker.font_size();
        if fw == 0 || fh == 0 {
            return image.clone();
        }

        // Estimate widget area in cells. Waifu gets ~40% width, full height minus chrome.
        let cols = if self.term_width > 0 {
            (self.term_width * 40 / 100).max(20) as u32
        } else {
            80
        };
        let rows = if self.term_height > 0 {
            self.term_height.saturating_sub(4).max(10) as u32
        } else {
            40
        };

        // Target pixel dimensions.
        let target_w = cols * fw as u32;
        let target_h = rows * fh as u32;

        if target_w == 0 || target_h == 0 {
            return image.clone();
        }

        // resize_to_fill: scales uniformly to cover the target, then center-crops to exact size.
        // CatmullRom is a good speed/quality balance (Lanczos3 is ~3x slower).
        image.resize_to_fill(target_w, target_h, FilterType::CatmullRom)
    }

    /// Kill the currently selected process.
    fn kill_selected_process(&mut self, force: bool) {
        if let Some(proc_info) = self.processes.get(self.process_scroll) {
            let pid = sysinfo::Pid::from_u32(proc_info.pid);
            if let Some(process) = self.proc_sys.process(pid) {
                if force {
                    process.kill(); // SIGKILL
                } else {
                    process.kill_with(sysinfo::Signal::Term); // SIGTERM
                }
            }
        }
    }

    /// Build a depth-first tree ordering of processes.
    fn build_tree(mut procs: Vec<ProcessInfo>) -> Vec<ProcessInfo> {
        use std::collections::HashMap;

        let pids: std::collections::HashSet<u32> = procs.iter().map(|p| p.pid).collect();

        // Build children map: ppid -> [indices].
        let mut children: HashMap<u32, Vec<usize>> = HashMap::new();
        for (i, p) in procs.iter().enumerate() {
            children.entry(p.ppid).or_default().push(i);
        }

        // Find roots (ppid not in our set, or ppid == 0).
        let roots: Vec<usize> = procs
            .iter()
            .enumerate()
            .filter(|(_, p)| p.ppid == 0 || !pids.contains(&p.ppid))
            .map(|(i, _)| i)
            .collect();

        let mut result = Vec::with_capacity(procs.len());
        let mut stack: Vec<(usize, usize)> = Vec::new(); // (index, depth)

        // Push roots in reverse so first root is popped first.
        for &idx in roots.iter().rev() {
            stack.push((idx, 0));
        }

        let mut visited = vec![false; procs.len()];
        while let Some((idx, depth)) = stack.pop() {
            if visited[idx] {
                continue;
            }
            visited[idx] = true;

            // Push children in reverse.
            let pid = procs[idx].pid;
            if let Some(child_indices) = children.get(&pid) {
                for &ci in child_indices.iter().rev() {
                    if !visited[ci] {
                        stack.push((ci, depth + 1));
                    }
                }
            }

            result.push((idx, depth));
        }

        // Add any unvisited procs at the end (shouldn't happen, but safety).
        for i in 0..procs.len() {
            if !visited[i] {
                result.push((i, 0));
            }
        }

        // Reorder procs by tree order and set depth.
        // We need to move items out without borrow issues, so use indices.
        let ordered: Vec<ProcessInfo> = result
            .into_iter()
            .map(|(idx, depth)| ProcessInfo {
                pid: procs[idx].pid,
                ppid: procs[idx].ppid,
                name: std::mem::take(&mut procs[idx].name),
                cmd: std::mem::take(&mut procs[idx].cmd),
                user: std::mem::take(&mut procs[idx].user),
                cpu_usage: procs[idx].cpu_usage,
                memory_bytes: procs[idx].memory_bytes,
                state: procs[idx].state,
                run_time_secs: procs[idx].run_time_secs,
                tree_depth: depth,
            })
            .collect();

        ordered
    }

    fn next_tab(&mut self) {
        let tabs = Tab::ALL;
        let idx = tabs.iter().position(|t| *t == self.active_tab).unwrap_or(0);
        self.active_tab = tabs[(idx + 1) % tabs.len()];
    }

    fn prev_tab(&mut self) {
        let tabs = Tab::ALL;
        let idx = tabs.iter().position(|t| *t == self.active_tab).unwrap_or(0);
        self.active_tab = tabs[(idx + tabs.len() - 1) % tabs.len()];
    }
}

#[cfg(test)]
impl App {
    /// Create a test App that does NOT touch the OS, terminal, or filesystem.
    /// All data fields are empty/default. Use builder-style methods to set state.
    pub fn test_new(cfg: TuiConfig) -> Self {
        let (waifu_fetch_tx, waifu_fetch_rx) = mpsc::channel(4);
        Self {
            cfg,
            active_tab: Tab::Dashboard,
            term_width: 160,
            term_height: 50,
            show_help: false,
            help_tab: 0,
            frozen: false,
            process_filter: String::new(),
            filter_mode: false,
            refresh_ms: 1000,
            show_cmd: false,
            tree_mode: false,
            sys: SysMetrics::empty(),
            cpu_history: VecDeque::new(),
            cpu_per_core_history: Vec::new(),
            mem_history: VecDeque::new(),
            swap_history: VecDeque::new(),
            net_rx_history: VecDeque::new(),
            net_tx_history: VecDeque::new(),
            load_history: VecDeque::new(),
            temp_history: VecDeque::new(),
            pending_kill: None,
            processes: Vec::new(),
            process_sort: ProcessSort::Cpu,
            sort_reverse: false,
            process_scroll: 0,
            total_process_count: 0,
            tailscale: None,
            claude: None,
            billing: None,
            k8s: None,
            waifu_state: None,
            waifu_gallery: Vec::new(),
            waifu_index: -1,
            waifu_show_info: false,
            waifu_name: String::new(),
            waifu_fetching: false,
            claude_personal: None,
            expanded: false,
            picker: Picker::from_fontsize((8, 16)),
            proc_sys: sysinfo::System::new(),
            users: sysinfo::Users::new_with_refreshed_list(),
            cache_reader: CacheReader::new(std::path::PathBuf::from("/nonexistent")),
            last_cache_read: Instant::now(),
            last_sys_refresh: Instant::now(),
            component_versions: Default::default(),
            waifu_fetch_rx,
            waifu_fetch_tx,
        }
    }

    /// Builder: set waifu gallery for testing navigation.
    pub fn with_waifu_gallery(mut self, gallery: Vec<WaifuEntry>) -> Self {
        if !gallery.is_empty() {
            self.waifu_index = 0;
        }
        self.waifu_gallery = gallery;
        self
    }

    /// Builder: enable waifu in config with an endpoint.
    pub fn with_waifu_enabled(mut self) -> Self {
        self.cfg.image.waifu_enabled = true;
        self.cfg.collectors.waifu.endpoint = "https://waifu.example.com".into();
        self
    }

    /// Builder: set processes for testing scroll/sort.
    pub fn with_processes(mut self, procs: Vec<ProcessInfo>) -> Self {
        self.total_process_count = procs.len();
        self.processes = procs;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TuiConfig;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn char_key(c: char) -> KeyEvent {
        key(KeyCode::Char(c))
    }

    fn make_procs(n: usize) -> Vec<ProcessInfo> {
        (0..n)
            .map(|i| ProcessInfo {
                pid: i as u32,
                ppid: 0,
                name: format!("p{i}"),
                cmd: String::new(),
                user: String::new(),
                cpu_usage: 0.0,
                memory_bytes: 0,
                state: ProcessState::Run,
                run_time_secs: 0,
                tree_depth: 0,
            })
            .collect()
    }

    // --- Tab Navigation ---

    #[test]
    fn test_tab_next_cycles() {
        let mut app = App::test_new(TuiConfig::default());
        assert_eq!(app.active_tab, Tab::Dashboard);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_tab, Tab::System);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_tab, Tab::Network);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_tab, Tab::Billing);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_tab, Tab::Build);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_tab, Tab::Dashboard);
    }

    #[test]
    fn test_tab_number_keys() {
        let mut app = App::test_new(TuiConfig::default());
        app.handle_key(char_key('3'));
        assert_eq!(app.active_tab, Tab::Network);
        app.handle_key(char_key('5'));
        assert_eq!(app.active_tab, Tab::Build);
        app.handle_key(char_key('1'));
        assert_eq!(app.active_tab, Tab::Dashboard);
    }

    #[test]
    fn test_tab_prev_wraps() {
        let mut app = App::test_new(TuiConfig::default());
        app.handle_key(key(KeyCode::BackTab));
        assert_eq!(app.active_tab, Tab::Build);
    }

    // --- Filter Mode ---

    #[test]
    fn test_filter_mode_captures_chars() {
        let mut app = App::test_new(TuiConfig::default());
        app.handle_key(char_key('/'));
        assert!(app.filter_mode);
        app.handle_key(char_key('f'));
        app.handle_key(char_key('o'));
        app.handle_key(char_key('o'));
        assert_eq!(app.process_filter, "foo");
        // Tab should NOT switch tabs in filter mode.
        app.handle_key(key(KeyCode::Tab));
        assert!(app.filter_mode);
        assert_eq!(app.active_tab, Tab::Dashboard);
    }

    #[test]
    fn test_filter_mode_esc_clears() {
        let mut app = App::test_new(TuiConfig::default());
        app.handle_key(char_key('/'));
        app.handle_key(char_key('x'));
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.filter_mode);
        assert!(app.process_filter.is_empty());
    }

    #[test]
    fn test_filter_mode_enter_keeps() {
        let mut app = App::test_new(TuiConfig::default());
        app.handle_key(char_key('/'));
        app.handle_key(char_key('x'));
        app.handle_key(key(KeyCode::Enter));
        assert!(!app.filter_mode);
        assert_eq!(app.process_filter, "x");
    }

    #[test]
    fn test_filter_mode_backspace() {
        let mut app = App::test_new(TuiConfig::default());
        app.handle_key(char_key('/'));
        app.handle_key(char_key('a'));
        app.handle_key(char_key('b'));
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.process_filter, "a");
    }

    // --- Help Overlay ---

    #[test]
    fn test_help_toggle() {
        let mut app = App::test_new(TuiConfig::default());
        app.handle_key(char_key('?'));
        assert!(app.show_help);
        assert_eq!(app.help_tab, 0);
        app.handle_key(char_key('?'));
        assert!(!app.show_help);
    }

    #[test]
    fn test_help_tab_navigation() {
        let mut app = App::test_new(TuiConfig::default());
        app.handle_key(char_key('?'));
        assert!(app.show_help);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.help_tab, 1);
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.help_tab, 2);
        app.handle_key(char_key('4'));
        assert_eq!(app.help_tab, 3);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.help_tab, 0); // wraps
    }

    #[test]
    fn test_help_dismiss_on_random_key() {
        let mut app = App::test_new(TuiConfig::default());
        app.handle_key(char_key('?'));
        assert!(app.show_help);
        app.handle_key(char_key('x'));
        assert!(!app.show_help);
    }

    // --- Expanded Mode ---

    #[test]
    fn test_expanded_mode_esc_exits() {
        let mut app = App::test_new(TuiConfig::default());
        app.expanded = true;
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.expanded);
    }

    #[test]
    fn test_expanded_mode_ignores_tab() {
        let mut app = App::test_new(TuiConfig::default());
        app.expanded = true;
        app.handle_key(key(KeyCode::Tab));
        assert!(app.expanded);
        assert_eq!(app.active_tab, Tab::Dashboard);
    }

    #[test]
    fn test_expanded_mode_waifu_keys() {
        let mut app = App::test_new(TuiConfig::default());
        app.expanded = true;
        app.handle_key(char_key('i'));
        assert!(app.waifu_show_info);
        app.handle_key(char_key('i'));
        assert!(!app.waifu_show_info);
    }

    // --- wants_waifu / has_waifu ---

    #[test]
    fn test_wants_waifu_disabled() {
        let app = App::test_new(TuiConfig::default());
        assert!(!app.wants_waifu());
        assert!(!app.has_waifu());
    }

    #[test]
    fn test_wants_waifu_enabled_with_endpoint() {
        let mut cfg = TuiConfig::default();
        cfg.image.waifu_enabled = true;
        cfg.collectors.waifu.endpoint = "https://waifu.example.com".into();
        let app = App::test_new(cfg);
        assert!(app.wants_waifu());
        assert!(!app.has_waifu());
    }

    #[test]
    fn test_wants_waifu_enabled_no_endpoint() {
        let mut cfg = TuiConfig::default();
        cfg.image.waifu_enabled = true;
        // No endpoint configured — wants_waifu should be false.
        let app = App::test_new(cfg);
        assert!(!app.wants_waifu());
    }

    #[test]
    fn test_has_waifu_requires_loaded_state() {
        let mut cfg = TuiConfig::default();
        cfg.image.waifu_enabled = true;
        cfg.collectors.waifu.endpoint = "https://example.com".into();
        let app = App::test_new(cfg);
        assert!(app.wants_waifu());
        assert!(!app.has_waifu()); // no image loaded yet
    }

    // --- Waifu Key Routing ---

    #[tokio::test]
    async fn test_dashboard_waifu_fetch_key() {
        let mut cfg = TuiConfig::default();
        cfg.image.waifu_enabled = true;
        cfg.collectors.waifu.endpoint = "https://example.com".into();
        let mut app = App::test_new(cfg);
        app.active_tab = Tab::Dashboard;
        app.handle_key(char_key('f'));
        assert!(app.waifu_fetching);
    }

    #[test]
    fn test_system_tab_n_is_sort() {
        let mut cfg = TuiConfig::default();
        cfg.image.waifu_enabled = true;
        cfg.collectors.waifu.endpoint = "https://example.com".into();
        let mut app = App::test_new(cfg);
        app.active_tab = Tab::System;
        app.handle_key(char_key('n'));
        assert_eq!(app.process_sort, ProcessSort::Name);
    }

    #[test]
    fn test_waifu_keys_require_wants_waifu() {
        let mut app = App::test_new(TuiConfig::default());
        app.active_tab = Tab::Dashboard;
        // Without waifu enabled, 'n' should be sort key
        app.handle_key(char_key('n'));
        assert_eq!(app.process_sort, ProcessSort::Name);
    }

    // --- Process Scroll & Sort ---

    #[test]
    fn test_process_scroll_bounded() {
        let mut app = App::test_new(TuiConfig::default()).with_processes(make_procs(5));
        for _ in 0..20 {
            app.handle_key(char_key('j'));
        }
        assert_eq!(app.process_scroll, 4);
    }

    #[test]
    fn test_refresh_rate_bounds() {
        let mut app = App::test_new(TuiConfig::default());
        assert_eq!(app.refresh_ms, 1000);
        for _ in 0..20 {
            app.handle_key(char_key('+'));
        }
        assert_eq!(app.refresh_ms, 250);
        for _ in 0..40 {
            app.handle_key(char_key('-'));
        }
        assert_eq!(app.refresh_ms, 5000);
    }

    #[test]
    fn test_build_tree_parent_child() {
        let procs = vec![
            ProcessInfo {
                pid: 1,
                ppid: 0,
                name: "init".into(),
                cmd: String::new(),
                user: String::new(),
                cpu_usage: 0.0,
                memory_bytes: 0,
                state: ProcessState::Run,
                run_time_secs: 0,
                tree_depth: 0,
            },
            ProcessInfo {
                pid: 2,
                ppid: 1,
                name: "child".into(),
                cmd: String::new(),
                user: String::new(),
                cpu_usage: 0.0,
                memory_bytes: 0,
                state: ProcessState::Run,
                run_time_secs: 0,
                tree_depth: 0,
            },
        ];
        let tree = App::build_tree(procs);
        assert_eq!(tree[0].pid, 1);
        assert_eq!(tree[0].tree_depth, 0);
        assert_eq!(tree[1].pid, 2);
        assert_eq!(tree[1].tree_depth, 1);
    }

    // --- Waifu Navigation ---

    fn make_gallery(n: usize) -> Vec<WaifuEntry> {
        (0..n)
            .map(|i| WaifuEntry {
                image: image::DynamicImage::new_rgb8(1, 1),
                name: format!("waifu_{i}"),
                hash: format!("hash_{i}"),
            })
            .collect()
    }

    #[tokio::test]
    async fn test_navigate_empty_noop() {
        let mut app = App::test_new(TuiConfig::default());
        app.waifu_navigate(1);
        assert_eq!(app.waifu_index, -1);
    }

    #[tokio::test]
    async fn test_navigate_forward_through_gallery() {
        let mut app = App::test_new(TuiConfig::default())
            .with_waifu_enabled()
            .with_waifu_gallery(make_gallery(3));
        assert_eq!(app.waifu_index, 0);
        app.waifu_navigate(1);
        assert_eq!(app.waifu_index, 1);
        assert_eq!(app.waifu_name, "waifu_1");
        app.waifu_navigate(1);
        assert_eq!(app.waifu_index, 2);
        assert_eq!(app.waifu_name, "waifu_2");
    }

    #[tokio::test]
    async fn test_navigate_wraps_forward() {
        let mut app = App::test_new(TuiConfig::default())
            .with_waifu_enabled()
            .with_waifu_gallery(make_gallery(3));
        app.waifu_index = 2;
        app.waifu_navigate(1);
        assert_eq!(app.waifu_index, 0);
        assert_eq!(app.waifu_name, "waifu_0");
    }

    #[tokio::test]
    async fn test_navigate_wraps_backward() {
        let mut app = App::test_new(TuiConfig::default())
            .with_waifu_enabled()
            .with_waifu_gallery(make_gallery(3));
        assert_eq!(app.waifu_index, 0);
        app.waifu_navigate(-1);
        assert_eq!(app.waifu_index, 2);
        assert_eq!(app.waifu_name, "waifu_2");
    }

    #[tokio::test]
    async fn test_navigate_single_item_stays() {
        let mut app = App::test_new(TuiConfig::default())
            .with_waifu_enabled()
            .with_waifu_gallery(make_gallery(1));
        assert_eq!(app.waifu_index, 0);
        app.waifu_navigate(1);
        assert_eq!(app.waifu_index, 0);
        app.waifu_navigate(-1);
        assert_eq!(app.waifu_index, 0);
    }

    #[tokio::test]
    async fn test_waifu_key_n_navigates_on_dashboard() {
        let mut app = App::test_new(TuiConfig::default())
            .with_waifu_enabled()
            .with_waifu_gallery(make_gallery(3));
        // Load initial image so has_waifu() is true.
        app.waifu_load_at(0);
        app.active_tab = Tab::Dashboard;
        assert!(app.has_waifu());
        app.handle_key(char_key('n'));
        assert_eq!(app.waifu_index, 1);
        app.handle_key(char_key('p'));
        assert_eq!(app.waifu_index, 0);
    }

    #[tokio::test]
    async fn test_waifu_random_selects_from_gallery() {
        let mut app = App::test_new(TuiConfig::default())
            .with_waifu_enabled()
            .with_waifu_gallery(make_gallery(5));
        app.waifu_load_at(0);
        app.waifu_random();
        // Random should select a valid index.
        assert!(app.waifu_index >= 0 && (app.waifu_index as usize) < 5);
    }

    // --- Freeze Toggle ---

    #[test]
    fn test_freeze_toggle() {
        let mut app = App::test_new(TuiConfig::default());
        assert!(!app.frozen);
        app.handle_key(char_key(' '));
        assert!(app.frozen);
        app.handle_key(char_key(' '));
        assert!(!app.frozen);
    }

    // --- Mouse Handling ---

    #[test]
    fn test_mouse_scroll() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        let mut app = App::test_new(TuiConfig::default()).with_processes(make_procs(20));
        app.active_tab = Tab::System;
        app.handle_mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 50,
            row: 20,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.process_scroll, 3);
    }

    // --- Property-Based Tests ---

    use proptest::prelude::*;

    fn arb_key_code() -> impl Strategy<Value = KeyCode> {
        prop_oneof![
            Just(KeyCode::Tab),
            Just(KeyCode::BackTab),
            Just(KeyCode::Esc),
            Just(KeyCode::Enter),
            Just(KeyCode::Up),
            Just(KeyCode::Down),
            Just(KeyCode::Left),
            Just(KeyCode::Right),
            Just(KeyCode::Home),
            Just(KeyCode::End),
            Just(KeyCode::PageUp),
            Just(KeyCode::PageDown),
            prop::char::range('a', 'z').prop_map(KeyCode::Char),
            prop::char::range('0', '9').prop_map(KeyCode::Char),
            Just(KeyCode::Char(' ')),
            Just(KeyCode::Char('+')),
            Just(KeyCode::Char('-')),
            Just(KeyCode::Char('/')),
            Just(KeyCode::Char('?')),
        ]
    }

    proptest! {
        #[test]
        fn key_handling_never_panics(actions in proptest::collection::vec(arb_key_code(), 0..50)) {
            let mut app = App::test_new(TuiConfig::default()).with_processes(make_procs(10));
            for code in &actions {
                let event = KeyEvent::new(*code, KeyModifiers::NONE);
                app.handle_key(event);
            }
            // Invariants after any key sequence:
            prop_assert!(Tab::ALL.contains(&app.active_tab));
            prop_assert!(app.help_tab <= 3);
            prop_assert!(app.refresh_ms >= 250 && app.refresh_ms <= 5000);
            prop_assert!(app.process_scroll <= 9); // 10 procs, max scroll = 9
        }

        #[test]
        fn resize_stores_dimensions(w in 40u16..=300u16, h in 10u16..=100u16) {
            let mut app = App::test_new(TuiConfig::default());
            app.on_resize(w, h);
            prop_assert_eq!(app.term_width, w);
            prop_assert_eq!(app.term_height, h);
        }

        #[test]
        fn wants_waifu_logic(
            enabled in proptest::bool::ANY,
            has_endpoint in proptest::bool::ANY,
        ) {
            let mut cfg = TuiConfig::default();
            cfg.image.waifu_enabled = enabled;
            if has_endpoint {
                cfg.collectors.waifu.endpoint = "https://waifu.example.com".into();
            }
            let app = App::test_new(cfg);
            let wants = app.wants_waifu();
            // Must be enabled AND have endpoint
            let expected = enabled && has_endpoint;
            prop_assert_eq!(wants, expected);
        }
    }
}
