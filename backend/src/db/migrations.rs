use sqlx::{Row, SqlitePool};

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(include_str!("../../migrations/001_init.sql"))
        .execute(pool)
        .await?;

    sqlx::raw_sql(include_str!("../../migrations/002_folders.sql"))
        .execute(pool)
        .await?;

    if !column_exists(pool, "documents", "folder_id").await? {
        sqlx::query("ALTER TABLE documents ADD COLUMN folder_id TEXT REFERENCES folders(id)")
            .execute(pool)
            .await?;
    }

    sqlx::raw_sql(include_str!("../../migrations/003_document_folders.sql"))
        .execute(pool)
        .await?;

    sqlx::raw_sql(include_str!("../../migrations/004_inbox_to_root.sql"))
        .execute(pool)
        .await?;

    Ok(())
}

async fn column_exists(
    pool: &SqlitePool,
    table_name: &str,
    column_name: &str,
) -> Result<bool, sqlx::Error> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let rows = sqlx::query(&pragma).fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .any(|row| row.get::<String, _>("name") == column_name))
}
