use bitcoin::{Address, Block, Network, Transaction};
use bitcoin::consensus::serialize;
use sqlx::PgPool;
use tracing::debug;

use crate::db::outputs::mark_output_spent;
use crate::indexer::pools::identify_pool;

/// Writes a full block and all its transactions, inputs, and outputs
/// to the database inside a single PostgreSQL transaction.
/// If anything fails, the entire block is rolled back.
pub async fn insert_block(
    pool: &PgPool,
    block: &Block,
    height: i32,
    network: Network,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let block_hash = block.block_hash().to_string();
    let prev_hash = block.header.prev_blockhash.to_string();
    let merkle_root = block.header.merkle_root.to_string();
    let block_size = serialize(block).len() as i32;
    let tx_count = block.txdata.len() as i32;

    // Extract pool name from coinbase script
    let pool_name = block.txdata.first().and_then(|coinbase_tx| {
        coinbase_tx.input.first().map(|input| {
            let hex = hex::encode(input.script_sig.as_bytes());
            identify_pool(&hex)
        })
    }).flatten();

    // --- Insert block ---
    sqlx::query!(
        "INSERT INTO blocks
            (height, hash, version, prev_hash, merkle_root, timestamp, bits, nonce, size, tx_count, pool_name)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         ON CONFLICT (height) DO NOTHING",
        height,
        block_hash,
        block.header.version.to_consensus(),
        prev_hash,
        merkle_root,
        block.header.time as i64,
        block.header.bits.to_consensus() as i64,
        block.header.nonce as i64,
        block_size,
        tx_count,
        pool_name,
    )
    .execute(&mut *tx)
    .await?;

    // --- Insert each transaction ---
    for (tx_index, bitcoin_tx) in block.txdata.iter().enumerate() {
        insert_transaction(&mut tx, bitcoin_tx, &block_hash, height, tx_index, network).await?;
    }

    tx.commit().await?;

    debug!("Committed block {} ({})", height, block_hash);
    Ok(())
}

/// Inserts a single transaction along with all its inputs and outputs.
async fn insert_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    bitcoin_tx: &Transaction,
    block_hash: &str,
    block_height: i32,
    tx_index: usize,
    network: Network,
) -> Result<(), sqlx::Error> {
    let txid = bitcoin_tx.compute_txid().to_string();
    let is_coinbase = bitcoin_tx.is_coinbase();
    let tx_size = serialize(bitcoin_tx).len() as i32;

    sqlx::query!(
        "INSERT INTO transactions
            (txid, block_height, block_hash, tx_index, version, locktime, is_coinbase, size)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (txid) DO NOTHING",
        txid,
        block_height,
        block_hash,
        tx_index as i32,
        bitcoin_tx.version.0,
        bitcoin_tx.lock_time.to_consensus_u32() as i64,
        is_coinbase,
        tx_size,
    )
    .execute(&mut **tx)
    .await?;

    // --- Insert inputs ---
    for (input_index, input) in bitcoin_tx.input.iter().enumerate() {
        let (prev_txid, prev_vout) = if is_coinbase {
            (None, None)
        } else {
            (
                Some(input.previous_output.txid.to_string()),
                Some(input.previous_output.vout as i32),
            )
        };

        // Encode witness items as hex strings
        let witness: Vec<String> = input
            .witness
            .iter()
            .map(|item| hex::encode(item))
            .collect();

        let witness_arr: Option<Vec<String>> = if witness.is_empty() {
            None
        } else {
            Some(witness)
        };

        sqlx::query!(
            "INSERT INTO transaction_inputs
                (txid, input_index, prev_txid, prev_vout, script_sig, sequence, witness)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (txid, input_index) DO NOTHING",
            txid,
            input_index as i32,
            prev_txid,
            prev_vout,
            hex::encode(input.script_sig.as_bytes()),
            input.sequence.0 as i64,
            witness_arr.as_deref(),
        )
        .execute(&mut **tx)
        .await?;

        // Mark the referenced output as spent
        if let (Some(prev_txid_str), Some(vout)) = (&prev_txid, prev_vout) {
            mark_output_spent(&mut **tx, prev_txid_str, vout).await?;
        }
    }

    // --- Insert outputs ---
    for (output_index, output) in bitcoin_tx.output.iter().enumerate() {
        let value_sats = output.value.to_sat() as i64;
        let script_hex = hex::encode(output.script_pubkey.as_bytes());

        // Determine script type and address
        let script_type = classify_script(&output.script_pubkey);
        let address = Address::from_script(&output.script_pubkey, network)
            .map(|a| a.to_string())
            .ok();

        sqlx::query!(
            "INSERT INTO transaction_outputs
                (txid, output_index, value_sats, script_pubkey, script_type, address, is_spent)
             VALUES ($1, $2, $3, $4, $5, $6, FALSE)
             ON CONFLICT (txid, output_index) DO NOTHING",
            txid,
            output_index as i32,
            value_sats,
            script_hex,
            script_type,
            address,
        )
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

/// Classifies a scriptPubKey into a human-readable script type string.
fn classify_script(script: &bitcoin::Script) -> Option<String> {
    if script.is_p2pkh() {
        Some("p2pkh".to_string())
    } else if script.is_p2sh() {
        Some("p2sh".to_string())
    } else if script.is_p2wpkh() {
        Some("p2wpkh".to_string())
    } else if script.is_p2wsh() {
        Some("p2wsh".to_string())
    } else if script.is_p2tr() {
        Some("p2tr".to_string())
    } else if script.is_op_return() {
        Some("op_return".to_string())
    } else {
        Some("unknown".to_string())
    }
}
