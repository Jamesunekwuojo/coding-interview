use crate::{error::ApiError, types::HealthResponse};
use axum::Extension;
use sqlx::PgPool;
use ts_server_fn::get;

#[get("/api/health")]
pub async fn health_handler(
    Extension(pool): Extension<PgPool>,
) -> Result<HealthResponse, ApiError> {
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(ApiError::storage)?;
    Ok(HealthResponse {
        status: "ok".into(),
    })
}
