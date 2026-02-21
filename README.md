# prompt-pulse-tui

A resizable terminal dashboard with live system metrics, Tailscale status, cloud billing, Claude AI usage tracking, Kubernetes cluster info, and anime character image rendering. Because your terminal deserves to be both informative *and* delightful.

Built with [Ratatui](https://ratatui.rs/) and async Rust. Think `btop` meets your homelab control plane, with a waifu gallery on the side.

<!-- TODO: Add screenshot once stable
![prompt-pulse-tui dashboard](docs/screenshot.png)
*Dashboard tab showing CPU sparklines, Tailscale peers, billing breakdown, and a waifu panel in Kitty protocol*
-->

## Features

- **Live system metrics** -- CPU per-core bars, memory/swap gauges, disk usage, temperatures, network throughput, load averages, battery status
- **60-second sparkline history** -- CPU, memory, swap, load, temperature, network RX/TX with rolling history buffers
- **Process manager** -- Scrollable process table with sort (CPU/memory/PID/name), filter (`/` search), tree view, and kill signals (dd = SIGTERM, D = SIGKILL)
- **Tailscale integration** -- Peer list with online/offline status, tailnet name, IPs, OS, traffic stats via LocalAPI
- **Kubernetes clusters** -- Node readiness, pod counts by namespace (running/pending/failed), multi-context support
- **Cloud billing** -- Multi-provider month-to-date costs (Civo, DigitalOcean, etc.), budget tracking, per-resource breakdown
- **Claude AI usage** -- API token consumption by model/workspace, daily burn rate, projected monthly cost, personal plan rate-limit gauge
- **Waifu image rendering** -- Full-color anime character images in your terminal with gallery navigation, random selection, and live fetching
- **Adaptive layout** -- Responsive design that rearranges widgets based on terminal width (wide vs narrow breakpoints at 120 columns)
- **5 tabbed views** -- Dashboard, System, Network, Billing, Build
- **Mouse support** -- Click tabs, scroll process table
- **Adjustable refresh rate** -- 250ms to 5s with `+`/`-` keys
- **Freeze mode** -- Space bar pauses all data collection
- **Build info tab** -- Git SHA, daemon version, Home Manager generation, Nix version, flake input revisions

## Quick Start

```bash
# Clone and build
git clone https://github.com/tinyland-inc/prompt-pulse-tui.git
cd prompt-pulse-tui
cargo build --release

# Run
./target/release/prompt-pulse-tui

# Or install directly
cargo install --path .
```

**Requirements:** Rust 1.75+ (stable toolchain)

## Configuration

Configuration lives at `~/.config/prompt-pulse/config.toml` (respects `XDG_CONFIG_HOME`). The TUI reads the same config file as the companion Go daemon. If no config file exists, sensible defaults are used.

```toml
[general]
cache_dir = "~/.cache/prompt-pulse"

[collectors.sysmetrics]
enabled = true

[collectors.tailscale]
enabled = true

[collectors.kubernetes]
enabled = true

[collectors.claude]
enabled = true

[collectors.billing]
enabled = true

[collectors.waifu]
enabled = true
endpoint = "https://your-waifu-mirror.example.com"
category = "sfw"

[image]
waifu_enabled = true
protocol = "auto"    # auto, kitty, sixel, iterm2, halfblocks

[theme]
name = "default"
```

## Data Panels

| Panel | Source | What It Shows |
|-------|--------|---------------|
| **Host** | `sysinfo` crate | Hostname, OS, kernel, CPU model/freq, uptime, load average, IP, battery |
| **CPU** | `sysinfo` crate | Per-core usage bars with color-coded utilization |
| **Memory** | `sysinfo` crate | RAM and swap usage with gauges and percentages |
| **Disk** | `sysinfo` crate | Mount points, filesystem type, used/total with bar charts |
| **Temperature** | `sysinfo` crate | Sensor readings with color thresholds (green/yellow/red) |
| **Network** | `sysinfo` crate | Per-interface RX/TX rates, total throughput sparklines |
| **Processes** | `sysinfo` crate | Top 100 by CPU, sortable, filterable, tree view, kill support |
| **Tailscale** | Daemon cache (LocalAPI) | Peer list, online status, tailnet name, IPs, traffic |
| **Kubernetes** | Daemon cache | Cluster contexts, node readiness, pod counts by namespace |
| **Billing** | Daemon cache | Per-provider costs, budget percent, resource-level breakdown |
| **Claude API** | Daemon cache | Token usage by model/workspace, burn rate, monthly projection |
| **Claude Personal** | Daemon cache | Rate-limit gauge (messages in window / limit), cooldown timer |
| **Waifu** | Live HTTP fetch | Anime character images with gallery, info overlay, auto-prefetch |
| **Build Info** | Build-time + runtime | TUI git SHA, daemon version, HM generation, Nix version, flake inputs |

## Terminal Image Rendering

The waifu panel uses [ratatui-image](https://github.com/benjajaja/ratatui-image) to render full-color images directly in the terminal. Protocol detection is automatic but can be overridden in config:

| Protocol | Terminals | Quality |
|----------|-----------|---------|
| **Kitty** | Kitty, Ghostty | Best -- true color, pixel-perfect |
| **iTerm2** | iTerm2, WezTerm | Great -- inline images |
| **Sixel** | mlterm, foot, xterm (with sixel) | Good -- wide compatibility |
| **Halfblocks** | Everything else | Fallback -- uses Unicode half-block characters |

The TUI auto-detects Ghostty and Kitty from `TERM_PROGRAM` when the terminal query falls back to halfblocks. Images are pre-scaled to cover the widget area (CSS `object-fit: cover` style) using CatmullRom filtering.

**Waifu gallery keys (Dashboard tab):**
- `n` / `p` -- Next / previous image
- `r` -- Random image
- `f` -- Fetch new image from live service
- `i` -- Toggle info overlay

**Expand mode:** Launch with `--expand waifu` for fullscreen image viewing.

## Keyboard Reference

| Key | Action |
|-----|--------|
| `Tab` / `Right` | Next tab |
| `Shift-Tab` / `Left` | Previous tab |
| `1`-`5` | Jump to tab |
| `Space` | Freeze/resume data collection |
| `j`/`k` or `Up`/`Down` | Scroll process table |
| `g` / `G` | Jump to top/bottom of processes |
| `/` | Filter processes by name or PID |
| `c` / `m` / `p` / `n` | Sort by CPU / Memory / PID / Name |
| `r` | Reverse sort order |
| `e` | Toggle full command display |
| `t` | Toggle process tree view |
| `dd` | Send SIGTERM to selected process |
| `D` | Send SIGKILL to selected process |
| `+` / `-` | Adjust refresh rate (250ms - 5s) |
| `?` | Help overlay with 4 tabs (TUI, Shell, Lab, Starship) |
| `q` / `Esc` | Quit |

## Architecture

```
src/
  main.rs          -- Entry point, terminal setup, event loop (250ms tick)
  app.rs           -- Application state, key/mouse handling, process tree builder
  config.rs        -- TOML config loading (XDG-aware)
  data/
    sysmetrics.rs  -- CPU, RAM, disk, network, temps, battery via sysinfo
    tailscale.rs   -- Tailscale peer status (daemon cache)
    billing.rs     -- Cloud provider billing (daemon cache)
    k8s.rs         -- Kubernetes cluster info (daemon cache)
    claude.rs      -- Claude API usage metrics (daemon cache)
    claudepersonal.rs -- Claude personal plan rate-limit tracking
    waifu.rs       -- Image decoding, gallery management
    waifu_client.rs -- Async HTTP fetch for live waifu images
    cache.rs       -- JSON cache reader for Go daemon files
    buildinfo.rs   -- Compile-time and runtime version metadata
  ui/
    mod.rs         -- Top-level draw with tab bar, help overlay
    layout.rs      -- Responsive layouts per tab (wide/narrow breakpoints)
    widgets/       -- Individual widget renderers (cpu, memory, disk, etc.)
```

The TUI operates in two data modes:
1. **Real-time** -- System metrics collected in-process via the `sysinfo` crate
2. **Cached** -- Tailscale, K8s, billing, and Claude data read from JSON files written by a companion Go daemon (re-read every 5 seconds)

## Development

```bash
# Run in debug mode (with tracing)
RUST_LOG=debug cargo run

# Run tests (includes property-based tests via proptest)
cargo test

# Run clippy
cargo clippy

# Format
cargo fmt

# Build optimized release (thin LTO, stripped)
cargo build --release
```

The release profile uses `opt-level = 3`, thin LTO, and symbol stripping for a compact binary.

## License

MIT

