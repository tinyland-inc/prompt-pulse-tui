#![allow(
    dead_code,
    clippy::redundant_closure,
    clippy::manual_div_ceil,
    clippy::if_same_then_else,
    clippy::needless_range_loop,
    clippy::derivable_impls
)]

mod app;
mod config;
mod data;
mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use ratatui_image::picker::Picker;
use tracing_subscriber::EnvFilter;

use crate::app::App;
use crate::config::TuiConfig;

const TICK_RATE: Duration = Duration::from_millis(250);

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing (RUST_LOG=debug for verbose output).
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(io::stderr)
        .init();

    // Parse CLI args: --expand <widget-id>
    let args: Vec<String> = std::env::args().collect();
    let expand_widget = args
        .windows(2)
        .find(|w| w[0] == "--expand")
        .map(|w| w[1].clone());

    let cfg = TuiConfig::load()?;

    // Terminal setup.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Query terminal for image protocol support and font size.
    // Must be called after EnterAlternateScreen but before event loop.
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| {
        tracing::warn!("failed to query terminal capabilities, falling back to halfblocks");
        Picker::from_fontsize((8, 16))
    });

    let mut app = App::new(cfg, picker, expand_widget).await?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app).await;

    // Restore terminal.
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Poll for events with tick-rate timeout.
        if event::poll(TICK_RATE)? {
            match event::read()? {
                Event::Key(key) => {
                    // Ctrl+C always quits.
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        return Ok(());
                    }
                    // q quits (unless in expand mode where Esc exits expand first).
                    if key.code == KeyCode::Char('q') {
                        return Ok(());
                    }
                    // Esc quits only if not in expand mode.
                    if key.code == KeyCode::Esc && !app.expanded {
                        return Ok(());
                    }
                    app.handle_key(key);
                }
                Event::Resize(w, h) => {
                    app.on_resize(w, h);
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse(mouse);
                }
                _ => {}
            }
        }

        // Tick: refresh real-time data (CPU, RAM, network).
        app.tick().await;
    }
}
