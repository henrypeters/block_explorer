use arboard::Clipboard;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use sqlx::PgPool;
use std::time::Duration;

use crate::tui::app::{App, Mode};
use crate::tui::queries::search;

const SEARCH_BOX_TOP: u16 = 4;
const SEARCH_BOX_BOTTOM: u16 = 6;

pub async fn handle_events(app: &mut App, pool: &PgPool) {
    if !event::poll(Duration::from_millis(100)).unwrap_or(false) {
        return;
    }

    match event::read() {
        Ok(Event::Key(key)) => {
            if key.kind != KeyEventKind::Press {
                return;
            }
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                app.should_quit = true;
                return;
            }
            match app.mode {
                Mode::Normal => handle_normal(app, key.code),
                Mode::Searching => handle_searching(app, pool, key.code).await,
                Mode::Copying => handle_copying(app, key.code),
            }
        }
        Ok(Event::Mouse(mouse)) => handle_mouse(app, pool, mouse).await,
        _ => {}
    }
}

fn handle_normal(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('/') => app.enter_search_mode(),
        KeyCode::Char('c') => {
            if app.result.is_some() {
                app.enter_copy_mode();
            }
        }
        KeyCode::Char('m') => {
            // Mempool only available on regtest
            let is_mainnet = app.network_stats.as_ref()
                .map(|s| s.chain == "main")
                .unwrap_or(false);
            if !is_mainnet {
                app.screen = crate::tui::app::Screen::Mempool;
                app.status = "Mempool  |  [b] Back to blocks   [↑↓] Scroll   [q] Quit".to_string();
            } else {
                app.status = "Mempool is not available on mainnet.".to_string();
            }
        }
        KeyCode::Char('b') => {
            app.screen = crate::tui::app::Screen::Blocks;
            app.status = "Click the search box or press / to search.".to_string();
        }
        KeyCode::Esc => {
            if app.detail_screen != crate::tui::app::DetailScreen::None {
                app.detail_screen = crate::tui::app::DetailScreen::None;
                app.detail_scroll = 0;
                app.status = "Click the search box or press / to search.".to_string();
            } else {
                app.clear();
            }
        }
        KeyCode::Up => {
            if app.detail_screen != crate::tui::app::DetailScreen::None {
                app.detail_scroll = app.detail_scroll.saturating_sub(1);
            } else {
                app.scroll_up();
            }
        }
        KeyCode::Down => {
            if app.detail_screen != crate::tui::app::DetailScreen::None {
                app.detail_scroll = app.detail_scroll.saturating_add(1);
            } else {
                app.scroll_down();
            }
        }
        _ => {}
    }
}

async fn handle_searching(app: &mut App, pool: &PgPool, key: KeyCode) {
    match key {
        KeyCode::Esc => app.exit_search_mode(),
        KeyCode::Enter => {
            let query = app.search_input.trim().to_string();
            if !query.is_empty() {
                app.searching = true;
                app.status = format!("Searching for: {query}...");
                let result = search(pool, &query).await;
                app.searching = false;
                app.status = match &result {
                    crate::tui::app::SearchResult::NotFound(_) => {
                        format!("No results for: \"{query}\"  —  Press Esc to go back")
                    }
                    crate::tui::app::SearchResult::Error(_) => {
                        format!("Error searching for: \"{query}\"  —  Press Esc to go back")
                    }
                    _ => format!("Results for: {query}"),
                };
                app.result = Some(result);
                app.scroll = 0;
            }
            app.exit_search_mode();
        }
        KeyCode::Backspace => { app.search_input.pop(); }
        KeyCode::Char(c) => { app.search_input.push(c); }
        _ => {}
    }
}

fn handle_copying(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.exit_copy_mode(),
        KeyCode::Up => {
            if app.copy_selected > 0 {
                app.copy_selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.copy_selected + 1 < app.copy_fields.len() {
                app.copy_selected += 1;
            }
        }
        KeyCode::Enter => {
            if let Some((label, value)) = app.copy_fields.get(app.copy_selected) {
                let label = label.clone();
                let value = value.clone();
                match Clipboard::new() {
                    Ok(mut clipboard) => {
                        match clipboard.set_text(&value) {
                            Ok(_) => {
                                app.status = format!("✔ Copied {label} to clipboard!");
                            }
                            Err(e) => {
                                app.status = format!("Failed to copy: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        app.status = format!("Clipboard unavailable: {e}");
                    }
                }
                app.exit_copy_mode();
            }
        }
        _ => {}
    }
}

async fn handle_mouse(app: &mut App, pool: &PgPool, mouse: crossterm::event::MouseEvent) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if mouse.row >= SEARCH_BOX_TOP && mouse.row <= SEARCH_BOX_BOTTOM {
                if app.mode != Mode::Searching {
                    app.enter_search_mode();
                }
            } else {
                if app.mode == Mode::Searching {
                    app.exit_search_mode();
                }
                app.mouse_x = mouse.column;
                app.mouse_y = mouse.row;

                // Check if a pending search was set by handle_panel_click
                app.handle_panel_click(mouse.column, mouse.row);

                // Execute block click search immediately
                if let Some(query) = app.pending_search.take() {
                    app.searching = true;
                    let result = search(pool, &query).await;
                    app.searching = false;
                    app.status = format!("Block #{}", query);
                    app.result = Some(result);
                    app.scroll = 0;
                }
            }
        }
        MouseEventKind::Moved => {
            app.mouse_x = mouse.column;
            app.mouse_y = mouse.row;
        }
        MouseEventKind::ScrollUp => {
            if app.detail_screen != crate::tui::app::DetailScreen::None {
                app.detail_scroll = app.detail_scroll.saturating_sub(1);
            } else {
                app.scroll_up();
            }
        }
        MouseEventKind::ScrollDown => {
            if app.detail_screen != crate::tui::app::DetailScreen::None {
                app.detail_scroll = app.detail_scroll.saturating_add(1);
            } else {
                app.scroll_down();
            }
        }
        _ => {}
    }
}
