#[path = "../../../plugins/review/server/mod.rs"]
pub mod review;
use crate::{
    auth::AuthenticatedUser,
    error::ApiError,
    types::{PluginRegistration, PluginRpcRequest, RpcResponse},
};
use sqlx::PgPool;

pub fn catalog() -> Vec<PluginRegistration> {
    vec![PluginRegistration {
        id: review::ID.into(),
        name: "검토".into(),
        manifest_url: format!("/plugins/{}/manifest.json", review::ID),
    }]
}

pub async fn dispatch(
    pool: &PgPool,
    user: &AuthenticatedUser,
    request: PluginRpcRequest,
) -> Result<RpcResponse, ApiError> {
    match request.plugin_id.as_str() {
        review::ID => review::dispatch(pool, user, request).await,
        _ => Err(ApiError::not_found()),
    }
}
