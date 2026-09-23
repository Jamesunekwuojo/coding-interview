pub mod auth;
pub mod dataroom;
pub mod db;
pub mod error;
pub mod handlers;
pub mod plugins;
pub mod types;
use axum::{Extension, Router, extract::DefaultBodyLimit};
use sqlx::PgPool;
pub fn app(pool: PgPool) -> Router {
    let router = ts_server_fn_axum::api_router()
        .fallback(|| async { error::ApiError::not_found() })
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(Extension(pool.clone()));
    auth::session_layer(router, pool)
}
