use bitcoin::{Address, Block, Network};
use bitcoin::consensus::serialize;
use colored::Colorize;

// ─── Public entry point ──────────────────────────────────────────────────────

/// Prints a full formatted summary of a block including all transactions,
/// inputs, and outputs. Called by both the initial sync and the poller.
pub fn print_block(block: &Block, height: u64, network: Network) {
    let summary = build_summary(block, height, network);
    print_block_summary(&summary);
}

// ─── Internal structs ────────────────────────────────────────────────────────

pub struct BlockSummary {
    pub height: u64,
    pub hash: String,
    pub prev_hash: String,
    pub timestamp: u32,
    pub size: usize,
    pub transactions: Vec<TxSummary>,
}

pub struct TxSummary {
    pub txid: String,
    pub is_coinbase: bool,
    pub inputs: Vec<InputSummary>,
    pub outputs: Vec<OutputSummary>,
    pub total_output_sats: u64,
}

pub enum InputSummary {
    Coinbase { script_data: String },
    Spend { prev_txid: String, prev_vout: u32 },
}

pub struct OutputSummary {
    pub value_sats: u64,
    pub address: Option<String>,
    pub script_type: String,
}

// ─── Builder ─────────────────────────────────────────────────────────────────

pub fn build_summary(block: &Block, height: u64, network: Network) -> BlockSummary {
    let transactions = block
        .txdata
        .iter()
        .map(|tx| {
            let is_coinbase = tx.is_coinbase();

            let inputs = tx
                .input
                .iter()
                .map(|input| {
                    if is_coinbase {
                        InputSummary::Coinbase {
                            script_data: hex::encode(input.script_sig.as_bytes()),
                        }
                    } else {
                        InputSummary::Spend {
                            prev_txid: input.previous_output.txid.to_string(),
                            prev_vout: input.previous_output.vout,
                        }
                    }
                })
                .collect();

            let outputs = tx
                .output
                .iter()
                .map(|out| {
                    let address = Address::from_script(&out.script_pubkey, network)
                        .map(|a| a.to_string())
                        .ok();
                    OutputSummary {
                        value_sats: out.value.to_sat(),
                        address,
                        script_type: classify_script(&out.script_pubkey),
                    }
                })
                .collect::<Vec<_>>();

            let total_output_sats = outputs.iter().map(|o| o.value_sats).sum();

            TxSummary {
                txid: tx.compute_txid().to_string(),
                is_coinbase,
                inputs,
                outputs,
                total_output_sats,
            }
        })
        .collect();

    BlockSummary {
        height,
        hash: block.block_hash().to_string(),
        prev_hash: block.header.prev_blockhash.to_string(),
        timestamp: block.header.time,
        size: serialize(block).len(),
        transactions,
    }
}

// ─── Printing ─────────────────────────────────────────────────────────────────

pub fn print_block_summary(b: &BlockSummary) {
    let divider = "═".repeat(72);
    let thin = "─".repeat(72);

    println!("\n{}", divider.cyan().dimmed());
    println!(
        "  {} {}   {}   {} bytes",
        "BLOCK".bold().cyan(),
        format!("#{}", b.height).bold().yellow(),
        format_timestamp(b.timestamp).dimmed(),
        b.size.to_string().dimmed()
    );
    println!("{}", divider.cyan().dimmed());
    println!("  {:<14} {}", "Hash:".dimmed(),     b.hash.bright_white());
    println!("  {:<14} {}", "Prev Hash:".dimmed(), b.prev_hash.bright_white());
    println!("  {:<14} {}", "Tx Count:".dimmed(),  b.transactions.len().to_string().bright_white());
    println!("{}", thin.dimmed());

    for (i, tx) in b.transactions.iter().enumerate() {
        print_tx_summary(i, tx);
    }
}

fn print_tx_summary(i: usize, tx: &TxSummary) {
    let label = if tx.is_coinbase {
        "COINBASE".green().bold()
    } else {
        "TX      ".bright_blue().bold()
    };

    println!(
        "\n  {} #{}  {}",
        label,
        i.to_string().dimmed(),
        truncate(&tx.txid, 44).bright_white()
    );
    println!(
        "           Total out: {}",
        format_sats(tx.total_output_sats).yellow()
    );

    println!("           {}", "INPUTS".dimmed());
    for (j, input) in tx.inputs.iter().enumerate() {
        match input {
            InputSummary::Coinbase { script_data } => {
                println!(
                    "             #{j}  {}  data: {}",
                    "coinbase".green(),
                    truncate(script_data, 40).dimmed()
                );
            }
            InputSummary::Spend { prev_txid, prev_vout } => {
                println!(
                    "             #{j}  {}:{}",
                    truncate(prev_txid, 44).bright_white(),
                    prev_vout.to_string().yellow()
                );
            }
        }
    }

    println!("           {}", "OUTPUTS".dimmed());
    for (k, out) in tx.outputs.iter().enumerate() {
        let addr = out.address.as_deref().unwrap_or("no address");
        println!(
            "             #{k}  {}  →  {} ({})",
            format_sats(out.value_sats).yellow(),
            addr.bright_white(),
            out.script_type.dimmed()
        );
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn classify_script(script: &bitcoin::Script) -> String {
    if script.is_p2pkh()          { "p2pkh".into() }
    else if script.is_p2sh()      { "p2sh".into() }
    else if script.is_p2wpkh()    { "p2wpkh".into() }
    else if script.is_p2wsh()     { "p2wsh".into() }
    else if script.is_p2tr()      { "p2tr".into() }
    else if script.is_op_return() { "op_return".into() }
    else                          { "unknown".into() }
}

pub fn format_timestamp(ts: u32) -> String {
    format!("{ts} (unix)")
}

pub fn format_sats(sats: u64) -> String {
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
