use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::tui::app::{App, Mode, SearchResult};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // header
            Constraint::Length(3), // search box
            Constraint::Length(1), // spacer
            Constraint::Min(0),    // main content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    render_header(frame, app, chunks[0]);
    render_search(frame, app, chunks[1]);

    let is_mainnet = app.network_stats.as_ref()
        .map(|s| s.chain == "main")
        .unwrap_or(false);

    if app.result.is_some() {
        // Search results take full width on both networks
        render_results(frame, app, chunks[3]);
    } else if !is_mainnet {
        // Regtest — original full-width layout
        if app.screen == crate::tui::app::Screen::Mempool {
            render_mempool(frame, app, chunks[3]);
        } else {
            render_block_list(frame, app, chunks[3]);
        }
    } else {
        // Mainnet — split layout with mining intelligence panels
        match app.detail_screen {
            crate::tui::app::DetailScreen::None => {
                let split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(55),
                        Constraint::Percentage(45),
                    ])
                    .split(chunks[3]);
                render_block_list(frame, app, split[0]);
                render_right_panels(frame, app, split[1]);
            }
            crate::tui::app::DetailScreen::NetworkStats => {
                render_network_detail(frame, app, chunks[3]);
            }
            crate::tui::app::DetailScreen::PoolIntelligence => {
                render_pool_detail(frame, app, chunks[3]);
            }
            crate::tui::app::DetailScreen::MinerPerformance => {
                render_miner_detail(frame, app, chunks[3]);
            }
        }
    }

    // Mempool notification (regtest only)
    if !is_mainnet {
        if let Some((tx, _)) = &app.mempool_notification {
            render_mempool_notification(frame, tx, chunks[3]);
        }
    }

    render_status(frame, app, chunks[4]);

    if app.mode == Mode::Copying {
        render_copy_overlay(frame, app, area);
    }
}

// ─── Header ──────────────────────────────────────────────────────────────────

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(20)])
        .split(area);

    let title = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  _StrataBTC_",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]),
    ]))
    .alignment(Alignment::Left);
    frame.render_widget(title, cols[0]);

    let network_label = match app.network_stats.as_ref().map(|s| s.chain.as_str()) {
        Some("main") => "MAINNET",
        Some("test") => "TESTNET",
        _ => "REGTEST",
    };

    let badge = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {} ", network_label),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ]))
    .alignment(Alignment::Right);
    frame.render_widget(badge, cols[1]);
}

// ─── Search box ──────────────────────────────────────────────────────────────

fn render_search(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.mode == Mode::Searching {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    // What to show inside the box
    let (content, content_style, alignment) = if app.searching {
        (
            format!("{} Searching...", app.spinner_frame()),
            Style::default().fg(Color::Yellow),
            Alignment::Left,
        )
    } else if app.search_input.is_empty() && app.mode != Mode::Searching {
        // Placeholder — centered, dark color
        (
            "block height / hash / txid / address".to_string(),
            Style::default().fg(Color::DarkGray),
            Alignment::Center,
        )
    } else {
        (
            app.search_input.clone(),
            Style::default().fg(Color::White),
            Alignment::Left,
        )
    };

    let input = Paragraph::new(content.as_str())
        .style(content_style)
        .alignment(alignment)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(" Search "),
        );
    frame.render_widget(input, area);

    // Show cursor only when actively typing
    if app.mode == Mode::Searching && !app.searching {
        frame.set_cursor_position((
            area.x + app.search_input.len() as u16 + 1,
            area.y + 1,
        ));
    }
}

// ─── Block list ──────────────────────────────────────────────────────────────

fn render_block_list(frame: &mut Frame, app: &App, area: Rect) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Latest Blocks — newest first ");

    if app.blocks.is_empty() {
        let msg = Paragraph::new("\n  Syncing blocks from Bitcoin Core...")
            .style(Style::default().fg(Color::DarkGray))
            .block(outer);
        frame.render_widget(msg, area);
        return;
    }

    // Render the outer border first
    frame.render_widget(outer, area);

    // Center the card container within the inner area
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    // Card width — 90% of inner width, capped at 96
    let card_width = (inner.width * 90 / 100).min(96).max(40);

    // Centered x position for cards
    let card_x = inner.x + inner.width.saturating_sub(card_width) / 2;

    // During intro: reveal oldest→newest
    // After intro: show all newest→oldest
    let blocks_to_show: Vec<&crate::tui::app::BlockRow> = match app.intro_reveal {
        Some(revealed) => {
            let total = app.blocks.len();
            let skip = total.saturating_sub(revealed);
            app.blocks.iter().skip(skip).collect()
        }
        None => app.blocks.iter().collect(),
    };

    // Each card is 5 lines + 1 gap = 6 lines per block
    const CARD_HEIGHT: u16 = 5;
    const CARD_GAP: u16 = 1;

    for (i, block) in blocks_to_show.iter().enumerate() {
        let card_top = inner.y as i32 + (i as i32) * (CARD_HEIGHT as i32 + CARD_GAP as i32);

        // Apply scroll offset
        let scrolled_top = card_top - app.block_scroll as i32;

        // Skip cards fully above the visible area
        if scrolled_top + CARD_HEIGHT as i32 <= inner.y as i32 {
            continue;
        }
        // Stop rendering cards fully below the visible area
        if scrolled_top >= (inner.y + inner.height) as i32 {
            break;
        }

        // Card must be within valid screen bounds
        if scrolled_top < 0 {
            continue;
        }

        let offset = if i == 0 { app.intro_anim_offset } else { 0 };

        let slide_x = (card_x as i32 + offset as i32)
            .max(inner.x as i32)
            .min((inner.x + inner.width).saturating_sub(1) as i32) as u16;

        // Clip card height to not exceed the inner area bottom
        let max_y = inner.y + inner.height;
        let actual_y = scrolled_top as u16;
        let available_height = max_y.saturating_sub(actual_y).min(CARD_HEIGHT);

        if available_height == 0 {
            break;
        }

        // Clip card width to not exceed the terminal buffer
        let terminal_area = frame.area();
        let available_width = terminal_area
            .width
            .saturating_sub(slide_x)
            .min(card_width)
            .min(inner.width);

        if available_width == 0 {
            continue;
        }

        let card_rect = Rect {
            x: slide_x,
            y: actual_y,
            width: available_width,
            height: available_height,
        };

        render_block_card(frame, block, card_rect);
    }

    // Hover detection — show popup for whichever card the mouse is over
    let mut hovered: Option<&crate::tui::app::BlockRow> = None;
    for (i, block) in blocks_to_show.iter().enumerate() {
        let card_top = inner.y as i32 + (i as i32) * (CARD_HEIGHT as i32 + CARD_GAP as i32);
        let scrolled_top = card_top - app.block_scroll as i32;
        if scrolled_top < 0 { continue; }
        let card_bottom = scrolled_top as u16 + CARD_HEIGHT;
        let card_left = inner.x + inner.width.saturating_sub(card_width) / 2;
        let card_right = card_left + card_width;

        if app.mouse_y >= scrolled_top as u16
            && app.mouse_y < card_bottom
            && app.mouse_x >= card_left
            && app.mouse_x < card_right
        {
            hovered = Some(block);
            break;
        }
    }

    if let Some(block) = hovered {
        render_block_hover_popup(frame, block, app.mouse_x, app.mouse_y, area);
    }
}
fn render_block_card(frame: &mut Frame, block: &crate::tui::app::BlockRow, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    // Hard guard: never render outside the terminal buffer
    let terminal_area = frame.area();
    if area.x >= terminal_area.width || area.y >= terminal_area.height {
        return;
    }

    let w = area.width as usize;
    let inner_w = w.saturating_sub(6); // account for ║  and  ║

    let lines = vec![
        Line::from(Span::styled(
            format!("╔{:═<width$}╗", "", width = w.saturating_sub(2)),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled("║  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("BLOCK #{}", block.height),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "  —  {} tx  —  {} bytes{:>width$}  ║",
                    block.tx_count, block.size, "",
                    width = inner_w.saturating_sub(
                        format!("BLOCK #{}  —  {} tx  —  {} bytes", block.height, block.tx_count, block.size).len()
                    )
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("║  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Hash:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:<width$}", truncate(&block.hash, inner_w.saturating_sub(7)), width = inner_w.saturating_sub(7)),
                Style::default().fg(Color::White),
            ),
            Span::styled("  ║", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("║  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Time:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:<width$}", block.timestamp, width = inner_w.saturating_sub(7)),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled("  ║", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(Span::styled(
            format!("╚{:═<width$}╝", "", width = w.saturating_sub(2)),
            Style::default().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

// ─── Results panel ───────────────────────────────────────────────────────────

fn render_results(frame: &mut Frame, app: &App, area: Rect) {
    let block_widget = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Results ");

    match &app.result {
        None => {
            frame.render_widget(
                Paragraph::new("").block(block_widget),
                area,
            );
        }

        Some(SearchResult::NotFound(msg)) => {
            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "  ✖  NOT FOUND",
                        Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(msg.clone(), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  Make sure you entered one of:",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "    • A block height         e.g.  5",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "    • A block hash (64 hex)  e.g.  0f9188f13cb7...",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "    • A transaction ID       e.g.  4a5e1e4baab8...",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "    • A bitcoin address      e.g.  bcrt1q...",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Esc to go back to the block list.",
                    Style::default().fg(Color::Yellow),
                )),
            ];
            render_scrollable(frame, area, block_widget, lines, 0);
        }

        Some(SearchResult::Error(msg)) => {
            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "  ✖  ERROR",
                        Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(msg.clone(), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Esc to go back to the block list.",
                    Style::default().fg(Color::Yellow),
                )),
            ];
            render_scrollable(frame, area, block_widget, lines, 0);
        }

        Some(SearchResult::Block(b)) => {
            render_scrollable(frame, area, block_widget, render_block_lines(b), app.scroll);
        }

        Some(SearchResult::Transaction(t)) => {
            render_scrollable(frame, area, block_widget, render_tx_lines(t), app.scroll);
        }

        Some(SearchResult::Address(a)) => {
            render_scrollable(frame, area, block_widget, render_address_lines(a), app.scroll);
        }
    }
}

fn render_scrollable(
    frame: &mut Frame,
    area: Rect,
    block_widget: Block,
    lines: Vec<Line<'static>>,
    scroll: u16,
) {
    let total_lines = lines.len() as u16;
    let inner_height = area.height.saturating_sub(2);

    let para = Paragraph::new(Text::from(lines))
        .block(block_widget)
        .scroll((scroll, 0));
    frame.render_widget(para, area);

    if total_lines > inner_height {
        let mut state =
            ScrollbarState::new(total_lines as usize).position(scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut state,
        );
    }
}

// ─── Copy overlay ─────────────────────────────────────────────────────────────

fn render_copy_overlay(frame: &mut Frame, app: &App, area: Rect) {
    // Centre a small popup
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = (app.copy_fields.len() as u16 + 4).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = app
        .copy_fields
        .iter()
        .map(|(label, value)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{label}: "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    truncate(value, (popup_width as usize).saturating_sub(label.len() + 4)),
                    Style::default().fg(Color::White),
                ),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.copy_selected));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Select field to copy — [↑↓] Navigate  [Enter] Copy  [Esc] Cancel "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, popup_area, &mut list_state);
}

// ─── Status bar ──────────────────────────────────────────────────────────────

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let keys = if app.mode == Mode::Searching {
        ""
    } else if app.mode == Mode::Copying {
        ""
    } else {
        match (&app.result, &app.screen) {
            (_, crate::tui::app::Screen::Mempool) => {
                "  [↑↓] Scroll   [b] Back to blocks   [q] Quit"
            }
            (Some(crate::tui::app::SearchResult::NotFound(_)), _)
            | (Some(crate::tui::app::SearchResult::Error(_)), _) => {
                "  [Esc] Back to blocks   [/] New search   [q] Quit"
            }
            (Some(_), _) => "  [↑↓] Scroll   [c] Copy   [Esc] Back   [q] Quit",
            (None, _) => {
                let is_mainnet = app.network_stats.as_ref()
                    .map(|s| s.chain == "main")
                    .unwrap_or(false);
                if is_mainnet {
                    "  [↑↓] Scroll   [q] Quit"
                } else {
                    "  [↑↓] Scroll   [m] Mempool   [q] Quit"
                }
            }
        }
    };

    let line = Line::from(vec![
        Span::styled(&app.status, Style::default().fg(Color::DarkGray)),
        Span::styled(
            keys,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        ),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

// ─── Block detail rendering ───────────────────────────────────────────────────

fn render_block_lines(b: &crate::tui::app::BlockResult) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  BLOCK  ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  #{}", b.height),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        label_value("  Hash:       ", &b.hash),
        label_value("  Prev Hash:  ", &b.prev_hash),
        label_value("  Timestamp:  ", &format!("{}", b.timestamp)),
        label_value("  Size:       ", &format!("{} bytes", b.size)),
        label_value("  Tx Count:   ", &format!("{}", b.tx_count)),
        Line::from(""),
        Line::from(Span::styled(
            "  TRANSACTIONS",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for tx in &b.transactions {
        lines.extend(render_tx_lines(tx));
    }

    lines
}

fn render_tx_lines(t: &crate::tui::app::TxResult) -> Vec<Line<'static>> {
    let label = if t.is_coinbase { "COINBASE" } else { "TX" };
    let label_color = if t.is_coinbase {
        Color::Green
    } else {
        Color::Cyan
    };

    let mut lines = vec![
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                label,
                Style::default()
                    .fg(label_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(t.txid.clone(), Style::default().fg(Color::White)),
        ]),
        label_value("    Block:    ", &format!("#{}", t.block_height)),
        Line::from(""),
        Line::from(Span::styled(
            "    INPUTS",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    for input in &t.inputs {
        if let (Some(prev_txid), Some(prev_vout)) = (&input.prev_txid, input.prev_vout) {
            lines.push(Line::from(vec![
                Span::raw(format!("      #{} ", input.input_index)),
                Span::styled(prev_txid.clone(), Style::default().fg(Color::White)),
                Span::styled(
                    format!(":{prev_vout}"),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw(format!("      #{} ", input.input_index)),
                Span::styled("coinbase  ", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("data: {}", truncate(&input.script_sig, 40)),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "    OUTPUTS",
        Style::default().fg(Color::DarkGray),
    )));

    for output in &t.outputs {
        let addr = output
            .address
            .clone()
            .unwrap_or_else(|| "no address".to_string());
        let script = output.script_type.clone().unwrap_or_default();
        let spent = if output.is_spent { " [spent]" } else { "" };
        lines.push(Line::from(vec![
            Span::raw(format!("      #{} ", output.output_index)),
            Span::styled(
                format_sats(output.value_sats),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  →  "),
            Span::styled(addr, Style::default().fg(Color::White)),
            Span::styled(
                format!(" ({script}){spent}"),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines
}

fn render_address_lines(a: &crate::tui::app::AddressResult) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  ADDRESS",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        label_value("  Address:        ", &a.address),
        label_value("  Balance:        ", &format_sats(a.balance_sats)),
        label_value("  Total Received: ", &format_sats(a.total_received_sats)),
        label_value("  Tx Count:       ", &format!("{}", a.tx_count)),
        Line::from(""),
        Line::from(Span::styled(
            "  UTXOS",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if a.utxos.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No unspent outputs.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for utxo in &a.utxos {
            lines.push(Line::from(vec![
                Span::raw(format!("  vout #{} ", utxo.output_index)),
                Span::styled(
                    format_sats(utxo.value_sats),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn label_value(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(Color::DarkGray)),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

fn format_sats(sats: i64) -> String {
    let btc = sats as f64 / 100_000_000.0;
    format!("{btc:.8} BTC")
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

// ─── Block hover popup ────────────────────────────────────────────────────────

fn render_block_hover_popup(
    frame: &mut Frame,
    block: &crate::tui::app::BlockRow,
    mouse_x: u16,
    mouse_y: u16,
    area: Rect,
) {
    let terminal = frame.area();
    let popup_width = 52u16;
    let popup_height = 9u16;

    // Position popup to the right of cursor, flip left if near right edge
    let x = if mouse_x + popup_width + 2 < terminal.width {
        mouse_x + 2
    } else {
        mouse_x.saturating_sub(popup_width + 2)
    };

    // Position popup below cursor, flip up if near bottom
    let y = if mouse_y + popup_height + 1 < terminal.height {
        mouse_y + 1
    } else {
        mouse_y.saturating_sub(popup_height + 1)
    };

    let popup_rect = Rect {
        x: x.min(terminal.width.saturating_sub(popup_width)),
        y: y.min(terminal.height.saturating_sub(popup_height)),
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_rect);

    let accent = Color::Cyan;
    let w = popup_rect.width as usize;

    let lines = vec![
        Line::from(Span::styled(
            format!("┌{:─<width$}┐", "", width = w.saturating_sub(2)),
            Style::default().fg(accent),
        )),
        Line::from(vec![
            Span::styled("│ ", Style::default().fg(accent)),
            Span::styled(
                format!("{:^width$}", format!("BLOCK #{}", block.height), width = w.saturating_sub(4)),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │", Style::default().fg(accent)),
        ]),
        Line::from(Span::styled(
            format!("├{:─<width$}┤", "", width = w.saturating_sub(2)),
            Style::default().fg(accent),
        )),
        Line::from(vec![
            Span::styled("│ ", Style::default().fg(accent)),
            Span::styled("Hash:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(truncate(&block.hash, w.saturating_sub(12)), Style::default().fg(Color::White)),
            Span::styled(" │", Style::default().fg(accent)),
        ]),
        Line::from(vec![
            Span::styled("│ ", Style::default().fg(accent)),
            Span::styled("Time:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(block.timestamp.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:>width$} │", "", width = w.saturating_sub(10 + block.timestamp.to_string().len())),
                Style::default().fg(accent),
            ),
        ]),
        Line::from(vec![
            Span::styled("│ ", Style::default().fg(accent)),
            Span::styled("Txs:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(block.tx_count.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:>width$} │", "", width = w.saturating_sub(10 + block.tx_count.to_string().len())),
                Style::default().fg(accent),
            ),
        ]),
        Line::from(vec![
            Span::styled("│ ", Style::default().fg(accent)),
            Span::styled("Size:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} bytes", block.size), Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:>width$} │", "", width = w.saturating_sub(10 + format!("{} bytes", block.size).len())),
                Style::default().fg(accent),
            ),
        ]),
        Line::from(Span::styled(
            format!("├{:─<width$}┤", "", width = w.saturating_sub(2)),
            Style::default().fg(accent),
        )),
        Line::from(vec![
            Span::styled("│ ", Style::default().fg(accent)),
            Span::styled(
                format!("{:^width$}", "Press / to search this block", width = w.saturating_sub(4)),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(" │", Style::default().fg(accent)),
        ]),
        Line::from(Span::styled(
            format!("└{:─<width$}┘", "", width = w.saturating_sub(2)),
            Style::default().fg(accent),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), popup_rect);
}

// ─── Mempool panel ───────────────────────────────────────────────────────────

fn render_mempool(frame: &mut Frame, app: &App, area: Rect) {
    let count = app.mempool_txs.len();
    let title = format!(" Mempool — {count} pending  |  [b] Back to blocks ");

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(title, Style::default().fg(Color::Cyan)));

    frame.render_widget(outer, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    if app.mempool_txs.is_empty() {
        let msg = Paragraph::new("\n  Mempool is empty.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    }

    let card_width = (inner.width * 90 / 100).min(96).max(40);
    let card_x = inner.x + inner.width.saturating_sub(card_width) / 2;
    const CARD_HEIGHT: u16 = 4;
    const CARD_GAP: u16 = 1;

    for (i, tx) in app.mempool_txs.iter().enumerate() {
        let card_top = inner.y as i32 + (i as i32) * (CARD_HEIGHT as i32 + CARD_GAP as i32);
        let scrolled_top = card_top - app.mempool_scroll as i32;

        if scrolled_top + CARD_HEIGHT as i32 <= inner.y as i32 { continue; }
        if scrolled_top >= (inner.y + inner.height) as i32 { break; }
        if scrolled_top < 0 { continue; }

        let terminal_area = frame.area();
        if scrolled_top as u16 >= terminal_area.height { break; }

        let available_height = (inner.y + inner.height)
            .saturating_sub(scrolled_top as u16)
            .min(CARD_HEIGHT);
        let available_width = terminal_area
            .width
            .saturating_sub(card_x)
            .min(card_width);

        if available_height == 0 || available_width == 0 { continue; }

        let card_rect = Rect {
            x: card_x,
            y: scrolled_top as u16,
            width: available_width,
            height: available_height,
        };

        render_mempool_card(frame, tx, card_rect);
    }
}

fn render_mempool_card(frame: &mut Frame, tx: &crate::tui::app::MempoolTx, area: Rect) {
    if area.height == 0 || area.width == 0 { return; }
    let terminal_area = frame.area();
    if area.x >= terminal_area.width || area.y >= terminal_area.height { return; }

    let w = area.width as usize;
    let border_color = if tx.is_new { Color::Yellow } else { Color::DarkGray };
    let label = if tx.is_new { "⚡ NEW  " } else { "       " };

    let lines = vec![
        Line::from(Span::styled(
            format!("╔{:═<width$}╗", "", width = w.saturating_sub(2)),
            Style::default().fg(border_color),
        )),
        Line::from(vec![
            Span::styled("║  ", Style::default().fg(border_color)),
            Span::styled(label, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(
                truncate(&tx.txid, w.saturating_sub(12)),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("{:>width$}  ║", "", width = w.saturating_sub(
                    11 + label.len() + truncate(&tx.txid, w.saturating_sub(12)).len()
                )),
                Style::default().fg(border_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("║  ", Style::default().fg(border_color)),
            Span::styled("Fee: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} sat", tx.fee_sats),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("   Size: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} bytes", tx.size),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!("{:>width$}  ║", "", width = w.saturating_sub(
                    30 + format!("{} sat", tx.fee_sats).len() + format!("{} bytes", tx.size).len()
                )),
                Style::default().fg(border_color),
            ),
        ]),
        Line::from(Span::styled(
            format!("╚{:═<width$}╝", "", width = w.saturating_sub(2)),
            Style::default().fg(border_color),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

/// Notification card shown at the center of the block list screen when a new tx arrives.
fn render_mempool_notification(
    frame: &mut Frame,
    tx: &crate::tui::app::MempoolTx,
    area: Rect,
) {
    let terminal = frame.area();

    // Dim the entire block list area first — draw a dark overlay row by row
    for row in area.y..area.y + area.height {
        if row >= terminal.height { break; }
        let row_rect = Rect { x: area.x, y: row, width: area.width, height: 1 };
        frame.render_widget(
            Paragraph::new(" ".repeat(area.width as usize).as_str())
                .style(Style::default().add_modifier(Modifier::DIM)),
            row_rect,
        );
    }

    // Notification box — 75% width, 9 lines tall
    let notif_width = (area.width * 75 / 100).max(60);
    let notif_height = 9u16;
    let x = area.x + area.width.saturating_sub(notif_width) / 2;
    let y = area.y + area.height.saturating_sub(notif_height) / 2;

    let notif_rect = Rect {
        x,
        y,
        width: notif_width.min(terminal.width.saturating_sub(x)),
        height: notif_height.min(terminal.height.saturating_sub(y)),
    };

    if notif_rect.width == 0 || notif_rect.height == 0 { return; }

    frame.render_widget(Clear, notif_rect);

    let w = notif_rect.width as usize;
    let inner_w = w.saturating_sub(4);

    let pink = Color::Gray;

    let lines = vec![
        Line::from(Span::styled(
            format!("┌{:─<width$}┐", "", width = w.saturating_sub(2)),
            Style::default().fg(pink),
        )),
        Line::from(vec![
            Span::styled("│", Style::default().fg(pink)),
            Span::styled(
                format!("{:^width$}", "", width = w.saturating_sub(2)),
                Style::default(),
            ),
            Span::styled("│", Style::default().fg(pink)),
        ]),
        Line::from(vec![
            Span::styled("│", Style::default().fg(pink)),
            Span::styled(
                format!("{:^width$}", "⚡  New Transaction Entering Mempool  ⚡", width = w.saturating_sub(2)),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled("│", Style::default().fg(pink)),
        ]),
        Line::from(vec![
            Span::styled("│", Style::default().fg(pink)),
            Span::styled(
                format!("{:^width$}", "", width = w.saturating_sub(2)),
                Style::default(),
            ),
            Span::styled("│", Style::default().fg(pink)),
        ]),
        Line::from(Span::styled(
            format!("├{:─<width$}┤", "", width = w.saturating_sub(2)),
            Style::default().fg(pink),
        )),
        Line::from(vec![
            Span::styled("│  ", Style::default().fg(pink)),
            Span::styled("TXID:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                truncate(&tx.txid, inner_w.saturating_sub(10)),
                Style::default().fg(Color::White),
            ),
            Span::styled("  │", Style::default().fg(pink)),
        ]),
        Line::from(vec![
            Span::styled("│  ", Style::default().fg(pink)),
            Span::styled("Fee:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} sat", tx.fee_sats), Style::default().fg(Color::Cyan)),
            Span::styled("   Size:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} bytes", tx.size), Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:>width$}  │", "",
                    width = inner_w.saturating_sub(
                        format!("Fee:      {} sat   Size:  {} bytes", tx.fee_sats, tx.size).len()
                    )
                ),
                Style::default().fg(pink),
            ),
        ]),
        Line::from(vec![
            Span::styled("│  ", Style::default().fg(pink)),
            Span::styled("Arrived:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(tx.arrived_at.clone(), Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:>width$}  │", "",
                    width = inner_w.saturating_sub(10 + tx.arrived_at.len())
                ),
                Style::default().fg(pink),
            ),
        ]),
        Line::from(Span::styled(
            format!("└{:─<width$}┘", "", width = w.saturating_sub(2)),
            Style::default().fg(pink),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), notif_rect);
    
    // Hint below the box
    let hint_y = y + notif_height;
    if hint_y < terminal.height {
        let hint_rect = Rect { x, y: hint_y, width: notif_width, height: 1 };
        frame.render_widget(
            Paragraph::new(
                Line::from(Span::styled(
                    format!("{:^width$}", "Press [m] to view all mempool transactions", width = notif_width as usize),
                    Style::default().fg(Color::DarkGray),
                ))
            ),
            hint_rect,
        );
    }
}

// ─── Network Select screen ────────────────────────────────────────────────────

fn render_network_select(frame: &mut Frame, app: &App, area: Rect) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Select Network  |  [↑↓] Navigate   [Enter] Confirm   [b] Back ");

    frame.render_widget(outer, area);

    let current_chain = app.network_stats
        .as_ref()
        .map(|s| s.chain.as_str())
        .unwrap_or("regtest");

    let networks = vec![
        ("Regtest",  "Local development network (Polar)",  "regtest"),
        ("Mainnet",  "Bitcoin mainnet via Chainstack",      "main"),
    ];

    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let x = area.x + area.width.saturating_sub(popup_width) / 2;
    let y = area.y + 2;
    let w = popup_width as usize;

    let mut lines = vec![Line::from(""), Line::from("")];

    for (i, (name, desc, chain)) in networks.iter().enumerate() {
        let is_selected = i == app.network_select_idx;
        let is_active = *chain == current_chain;
        let border_color = if is_selected { Color::Green } else { Color::DarkGray };
        let name_color = if is_selected { Color::Green } else { Color::White };
        let active_label = if is_active { "  ◀ ACTIVE" } else { "" };

        lines.push(Line::from(Span::styled(
            format!("┌{:─<width$}┐", "", width = w.saturating_sub(2)),
            Style::default().fg(border_color),
        )));
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(border_color)),
            Span::styled(
                format!("{}{}", name, active_label),
                Style::default().fg(name_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>width$} │", "", width = w.saturating_sub(4 + name.len() + active_label.len())),
                Style::default().fg(border_color),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(border_color)),
            Span::styled(
                format!("{:<width$}", desc, width = w.saturating_sub(4)),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(" │", Style::default().fg(border_color)),
        ]));
        lines.push(Line::from(Span::styled(
            format!("└{:─<width$}┘", "", width = w.saturating_sub(2)),
            Style::default().fg(border_color),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        format!("{:^width$}", "To switch networks: update .env and restart", width = w),
        Style::default().fg(Color::DarkGray),
    )));

    let content_rect = Rect {
        x: x.max(area.x + 1),
        y: y.max(area.y + 1),
        width: popup_width.min(area.width.saturating_sub(2)),
        height: area.height.saturating_sub(4),
    };

    frame.render_widget(Paragraph::new(Text::from(lines)), content_rect);
}

fn render_network_stats(frame: &mut Frame, app: &App, area: Rect) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Network Stats  |  [b] Back to blocks ");

    match &app.network_stats {
        None => {
            let msg = Paragraph::new("\n  Loading network stats...")
                .style(Style::default().fg(Color::DarkGray))
                .block(outer);
            frame.render_widget(msg, area);
        }
        Some(stats) => {
            frame.render_widget(outer, area);

            let inner = Rect {
                x: area.x + 2,
                y: area.y + 2,
                width: area.width.saturating_sub(4),
                height: area.height.saturating_sub(4),
            };

            let card_width = (inner.width * 90 / 100).min(90).max(50);
            let card_x = inner.x + inner.width.saturating_sub(card_width) / 2;
            let w = card_width as usize;

            let hashps_display = format_hashrate(stats.network_hashps);
            let difficulty_display = format_difficulty(stats.difficulty);
            let subsidy_btc = stats.block_subsidy_sats as f64 / 100_000_000.0;
            let adjustment_hrs = stats.next_adjustment_blocks * 10 / 60;
            let height_str = stats.height.to_string();
            let mempool_str = stats.mempool_tx_count.to_string();
            let subsidy_str = format!("{:.8} BTC", subsidy_btc);
            let adjustment_str = format!("{} blocks (~{} hours)", stats.next_adjustment_blocks, adjustment_hrs);
            let avg_block_str = format!("{:.1} seconds", stats.avg_block_time_secs);
            let chain_str = stats.chain.to_uppercase();

            let lines = vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    format!("{:^width$}", "─── Bitcoin Network ───", width = w),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                stat_line("Chain:               ", &chain_str, Color::Yellow),
                stat_line("Block Height:        ", &height_str, Color::White),
                stat_line("Network Hashrate:    ", &hashps_display, Color::Green),
                stat_line("Difficulty:          ", &difficulty_display, Color::White),
                stat_line("Mempool Txs:         ", &mempool_str, Color::Cyan),
                Line::from(""),
                Line::from(vec![Span::styled(
                    format!("{:^width$}", "─── Mining ───", width = w),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                stat_line("Block Subsidy:       ", &subsidy_str, Color::Yellow),
                stat_line("Next Adjustment:     ", &adjustment_str, Color::White),
                stat_line("Avg Block Time:      ", &avg_block_str, Color::White),
                Line::from(""),
                Line::from(vec![Span::styled(
                    format!("{:^width$}", "Refreshes every 30 seconds", width = w),
                    Style::default().fg(Color::DarkGray),
                )]),
            ];

            let stats_rect = Rect {
                x: card_x,
                y: inner.y,
                width: card_width,
                height: inner.height,
            };

            frame.render_widget(Paragraph::new(Text::from(lines)), stats_rect);
        }
    }
}

fn stat_line<'a>(label: &'a str, value: &'a str, value_color: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(label, Style::default().fg(Color::DarkGray)),
        Span::styled(value, Style::default().fg(value_color).add_modifier(Modifier::BOLD)),
    ])
}

fn format_hashrate(hashps: f64) -> String {
    if hashps >= 1e18 {
        format!("{:.2} EH/s", hashps / 1e18)
    } else if hashps >= 1e15 {
        format!("{:.2} PH/s", hashps / 1e15)
    } else if hashps >= 1e12 {
        format!("{:.2} TH/s", hashps / 1e12)
    } else if hashps >= 1e9 {
        format!("{:.2} GH/s", hashps / 1e9)
    } else if hashps >= 1e6 {
        format!("{:.2} MH/s", hashps / 1e6)
    } else {
        format!("{:.0} H/s", hashps)
    }
}

fn format_difficulty(diff: f64) -> String {
    if diff >= 1e12 {
        format!("{:.2} T", diff / 1e12)
    } else if diff >= 1e9 {
        format!("{:.2} B", diff / 1e9)
    } else if diff >= 1e6 {
        format!("{:.2} M", diff / 1e6)
    } else {
        format!("{:.2}", diff)
    }
}

// ─── Right panels ─────────────────────────────────────────────────────────────

fn render_right_panels(frame: &mut Frame, app: &App, area: Rect) {
    let panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 3), // Network Stats — equal third
            Constraint::Ratio(1, 3), // Top Pools — equal third
            Constraint::Ratio(1, 3), // Miner Performance — equal third
        ])
        .split(area);

    render_network_stats_panel(frame, app, panels[0]);
    render_top_pools_panel(frame, app, panels[1]);
    render_miner_perf_panel(frame, app, panels[2]);
}

fn render_network_stats_panel(frame: &mut Frame, app: &App, area: Rect) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Network Stats ");
    match &app.network_stats {
        None => {
            frame.render_widget(
                Paragraph::new("  Loading...").style(Style::default().fg(Color::DarkGray)).block(outer),
                area,
            );
        }
        Some(stats) => {
            let hashps = format_hashrate(stats.network_hashps);
            let diff = format_difficulty(stats.difficulty);
            let subsidy = stats.block_subsidy_sats as f64 / 100_000_000.0;
            let subsidy_str = format!("{:.3} BTC", subsidy);
            let adj_str = format!("{} blocks", stats.next_adjustment_blocks);
            let mempool_str = format!("{} txs", stats.mempool_tx_count);
            let lines = vec![
                stat_line("  Hashrate:   ", &hashps, Color::Green),
                stat_line("  Difficulty: ", &diff, Color::White),
                stat_line("  Next adj:   ", &adj_str, Color::White),
                stat_line("  Subsidy:    ", &subsidy_str, Color::Yellow),
                stat_line("  Mempool:    ", &mempool_str, Color::Cyan),
            ];
            frame.render_widget(
                Paragraph::new(Text::from(lines)).block(outer),
                area,
            );
        }
    }
}

fn render_top_pools_panel(frame: &mut Frame, app: &App, area: Rect) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Top Pools — click to expand ");

    if app.pools.is_empty() {
        frame.render_widget(
            Paragraph::new("  No pool data yet...").style(Style::default().fg(Color::DarkGray)).block(outer),
            area,
        );
        return;
    }

    let mut lines = vec![Line::from("")];
    for pool in app.pools.iter().take(5) {
        let bar_len = ((pool.share_pct / 100.0) * 12.0) as usize;
        let bar = "█".repeat(bar_len);
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<14}", truncate(&pool.name, 14)), Style::default().fg(Color::White)),
            Span::styled(format!("{:>5.1}%  ", pool.share_pct), Style::default().fg(Color::Yellow)),
            Span::styled(bar, Style::default().fg(Color::Cyan)),
        ]));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)).block(outer), area);

    // Detect click on this panel
    if app.mouse_y >= area.y && app.mouse_y < area.y + area.height
        && app.mouse_x >= area.x && app.mouse_x < area.x + area.width {
        // Click handled in events.rs
    }
}

fn render_miner_perf_panel(frame: &mut Frame, app: &App, area: Rect) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Miner Performance — click to expand ");

    if app.miner_perfs.is_empty() {
        frame.render_widget(
            Paragraph::new("  No miner data yet...").style(Style::default().fg(Color::DarkGray)).block(outer),
            area,
        );
        return;
    }

    let mut lines = vec![Line::from("")];
    for miner in app.miner_perfs.iter().take(3) {
        let status = if miner.gap_hours.is_some() { "⚠" } else { "✔" };
        let color = if miner.gap_hours.is_some() { Color::Yellow } else { Color::Green };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", status), Style::default().fg(color)),
            Span::styled(truncate(&miner.address, 20), Style::default().fg(Color::White)),
            Span::styled(
                format!("  {:.2} EH/s", miner.est_hashrate_ehs),
                Style::default().fg(Color::Cyan),
            ),
        ]));
        if let Some(gap) = miner.gap_hours {
            lines.push(Line::from(Span::styled(
                format!("    GAP: {:.1}h", gap),
                Style::default().fg(Color::Yellow),
            )));
        }
    }

    frame.render_widget(Paragraph::new(Text::from(lines)).block(outer), area);
}

// ─── Network Stats dedicated screen ──────────────────────────────────────────

fn render_network_detail(frame: &mut Frame, app: &App, area: Rect) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Network Stats  |  [Esc] Back ");
    frame.render_widget(outer, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    match &app.network_stats {
        None => {
            frame.render_widget(
                Paragraph::new("\n  Loading network stats...").style(Style::default().fg(Color::DarkGray)),
                inner,
            );
        }
        Some(stats) => {
            let hashps = format_hashrate(stats.network_hashps);
            let diff = format_difficulty(stats.difficulty);
            let subsidy = stats.block_subsidy_sats as f64 / 100_000_000.0;
            let adj_hrs = stats.next_adjustment_blocks * 10 / 60;
            let adj_days = adj_hrs / 24;
            let height_str = stats.height.to_string();
            let mempool_str = format!("{} transactions", stats.mempool_tx_count);
            let subsidy_str = format!("{:.3} BTC", subsidy);
            let adj_str = format!("{} blocks (~{} days)", stats.next_adjustment_blocks, adj_days);
            let chain_str = stats.chain.to_uppercase();
            let avg_block_str = format!("{:.1} seconds", stats.avg_block_time_secs);

            let lines = vec![
                Line::from(""),
                Line::from(Span::styled("  ─── Bitcoin Network ───", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from(""),
                stat_line("  Chain:                 ", &chain_str, Color::Yellow),
                stat_line("  Block Height:          ", &height_str, Color::White),
                stat_line("  Network Hashrate:      ", &hashps, Color::Green),
                stat_line("  Difficulty:            ", &diff, Color::White),
                stat_line("  Mempool:               ", &mempool_str, Color::Cyan),
                Line::from(""),
                Line::from(Span::styled("  ─── Mining Economics ───", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from(""),
                stat_line("  Block Subsidy:         ", &subsidy_str, Color::Yellow),
                stat_line("  Next Adjustment:       ", &adj_str, Color::White),
                stat_line("  Avg Block Time:        ", &avg_block_str, Color::White),
                Line::from(""),
                Line::from(Span::styled("  ─── Block Production ───", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from(""),
                stat_line("  Blocks expected/day:   ", "144", Color::White),
                stat_line("  Subsidy halving every: ", "210,000 blocks", Color::White),
            ];

            frame.render_widget(Paragraph::new(Text::from(lines)), inner);
        }
    }
}

fn render_pool_detail(frame: &mut Frame, app: &App, area: Rect) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Pool Intelligence  |  [Esc] Back   [↑↓] Scroll ");
    frame.render_widget(outer, area);

    let inner = Rect { x: area.x + 1, y: area.y + 1, width: area.width.saturating_sub(2), height: area.height.saturating_sub(2) };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {:<18} {:>7}  {:>7}  {:>12}  {:>12}", "Pool", "Blocks", "Share", "Est Hashrate", "Avg Interval"),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(format!("  {}", "─".repeat(68)), Style::default().fg(Color::DarkGray))),
    ];

    for pool in &app.pools {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<18}", truncate(&pool.name, 18)), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {:>7}", pool.blocks), Style::default().fg(Color::Cyan)),
            Span::styled(format!("  {:>6.1}%", pool.share_pct), Style::default().fg(Color::Yellow)),
            Span::styled(format!("  {:>10.2} EH/s", pool.est_hashrate_ehs), Style::default().fg(Color::Green)),
            Span::styled(format!("  {:>9.1} min", pool.avg_interval_mins), Style::default().fg(Color::White)),
        ]));
    }

    // Miner contributions per pool
    for (pool_name, miners) in &app.pool_miners {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                format!("  ── {} — Miner Addresses ──", pool_name),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<46} {:>7}  {:>7}  {:>14}", "Address", "Blocks", "Share", "Est Revenue"),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(Span::styled(format!("  {}", "─".repeat(82)), Style::default().fg(Color::DarkGray))));

        for miner in miners {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<46}", truncate(&miner.address, 46)), Style::default().fg(Color::White)),
                Span::styled(format!(" {:>7}", miner.blocks), Style::default().fg(Color::Cyan)),
                Span::styled(format!("  {:>6.1}%", miner.share_pct), Style::default().fg(Color::Yellow)),
                Span::styled(format!("  {:>10.3} BTC", miner.est_revenue_btc), Style::default().fg(Color::Yellow)),
            ]));
        }
    }

    let total = lines.len() as u16;
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((app.detail_scroll, 0)),
        inner,
    );

    if total > inner.height {
        let mut state = ratatui::widgets::ScrollbarState::new(total as usize).position(app.detail_scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut state,
        );
    }
}

// ─── Miner Performance detail screen ─────────────────────────────────────────

fn render_miner_detail(frame: &mut Frame, app: &App, area: Rect) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Miner Performance Intelligence  |  [Esc] Back   [↑↓] Scroll ");
    frame.render_widget(outer, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    if app.miner_perfs.is_empty() {
        frame.render_widget(
            Paragraph::new("\n  No miner data available yet. Waiting for blocks to be indexed.")
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let mut lines = vec![Line::from("")];
    let w = inner.width as usize;

    for miner in &app.miner_perfs {
        let is_degraded = miner.gap_hours.is_some();
        let status_icon = if is_degraded { "⚠ GAP DETECTED" } else { "✔ HEALTHY" };
        let status_color = if is_degraded { Color::Yellow } else { Color::Green };
        let border_color = Color::Green;

        let addr = truncate(&miner.address, w.saturating_sub(22));
        let status_pad = w.saturating_sub(addr.len() + status_icon.len() + 8);

        lines.push(Line::from(Span::styled(
            format!("  ┌{:─<width$}┐", "", width = w.saturating_sub(4)),
            Style::default().fg(border_color),
        )));
        lines.push(Line::from(vec![
            Span::styled("  │ ", Style::default().fg(border_color)),
            Span::styled(addr.clone(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {}", status_icon), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:>width$}  │", "", width = status_pad), Style::default().fg(border_color)),
        ]));

        let blocks_str = format!("{}", miner.blocks);
        let hash_str = format!("{:.2} EH/s", miner.est_hashrate_ehs);
        let b_pad = w.saturating_sub(blocks_str.len() + hash_str.len() + 36);
        lines.push(Line::from(vec![
            Span::styled("  │ ", Style::default().fg(border_color)),
            Span::styled("Blocks mined: ", Style::default().fg(Color::DarkGray)),
            Span::styled(blocks_str, Style::default().fg(Color::Cyan)),
            Span::styled("     Est hashrate: ", Style::default().fg(Color::DarkGray)),
            Span::styled(hash_str, Style::default().fg(Color::Green)),
            Span::styled(format!("{:>width$}  │", "", width = b_pad), Style::default().fg(border_color)),
        ]));

        let rev_str = format!("{:.3} BTC", miner.total_revenue_btc);
        let avg_str = format!("{:.1} min", miner.avg_interval_mins);
        let r_pad = w.saturating_sub(rev_str.len() + avg_str.len() + 36);
        lines.push(Line::from(vec![
            Span::styled("  │ ", Style::default().fg(border_color)),
            Span::styled("Total revenue: ", Style::default().fg(Color::DarkGray)),
            Span::styled(rev_str, Style::default().fg(Color::Yellow)),
            Span::styled("  Avg interval: ", Style::default().fg(Color::DarkGray)),
            Span::styled(avg_str, Style::default().fg(Color::White)),
            Span::styled(format!("{:>width$}  │", "", width = r_pad), Style::default().fg(border_color)),
        ]));

        if let (Some(gap), Some(btc_lost)) = (miner.gap_hours, miner.est_btc_lost) {
            let gap_str = format!("{:.1} hours", gap);
            let lost_str = format!("{:.5} BTC", btc_lost);
            let g_pad = w.saturating_sub(gap_str.len() + lost_str.len() + 36);
            lines.push(Line::from(vec![
                Span::styled("  │ ", Style::default().fg(border_color)),
                Span::styled("Gap duration: ", Style::default().fg(Color::DarkGray)),
                Span::styled(gap_str, Style::default().fg(Color::Yellow)),
                Span::styled("  Est BTC lost: ", Style::default().fg(Color::DarkGray)),
                Span::styled(lost_str, Style::default().fg(Color::Yellow)),
                Span::styled(format!("{:>width$}  │", "", width = g_pad), Style::default().fg(border_color)),
            ]));
        }

        lines.push(Line::from(Span::styled(
            format!("  └{:─<width$}┘", "", width = w.saturating_sub(4)),
            Style::default().fg(border_color),
        )));
        lines.push(Line::from(""));
    }

    let total = lines.len() as u16;
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((app.detail_scroll, 0)),
        inner,
    );

    if total > inner.height {
        let mut state = ratatui::widgets::ScrollbarState::new(total as usize)
            .position(app.detail_scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut state,
        );
    }
}
