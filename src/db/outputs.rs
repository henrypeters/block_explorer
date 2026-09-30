use sqlx::PgExecutor;

/// Marks a transaction output as spent.
/// Called when we encounter an input that references this output.
pub async fn mark_output_spent<'e>(
    executor: impl PgExecutor<'e>,
    prev_txid: &str,
    prev_vout: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE transaction_outputs SET is_spent = TRUE
         WHERE txid = $1 AND output_index = $2",
        prev_txid,
        prev_vout
    )
    .execute(executor)
    .await?;

    Ok(())
}
