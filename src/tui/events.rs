use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use sqlx::PgPool;
use std::time::Duration;

use crate::tui::app::{App, Mode};
use crate::tui::queries::search;

/// The row range of the search box in the layout (row 3, height 3 → rows 3..6)
/// We detect mouse clicks within this area to activate the search box.
const SEARCH_BOX_TOP: u16 = 3;
const SEARCH_BOX_BOTTOM: u16 = 5;

pub async fn handle_events(app: &mut App, pool: &PgPool) {
    if !event::poll(Duration::from_millis(100)).unwrap_or(false) {
        return;
    }

    match event::read() {
        Ok(Event::Key(key)) => {
            if key.kind != KeyEventKind::Press {
                return;
            }
            match app.mode {
                Mode::Normal => handle_normal(app, key.code),
                Mode::Searching => handle_searching(app, pool, key.code).await,
            }
        }
        Ok(Event::Mouse(mouse)) => {
            handle_mouse(app, mouse);
        }
        _ => {}
    }
}

fn handle_normal(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('/') => app.enter_search_mode(),
        KeyCode::Esc => app.clear(),
        KeyCode::Up => app.scroll_up(),
        KeyCode::Down => app.scroll_down(),
        _ => {}
    }
}

async fn handle_searching(app: &mut App, pool: &PgPool, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.exit_search_mode();
        }
        KeyCode::Enter => {
            let query = app.search_input.trim().to_string();
            if !query.is_empty() {
                app.status = format!("Searching for: {query}...");
                app.result = Some(search(pool, &query).await);
                app.scroll = 0;
                app.status = format!("Results for: {query}  |  Press Esc to clear");
            }
            app.exit_search_mode();
        }
        KeyCode::Backspace => {
            app.search_input.pop();
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
        }
        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    match mouse.kind {
        // Left click on the search box area → activate search mode
        MouseEventKind::Down(MouseButton::Left) => {
            if mouse.row >= SEARCH_BOX_TOP && mouse.row <= SEARCH_BOX_BOTTOM {
                app.enter_search_mode();
            }
        }
        // Scroll wheel in block list
        MouseEventKind::ScrollUp => app.scroll_up(),
        MouseEventKind::ScrollDown => app.scroll_down(),
        _ => {}
    }
}
