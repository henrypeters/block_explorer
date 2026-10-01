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
use crate::tui::queries::load_recent_blocks;
use crate::tui::ui::render;

/// Shared state between the TUI and background tasks
pub struct SharedState {
    pub syncing: bool,
    pub latest_block_height: Option<i32>,
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
    // Start intro from oldest block (blocks are newest-first, so reverse for intro)
    app.intro_reveal = Some(0);

    let mut last_block_refresh = Instant::now();
    let mut last_known_height: Option<i32> = app.blocks.first().map(|b| b.height);

    loop {
        // Advance animation tick
        app.tick();

        // Sync shared state into app
        if let Ok(state) = shared.lock() {
            app.syncing = state.syncing;

            // Detect new block arrival for slide animation
            if let Some(new_height) = state.latest_block_height {
                if last_known_height.map_or(true, |h| new_height > h) {
                    app.trigger_new_block_slide(new_height);
                    last_known_height = Some(new_height);
                    // Refresh block list immediately
                    app.blocks = load_recent_blocks(pool).await;
                    last_block_refresh = Instant::now();
                }
            }
        }

        // Refresh block list every 5 seconds
        if last_block_refresh.elapsed() > Duration::from_secs(5) {
            app.blocks = load_recent_blocks(pool).await;
            last_block_refresh = Instant::now();
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
