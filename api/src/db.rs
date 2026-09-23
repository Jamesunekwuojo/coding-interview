use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::error::ApiError;

pub static MIGRATIONS: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn connect(url: &str) -> Result<PgPool, ApiError> {
    PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(5))
        .connect(url)
        .await
        .map_err(ApiError::storage)
}

pub async fn initialize(pool: &PgPool) -> Result<(), ApiError> {
    MIGRATIONS.run(pool).await.map_err(ApiError::storage)
}
