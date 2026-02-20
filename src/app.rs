use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

use crate::config::TuiConfig;
use crate::data::{self, CacheReader, SysMetrics, TailscaleStatus, ClaudeUsage, BillingReport, K8sStatus};
use crate::data::claudepersonal::ClaudePersonalReport;

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
}

impl Tab {
    pub const ALL: &[Tab] = &[Tab::Dashboard, Tab::System, Tab::Network, Tab::Billing];

    pub fn title(&self) -> &str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::System => "System",
            Tab::Network => "Network",
            Tab::Billing => "Billing",
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
    pub tree_depth: usize,  // 0 = root, 1+ = child depth
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
    pub help_tab: usize,  // 0=TUI, 1=Shell, 2=Lab, 3=Starship
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
    pub temp_history: VecDeque<f64>,  // max temperature over last 60s

    // Process kill: double-d (btm-style) confirmation.
    pub pending_kill: Option<Instant>,  // timestamp of first 'd' press

    // Top processes by CPU usage.
    pub processes: Vec<ProcessInfo>,
    pub process_sort: ProcessSort,
    pub sort_reverse: bool,
    pub process_scroll: usize,
    pub total_process_count: usize,  // unfiltered count for title display

    // Cached data from Go daemon.
    pub tailscale: Option<TailscaleStatus>,
    pub claude: Option<ClaudeUsage>,
    pub billing: Option<BillingReport>,
    pub k8s: Option<K8sStatus>,

    // Waifu image rendering state (ratatui-image StatefulProtocol).
    pub waifu_state: Option<StatefulProtocol>,

    // Waifu sequential navigation state.
    pub waifu_images: Vec<PathBuf>,
    pub waifu_index: i32,
    pub waifu_show_info: bool,
    pub waifu_name: String,
    pub waifu_fetching: bool,  // true while an async fetch is in flight

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

    // Channel for receiving live-fetched waifu image paths.
    waifu_fetch_rx: mpsc::Receiver<PathBuf>,
    waifu_fetch_tx: mpsc::Sender<PathBuf>,
}

impl App {
    pub async fn new(cfg: TuiConfig, mut picker: Picker, expand_widget: Option<String>) -> Result<Self> {
        let cache_reader = CacheReader::new(cfg.cache_dir());
        let sys = SysMetrics::collect();

        // Initial cache read.
        let tailscale = cache_reader.read_tailscale();
        let claude = cache_reader.read_claude();
        let billing = cache_reader.read_billing();
        let k8s = cache_reader.read_k8s();
        let claude_personal = cache_reader.read_claude_personal();

        // Load waifu image list and initial image.
        let waifu_images = if cfg.image.waifu_enabled {
            data::waifu::list_images(&cfg)
        } else {
            Vec::new()
        };

        let waifu_state = if cfg.image.waifu_enabled {
            data::waifu::load_cached_waifu(&cfg)
                .ok()
                .flatten()
                .map(|img| picker.new_resize_protocol(img))
        } else {
            None
        };

        // Find the index of the initially loaded image (newest by mtime).
        let waifu_index: i32 = if !waifu_images.is_empty() {
            // The newest image is what load_cached_waifu picked; find it in the sorted list.
            // Since we don't know the exact path, default to last index.
            (waifu_images.len() as i32) - 1
        } else {
            -1
        };

        let waifu_name = if waifu_index >= 0 {
            data::waifu::format_image_name(&waifu_images[waifu_index as usize])
        } else {
            String::new()
        };

        // Expand mode from CLI flag.
        let expanded = expand_widget.as_deref() == Some("waifu");

        // Channel for async waifu fetch results.
        let (waifu_fetch_tx, waifu_fetch_rx) = mpsc::channel(4);

        // Initialize process system with CPU refresh for usage tracking.
        let mut proc_sys = sysinfo::System::new();
        proc_sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        let users = sysinfo::Users::new_with_refreshed_list();

        Ok(Self {
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
            waifu_images,
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
            waifu_fetch_rx,
            waifu_fetch_tx,
        })
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
                _ => self.show_help = false,  // Any other key dismisses
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

        // Dashboard tab: waifu navigation keys when waifu is loaded.
        if self.active_tab == Tab::Dashboard && self.has_waifu() {
            match key.code {
                KeyCode::Char('n') => { self.waifu_navigate(1); return; }
                KeyCode::Char('i') => { self.waifu_show_info = !self.waifu_show_info; return; }
                KeyCode::Char('f') => { self.waifu_fetch_live(); return; }
                _ => {}
            }
        }
        // Dashboard tab: 'f' to fetch even when no images loaded yet.
        if self.active_tab == Tab::Dashboard && !self.has_waifu() {
            if key.code == KeyCode::Char('f') {
                self.waifu_fetch_live();
                return;
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
            // Process table navigation (System tab).
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.processes.is_empty() {
                    self.process_scroll = (self.process_scroll + 1).min(self.processes.len().saturating_sub(1));
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
            KeyCode::Char('p') => {
                if self.active_tab == Tab::Dashboard && self.has_waifu() {
                    self.waifu_navigate(-1);
                } else {
                    self.process_sort = ProcessSort::Pid;
                }
            }
            KeyCode::Char('n') => self.process_sort = ProcessSort::Name,
            KeyCode::Char('r') => {
                if self.active_tab == Tab::Dashboard && self.has_waifu() {
                    self.waifu_random();
                } else {
                    self.sort_reverse = !self.sort_reverse;
                }
            }
            // Page up/down for process table.
            KeyCode::PageDown => {
                if !self.processes.is_empty() {
                    self.process_scroll = (self.process_scroll + 10).min(self.processes.len().saturating_sub(1));
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
                    self.process_scroll = (self.process_scroll + 3).min(self.processes.len().saturating_sub(1));
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
        self.term_width = w;
        self.term_height = h;
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
                self.cpu_per_core_history = vec![VecDeque::with_capacity(HISTORY_LEN); snap.cpu_usage.len()];
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
            let max_temp = snap.temperatures.iter()
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
            self.proc_sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            self.total_process_count = self.proc_sys.processes()
                .values()
                .filter(|p| p.cpu_usage() > 0.0)
                .count();
            let filter_lower = self.process_filter.to_lowercase();
            let mut procs: Vec<ProcessInfo> = self.proc_sys.processes()
                .values()
                .filter(|p| p.cpu_usage() > 0.0)
                .filter(|p| {
                    if filter_lower.is_empty() {
                        true
                    } else {
                        p.name().to_string_lossy().to_lowercase().contains(&filter_lower)
                            || p.pid().as_u32().to_string().contains(&filter_lower)
                            || p.cmd().iter().any(|s| s.to_string_lossy().to_lowercase().contains(&filter_lower))
                    }
                })
                .map(|p| {
                    let cmd_parts: Vec<String> = p.cmd().iter().map(|s| s.to_string_lossy().to_string()).collect();
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
                    let user = p.user_id()
                        .and_then(|uid| self.users
                            .iter()
                            .find(|u| u.id() == uid)
                            .map(|u| u.name().to_string()))
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
                ProcessSort::Cpu => procs.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal)),
                ProcessSort::Memory => procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes)),
                ProcessSort::Pid => procs.sort_by(|a, b| a.pid.cmp(&b.pid)),
                ProcessSort::Name => procs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
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

    /// Navigate to a waifu image by relative offset (1 = next, -1 = prev).
    pub fn waifu_navigate(&mut self, delta: i32) {
        let n = self.waifu_images.len() as i32;
        if n == 0 {
            return;
        }
        let base = if self.waifu_index >= 0 { self.waifu_index } else { 0 };
        let new_idx = ((base + delta) % n + n) % n;
        self.waifu_load_at(new_idx as usize);
    }

    /// Navigate to a random waifu image.
    pub fn waifu_random(&mut self) {
        let n = self.waifu_images.len();
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
        let cache_dir = self.cfg.cache_dir().join("waifu");
        let tx = self.waifu_fetch_tx.clone();
        self.waifu_fetching = true;

        tokio::spawn(async move {
            match data::waifu_client::fetch_random(&endpoint, &category, &cache_dir).await {
                Ok(path) => { let _ = tx.send(path).await; }
                Err(e) => { tracing::warn!("waifu fetch failed: {}", e); }
            }
        });
    }

    /// Poll for completed live fetch results (called from tick).
    fn poll_waifu_fetch(&mut self) {
        while let Ok(path) = self.waifu_fetch_rx.try_recv() {
            self.waifu_fetching = false;
            // Reload the image list and navigate to the new image.
            self.waifu_images = data::waifu::list_images(&self.cfg);
            if let Some(idx) = self.waifu_images.iter().position(|p| *p == path) {
                self.waifu_load_at(idx);
            } else if !self.waifu_images.is_empty() {
                // Image might have a different path; load the newest.
                self.waifu_load_at(self.waifu_images.len() - 1);
            }
        }
    }

    /// Load the waifu image at the given index.
    fn waifu_load_at(&mut self, idx: usize) {
        if idx >= self.waifu_images.len() {
            return;
        }
        let path = &self.waifu_images[idx];
        if let Ok(img) = data::waifu::load_image(path) {
            self.waifu_state = Some(self.picker.new_resize_protocol(img));
            self.waifu_index = idx as i32;
            self.waifu_name = data::waifu::format_image_name(path);
        }
    }

    /// Kill the currently selected process.
    fn kill_selected_process(&mut self, force: bool) {
        if let Some(proc_info) = self.processes.get(self.process_scroll) {
            let pid = sysinfo::Pid::from_u32(proc_info.pid);
            if let Some(process) = self.proc_sys.process(pid) {
                if force {
                    process.kill();  // SIGKILL
                } else {
                    process.kill_with(sysinfo::Signal::Term);  // SIGTERM
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
        let roots: Vec<usize> = procs.iter().enumerate()
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
        let ordered: Vec<ProcessInfo> = result.into_iter().map(|(idx, depth)| {
            ProcessInfo {
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
            }
        }).collect();

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
