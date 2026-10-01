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
            Constraint::Min(0),    // block list OR results
            Constraint::Length(1), // status bar
        ])
        .split(area);

    render_header(frame, app, chunks[0]);
    render_search(frame, app, chunks[1]);
    // chunks[2] is the spacer — nothing rendered there
    if app.result.is_some() {
        render_results(frame, app, chunks[3]);
    } else {
        render_block_list(frame, app, chunks[3]);
    }

    render_status(frame, app, chunks[4]);

    // Copy mode overlay — rendered on top of everything
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

    let badge = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " REGTEST ",
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

    // Card width — fixed at 96 chars
    const CARD_WIDTH: u16 = 96;

    // Center the card container within the inner area
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    // Centered x position for cards
    let card_x = inner.x + inner.width.saturating_sub(CARD_WIDTH) / 2;

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
            .min((inner.x + inner.width) as i32) as u16;

        // Clip card height to not exceed the inner area bottom
        let max_y = inner.y + inner.height;
        let actual_y = scrolled_top as u16;
        let available_height = max_y.saturating_sub(actual_y).min(CARD_HEIGHT);

        if available_height == 0 {
            break;
        }

        let card_rect = Rect {
            x: slide_x,
            y: actual_y,
            width: CARD_WIDTH.min(inner.width),
            height: available_height,
        };

        render_block_card(frame, block, card_rect);
    }
}

/// Renders a single block card into the given rect.
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
        match &app.result {
            Some(SearchResult::NotFound(_)) | Some(SearchResult::Error(_)) => {
                "  [Esc] Back to blocks   [/] New search   [q] Quit"
            }
            Some(_) => "  [↑↓] Scroll   [c] Copy   [Esc] Back to blocks   [q] Quit",
            None => "  [↑↓] Scroll   [q] Quit",
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
