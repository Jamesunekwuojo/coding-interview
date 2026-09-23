pub mod models;
pub mod types;
use crate::{
    auth::AuthenticatedUser,
    error::ApiError,
    types::{PluginRpcRequest, RpcResponse},
};
use sqlx::PgPool;
use types::ReviewHealthResponse;

pub const ID: &str = "review";

pub async fn dispatch(
    _pool: &PgPool,
    user: &AuthenticatedUser,
    request: PluginRpcRequest,
) -> Result<RpcResponse, ApiError> {
    if request.workspace_id != user.workspace_id {
        return Err(ApiError::forbidden());
    }
    match request.method.as_str() {
        "health" => Ok(RpcResponse {
            result: serde_json::to_value(ReviewHealthResponse {
                status: "ok".into(),
                plugin_id: ID.into(),
            })
            .map_err(ApiError::storage)?,
        }),
        _ => Err(ApiError::not_implemented("review::dispatch")),
    }
}
