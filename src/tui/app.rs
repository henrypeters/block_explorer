/// What the user is currently doing
#[derive(Debug, PartialEq)]
pub enum Mode {
    /// Normal mode — keyboard shortcuts active
    Normal,
    /// Typing in the search box
    Searching,
    /// Selecting a field to copy
    Copying,
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

    /// List of copyable fields from the current result
    pub copy_fields: Vec<(String, String)>,

    /// Currently selected copy field index
    pub copy_selected: usize,

    /// Animation tick counter — incremented every frame
    pub tick: u64,

    /// Whether the background sync is still running
    pub syncing: bool,

    /// Whether a search query is currently running
    pub searching: bool,

    /// Height of the most recently arrived new block (for flash animation)
    pub new_block_flash: Option<(i32, u64)>, // (height, tick_when_arrived)

    /// Intro animation state — how many blocks to show (None = intro done)
    pub intro_reveal: Option<usize>,

    /// Instant when the last intro frame was shown
    pub intro_last_tick: std::time::Instant,

    /// Horizontal slide-in offset for the newest block (starts at -40 or +40, moves to 0)
    pub intro_anim_offset: i16,

    /// Whether the current slide is from the right (true) or left (false)
    pub intro_anim_from_right: bool,
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
            copy_fields: Vec::new(),
            copy_selected: 0,
            tick: 0,
            syncing: true,
            searching: false,
            new_block_flash: None,
            intro_reveal: Some(0),
            intro_last_tick: std::time::Instant::now(),
            intro_anim_offset: 0,
            intro_anim_from_right: true, // first block slides in from right
        }
    }

    /// Called every frame to advance animations
    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);

        // Advance slide-in offset toward 0 (4 chars per frame)
        if self.intro_anim_offset < 0 {
            self.intro_anim_offset = (self.intro_anim_offset + 4).min(0);
        } else if self.intro_anim_offset > 0 {
            self.intro_anim_offset = (self.intro_anim_offset - 4).max(0);
        }

        // Advance intro animation — reveal one more block every 0.5s
        if let Some(revealed) = self.intro_reveal {
            if self.intro_last_tick.elapsed() >= std::time::Duration::from_millis(500) {
                let total = self.blocks.len();
                if revealed >= total {
                    self.intro_reveal = None;
                    self.intro_anim_offset = 0;
                } else {
                    self.intro_reveal = Some(revealed + 1);
                    // Alternate direction: odd reveals from right, even from left
                    self.intro_anim_from_right = (revealed % 2) == 0;
                    self.intro_anim_offset = if self.intro_anim_from_right { 40 } else { -40 };
                }
                self.intro_last_tick = std::time::Instant::now();
            }
        }
    }

    /// Trigger a slide-in animation for a newly mined block.
    /// Alternates direction based on the block height (even=right, odd=left).
    pub fn trigger_new_block_slide(&mut self, height: i32) {
        self.intro_anim_from_right = (height % 2) == 0;
        self.intro_anim_offset = if self.intro_anim_from_right { 40 } else { -40 };
    }

    /// Returns the current spinner frame character
    pub fn spinner_frame(&self) -> &'static str {
        const FRAMES: &[&str] = &["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"];
        FRAMES[(self.tick / 3) as usize % FRAMES.len()]
    }

    /// Returns true if the new block flash is still active for a given height
    pub fn is_flashing(&self, height: i32) -> bool {
        if let Some((h, arrived_tick)) = self.new_block_flash {
            h == height && self.tick.saturating_sub(arrived_tick) < 20
        } else {
            false
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

    pub fn enter_copy_mode(&mut self) {
        // Build list of copyable fields from current result
        self.copy_fields = match &self.result {
            Some(SearchResult::Block(b)) => vec![
                ("Block Hash".to_string(),  b.hash.clone()),
                ("Prev Hash".to_string(),   b.prev_hash.clone()),
                ("Height".to_string(),      b.height.to_string()),
            ],
            Some(SearchResult::Transaction(t)) => vec![
                ("TXID".to_string(),        t.txid.clone()),
                ("Block Height".to_string(), t.block_height.to_string()),
            ],
            Some(SearchResult::Address(a)) => vec![
                ("Address".to_string(),     a.address.clone()),
                ("Balance".to_string(),     format!("{}", a.balance_sats)),
            ],
            _ => return,
        };

        if self.copy_fields.is_empty() {
            return;
        }

        self.copy_selected = 0;
        self.mode = Mode::Copying;
        self.status = "Select a field to copy. [↑↓] Navigate  [Enter] Copy  [Esc] Cancel".to_string();
    }

    pub fn exit_copy_mode(&mut self) {
        self.mode = Mode::Normal;
        self.copy_fields.clear();
        self.status = "Press / to search. [c] Copy a field. Press q to quit.".to_string();
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
            // Cap scroll so we can't go past the last block card
            // Each card is 6 lines (5 + 1 gap)
            let max_scroll = (self.blocks.len() as u16).saturating_sub(1) * 6;
            self.block_scroll = (self.block_scroll + 1).min(max_scroll);
        }
    }
}
