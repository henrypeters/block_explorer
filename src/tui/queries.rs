use sqlx::PgPool;

use crate::tui::app::{
    AddressResult, BlockResult, BlockRow, InputResult, OutputResult, SearchResult, TxResult,
};

/// Loads the most recent blocks from the database (latest first).
pub async fn load_recent_blocks(pool: &PgPool) -> Vec<BlockRow> {
    sqlx::query!(
        "SELECT height, hash, timestamp, tx_count, size
         FROM blocks ORDER BY height DESC LIMIT 100"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| BlockRow {
        height: r.height,
        hash: r.hash,
        timestamp: r.timestamp,
        tx_count: r.tx_count,
        size: r.size,
    })
    .collect()
}

/// Detects what the user typed and runs the appropriate query.
pub async fn search(pool: &PgPool, query: &str) -> SearchResult {
    let query = query.trim();

    if query.is_empty() {
        return SearchResult::NotFound("Empty query.".to_string());
    }

    // Pure digits → block height
    if query.chars().all(|c| c.is_ascii_digit()) {
        let height: i32 = query.parse().unwrap_or(-1);
        return search_block_by_height(pool, height).await;
    }

    // 64 hex chars → block hash or txid
    if query.len() == 64 && query.chars().all(|c| c.is_ascii_hexdigit()) {
        // Try block hash first, then txid
        let block = search_block_by_hash(pool, query).await;
        if !matches!(block, SearchResult::NotFound(_)) {
            return block;
        }
        return search_tx(pool, query).await;
    }

    // Starts with bc1, bcrt1, 1, 3 → address
    if query.starts_with("bcrt1")
        || query.starts_with("bc1")
        || query.starts_with('1')
        || query.starts_with('3')
    {
        return search_address(pool, query).await;
    }

    SearchResult::NotFound(format!("Could not identify query type: {query}"))
}

async fn search_block_by_height(pool: &PgPool, height: i32) -> SearchResult {
    let row = sqlx::query!(
        "SELECT height, hash, prev_hash, timestamp, size, tx_count
         FROM blocks WHERE height = $1",
        height
    )
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(b)) => {
            let txs = fetch_transactions_for_block(pool, b.height).await;
            SearchResult::Block(BlockResult {
                height: b.height,
                hash: b.hash,
                prev_hash: b.prev_hash,
                timestamp: b.timestamp,
                size: b.size,
                tx_count: b.tx_count,
                transactions: txs,
            })
        }
        Ok(None) => SearchResult::NotFound(format!("No block at height {height}")),
        Err(e) => SearchResult::Error(e.to_string()),
    }
}

async fn search_block_by_hash(pool: &PgPool, hash: &str) -> SearchResult {
    let row = sqlx::query!(
        "SELECT height, hash, prev_hash, timestamp, size, tx_count
         FROM blocks WHERE hash = $1",
        hash
    )
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(b)) => {
            let txs = fetch_transactions_for_block(pool, b.height).await;
            SearchResult::Block(BlockResult {
                height: b.height,
                hash: b.hash,
                prev_hash: b.prev_hash,
                timestamp: b.timestamp,
                size: b.size,
                tx_count: b.tx_count,
                transactions: txs,
            })
        }
        Ok(None) => SearchResult::NotFound(format!("No block with hash {hash}")),
        Err(e) => SearchResult::Error(e.to_string()),
    }
}

async fn search_tx(pool: &PgPool, txid: &str) -> SearchResult {
    let row = sqlx::query!(
        "SELECT txid, block_height, is_coinbase FROM transactions WHERE txid = $1",
        txid
    )
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(t)) => {
            let inputs = fetch_inputs(pool, &t.txid).await;
            let outputs = fetch_outputs(pool, &t.txid).await;
            SearchResult::Transaction(TxResult {
                txid: t.txid,
                block_height: t.block_height,
                is_coinbase: t.is_coinbase,
                inputs,
                outputs,
            })
        }
        Ok(None) => SearchResult::NotFound(format!("No transaction with txid {txid}")),
        Err(e) => SearchResult::Error(e.to_string()),
    }
}

async fn search_address(pool: &PgPool, address: &str) -> SearchResult {
    // Get all unspent outputs for this address
    let utxos = sqlx::query!(
        "SELECT output_index, value_sats, address, script_type, is_spent, txid
         FROM transaction_outputs
         WHERE address = $1 AND is_spent = FALSE",
        address
    )
    .fetch_all(pool)
    .await;

    match utxos {
        Ok(rows) => {
            let balance_sats: i64 = rows.iter().map(|r| r.value_sats).sum();

            // Total ever received — cast to BIGINT to avoid NUMERIC type issues
            let total: i64 = sqlx::query_scalar!(
                "SELECT COALESCE(SUM(value_sats), 0)::BIGINT
                 FROM transaction_outputs WHERE address = $1",
                address
            )
            .fetch_one(pool)
            .await
            .unwrap_or(Some(0))
            .unwrap_or(0);

            // Distinct tx count
            let tx_count: i64 = sqlx::query_scalar!(
                "SELECT COUNT(DISTINCT txid)
                 FROM transaction_outputs WHERE address = $1",
                address
            )
            .fetch_one(pool)
            .await
            .unwrap_or(Some(0))
            .unwrap_or(0);

            if rows.is_empty() && total == 0 {
                return SearchResult::NotFound(format!("Address {address} not found"));
            }

            let utxo_list = rows
                .iter()
                .map(|r| OutputResult {
                    output_index: r.output_index,
                    value_sats: r.value_sats,
                    address: r.address.clone(),
                    script_type: r.script_type.clone(),
                    is_spent: r.is_spent,
                })
                .collect();

            SearchResult::Address(AddressResult {
                address: address.to_string(),
                balance_sats,
                total_received_sats: total,
                tx_count,
                utxos: utxo_list,
            })
        }
        Err(e) => SearchResult::Error(e.to_string()),
    }
}

async fn fetch_transactions_for_block(pool: &PgPool, height: i32) -> Vec<TxResult> {
    let rows = sqlx::query!(
        "SELECT txid, block_height, is_coinbase FROM transactions
         WHERE block_height = $1 ORDER BY tx_index ASC",
        height
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut txs = vec![];
    for row in rows {
        let inputs = fetch_inputs(pool, &row.txid).await;
        let outputs = fetch_outputs(pool, &row.txid).await;
        txs.push(TxResult {
            txid: row.txid,
            block_height: row.block_height,
            is_coinbase: row.is_coinbase,
            inputs,
            outputs,
        });
    }
    txs
}

async fn fetch_inputs(pool: &PgPool, txid: &str) -> Vec<InputResult> {
    sqlx::query!(
        "SELECT input_index, prev_txid, prev_vout, script_sig
         FROM transaction_inputs WHERE txid = $1 ORDER BY input_index ASC",
        txid
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| InputResult {
        input_index: r.input_index,
        prev_txid: r.prev_txid,
        prev_vout: r.prev_vout,
        script_sig: r.script_sig,
    })
    .collect()
}

async fn fetch_outputs(pool: &PgPool, txid: &str) -> Vec<OutputResult> {
    sqlx::query!(
        "SELECT output_index, value_sats, address, script_type, is_spent
         FROM transaction_outputs WHERE txid = $1 ORDER BY output_index ASC",
        txid
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| OutputResult {
        output_index: r.output_index,
        value_sats: r.value_sats,
        address: r.address,
        script_type: r.script_type,
        is_spent: r.is_spent,
    })
    .collect()
}
