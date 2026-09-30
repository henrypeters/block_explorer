use sqlx::PgPool;

/// Reads the last indexed height from the database.
/// Returns -1 if nothing has been indexed yet.
pub async fn get_last_indexed_height(pool: &PgPool) -> Result<i32, sqlx::Error> {
    let row = sqlx::query!("SELECT last_indexed_height FROM indexer_state WHERE id = 1")
        .fetch_one(pool)
        .await?;

    Ok(row.last_indexed_height)
}

/// Updates the last indexed height after a block has been successfully committed.
pub async fn set_last_indexed_height(
    pool: &PgPool,
    height: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE indexer_state SET last_indexed_height = $1, updated_at = NOW() WHERE id = 1",
        height
    )
    .execute(pool)
    .await?;

    Ok(())
}
