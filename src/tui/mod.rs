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
use std::time::{Duration, Instant};

use crate::tui::app::App;
use crate::tui::events::handle_events;
use crate::tui::queries::load_recent_blocks;
use crate::tui::ui::render;

/// Starts the interactive TUI. Runs until the user presses q.
pub async fn run(pool: &PgPool) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    // Load initial block list
    app.blocks = load_recent_blocks(pool).await;

    let mut last_block_refresh = Instant::now();

    loop {
        terminal.draw(|frame| render(frame, &app))?;

        handle_events(&mut app, pool).await;

        if app.should_quit {
            break;
        }

        // Refresh block list every 6 seconds to show newly mined blocks
        if last_block_refresh.elapsed() > Duration::from_secs(6) {
            app.blocks = load_recent_blocks(pool).await;
            last_block_refresh = Instant::now();
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
