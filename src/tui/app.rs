/// What the user is currently doing
#[derive(Debug, PartialEq)]
pub enum Mode {
    /// Normal mode — keyboard shortcuts active
    Normal,
    /// Typing in the search box
    Searching,
}

/// The result of a search query
#[derive(Debug, Clone)]
pub enum SearchResult {
    Block(BlockResult),
    Transaction(TxResult),
    Address(AddressResult),
    NotFound(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct BlockResult {
    pub height: i32,
    pub hash: String,
    pub prev_hash: String,
    pub timestamp: i64,
    pub size: i32,
    pub tx_count: i32,
    pub transactions: Vec<TxResult>,
}

#[derive(Debug, Clone)]
pub struct TxResult {
    pub txid: String,
    pub block_height: i32,
    pub is_coinbase: bool,
    pub inputs: Vec<InputResult>,
    pub outputs: Vec<OutputResult>,
}

#[derive(Debug, Clone)]
pub struct InputResult {
    pub input_index: i32,
    pub prev_txid: Option<String>,
    pub prev_vout: Option<i32>,
    pub script_sig: String,
}

#[derive(Debug, Clone)]
pub struct OutputResult {
    pub output_index: i32,
    pub value_sats: i64,
    pub address: Option<String>,
    pub script_type: Option<String>,
    pub is_spent: bool,
}

#[derive(Debug, Clone)]
pub struct AddressResult {
    pub address: String,
    pub balance_sats: i64,
    pub total_received_sats: i64,
    pub tx_count: i64,
    pub utxos: Vec<OutputResult>,
}

/// A single row in the block list panel
#[derive(Debug, Clone)]
pub struct BlockRow {
    pub height: i32,
    pub hash: String,
    pub timestamp: i64,
    pub tx_count: i32,
    pub size: i32,
}

/// The full application state
pub struct App {
    /// Current interaction mode
    pub mode: Mode,

    /// What the user has typed in the search box
    pub search_input: String,

    /// The result of the last search (shown in results panel)
    pub result: Option<SearchResult>,

    /// Live block list shown underneath the search box (latest first)
    pub blocks: Vec<BlockRow>,

    /// Scroll offset for the results panel
    pub scroll: u16,

    /// Scroll offset for the block list panel
    pub block_scroll: u16,

    /// Status message shown at the bottom
    pub status: String,

    /// Whether the app should quit
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            search_input: String::new(),
            result: None,
            blocks: Vec::new(),
            scroll: 0,
            block_scroll: 0,
            status: "Click the search box or press / to search.".to_string(),
            should_quit: false,
        }
    }

    pub fn enter_search_mode(&mut self) {
        self.mode = Mode::Searching;
        self.status = "Type your query and press Enter. Press Esc to cancel.".to_string();
    }

    pub fn exit_search_mode(&mut self) {
        self.mode = Mode::Normal;
        self.status = "Press / to search. Press q to quit.".to_string();
    }

    pub fn clear(&mut self) {
        self.search_input.clear();
        self.result = None;
        self.scroll = 0;
        self.status = "Click the search box or press / to search.".to_string();
    }

    pub fn scroll_up(&mut self) {
        if self.result.is_some() {
            self.scroll = self.scroll.saturating_sub(1);
        } else {
            self.block_scroll = self.block_scroll.saturating_sub(1);
        }
    }

    pub fn scroll_down(&mut self) {
        if self.result.is_some() {
            self.scroll = self.scroll.saturating_add(1);
        } else {
            self.block_scroll = self.block_scroll.saturating_add(1);
        }
    }
}
