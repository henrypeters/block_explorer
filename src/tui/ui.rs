use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::tui::app::{App, Mode, SearchResult};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // search box
            Constraint::Min(0),    // block list OR results
            Constraint::Length(1), // status bar
        ])
        .split(area);

    render_header(frame, chunks[0]);
    render_search(frame, app, chunks[1]);

    if app.result.is_some() {
        render_results(frame, app, chunks[2]);
    } else {
        render_block_list(frame, app, chunks[2]);
    }

    render_status(frame, app, chunks[3]);
}

// ─── Header ──────────────────────────────────────────────────────────────────

fn render_header(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new("  ₿  Bitcoin Block Explorer  —  regtest")
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .alignment(Alignment::Left);
    frame.render_widget(title, area);
}

// ─── Search box ──────────────────────────────────────────────────────────────

fn render_search(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.mode == Mode::Searching {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let hint = if app.mode == Mode::Searching {
        " [Enter] Search  [Esc] Cancel "
    } else {
        " Click here or press / to search "
    };

    let input = Paragraph::new(app.search_input.as_str())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(format!(" 🔍 Search: block height / hash / txid / address  {hint}")),
        );
    frame.render_widget(input, area);

    if app.mode == Mode::Searching {
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

    let mut lines: Vec<Line> = vec![];

    for block in &app.blocks {
        let top_border = format!("  ╔{:═<68}╗", "");
        let bot_border = format!("  ╚{:═<68}╝", "");

        lines.push(Line::from(
            Span::styled(top_border, Style::default().fg(Color::DarkGray))
        ));
        lines.push(Line::from(vec![
            Span::styled("  ║  ".to_string(), Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("BLOCK #{}", block.height),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "  —  {} tx  —  {} bytes{:>width$}  ║",
                    block.tx_count,
                    block.size,
                    "",
                    width = 64usize.saturating_sub(
                        format!("BLOCK #{}  —  {} tx  —  {} bytes", block.height, block.tx_count, block.size).len()
                    )
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ║  ".to_string(), Style::default().fg(Color::DarkGray)),
            Span::styled("Hash:  ".to_string(), Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:<58}", truncate(&block.hash, 58)),
                Style::default().fg(Color::White),
            ),
            Span::styled("  ║".to_string(), Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ║  ".to_string(), Style::default().fg(Color::DarkGray)),
            Span::styled("Time:  ".to_string(), Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:<58}", block.timestamp),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled("  ║".to_string(), Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(
            Span::styled(bot_border, Style::default().fg(Color::DarkGray))
        ));
        // Gap between cards
        lines.push(Line::from(""));
    }

    let total_lines = lines.len() as u16;
    let inner_height = area.height.saturating_sub(2);

    let para = Paragraph::new(Text::from(lines))
        .block(outer)
        .scroll((app.block_scroll, 0));
    frame.render_widget(para, area);

    if total_lines > inner_height {
        let mut state = ScrollbarState::new(total_lines as usize)
            .position(app.block_scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut state,
        );
    }
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
            let text = Paragraph::new(format!("\n  Not found: {msg}"))
                .style(Style::default().fg(Color::Red))
                .block(block_widget);
            frame.render_widget(text, area);
        }

        Some(SearchResult::Error(msg)) => {
            let text = Paragraph::new(format!("\n  Error: {msg}"))
                .style(Style::default().fg(Color::Red))
                .block(block_widget);
            frame.render_widget(text, area);
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

// ─── Status bar ──────────────────────────────────────────────────────────────

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let keys = if app.mode == Mode::Searching {
        ""
    } else if app.result.is_some() {
        "  [↑↓] Scroll   [Esc] Back to blocks   [q] Quit"
    } else {
        "  [↑↓] Scroll   [q] Quit"
    };

    let line = Line::from(vec![
        Span::styled(&app.status, Style::default().fg(Color::DarkGray)),
        Span::styled(keys, Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)),
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
