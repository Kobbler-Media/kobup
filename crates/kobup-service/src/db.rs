use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};

pub type DbPool = Pool<Sqlite>;

pub async fn connect(database_url: &str) -> Result<DbPool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    // Enable WAL mode for better concurrent read/write
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;

    Ok(pool)
}

pub async fn migrate(pool: &DbPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!().run(pool).await
}
