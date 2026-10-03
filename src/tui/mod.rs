pub mod app;
pub mod events;
pub mod queries;
pub mod ui;

use crossterm::{
    event::EnableMouseCapture,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use sqlx::PgPool;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::tui::app::App;
use crate::tui::events::handle_events;
use crate::tui::queries::{load_recent_blocks, load_pools, load_pool_miners, load_miner_perfs};
use crate::tui::ui::render;

/// Shared state between the TUI and background tasks
pub struct SharedState {
    pub syncing: bool,
    pub latest_block_height: Option<i32>,
    pub new_mempool_txs: Vec<crate::tui::app::MempoolTx>,
    pub all_mempool_txs: Vec<crate::tui::app::MempoolTx>,
    pub network_stats: Option<crate::tui::app::NetworkStats>,
}

pub async fn run(pool: &PgPool, shared: Arc<Mutex<SharedState>>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new();
    app.blocks = load_recent_blocks(pool).await;
    app.intro_reveal = Some(0);

    let mut last_block_refresh = Instant::now();
    let mut last_panel_refresh = Instant::now();
    let mut last_known_height: Option<i32> = app.blocks.first().map(|b| b.height);

    loop {
        app.tick();

        // Sync shared state into app
        if let Ok(mut state) = shared.lock() {
            app.syncing = state.syncing;

            if let Some(new_height) = state.latest_block_height {
                if last_known_height.map_or(true, |h| new_height > h) {
                    app.trigger_new_block_slide(new_height);
                    last_known_height = Some(new_height);
                    app.blocks = load_recent_blocks(pool).await;
                    last_block_refresh = Instant::now();
                }
            }

            if !state.new_mempool_txs.is_empty() {
                if let Some(tx) = state.new_mempool_txs.last().cloned() {
                    app.mempool_notification = Some((tx, app.frame_count));
                }
                state.new_mempool_txs.clear();
            }

            app.mempool_txs = state.all_mempool_txs.clone();

            if let Some(stats) = state.network_stats.clone() {
                app.network_stats = Some(stats);
            }
        }

        // Refresh block list every 2 seconds
        if last_block_refresh.elapsed() > Duration::from_secs(2) {
            app.blocks = load_recent_blocks(pool).await;
            last_block_refresh = Instant::now();
        }

        // Refresh right panels every 30 seconds
        if last_panel_refresh.elapsed() > Duration::from_secs(30) {
            let hashps = app.network_stats.as_ref().map(|s| s.network_hashps).unwrap_or(0.0);
            app.pools = load_pools(pool, hashps).await;
            app.pool_miners = load_pool_miners(pool).await;
            app.miner_perfs = load_miner_perfs(pool, hashps).await;
            last_panel_refresh = Instant::now();
        }

        // Initial load of panels
        if app.pools.is_empty() {
            let hashps = app.network_stats.as_ref().map(|s| s.network_hashps).unwrap_or(0.0);
            app.pools = load_pools(pool, hashps).await;
            app.pool_miners = load_pool_miners(pool).await;
            app.miner_perfs = load_miner_perfs(pool, hashps).await;
        }

        terminal.draw(|frame| render(frame, &app))?;
        handle_events(&mut app, pool).await;

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
