use crate::{
    auth::CurrentUser,
    error::ApiError,
    plugins,
    types::{PluginRpcRequest, RpcResponse},
};
use axum::{Extension, Json};
use sqlx::PgPool;
use ts_server_fn::post;

#[post("/api/plugins/rpc")]
pub async fn plugin_rpc_handler(
    user: CurrentUser,
    Extension(pool): Extension<PgPool>,
    Json(request): Json<PluginRpcRequest>,
) -> Result<RpcResponse, ApiError> {
    plugins::dispatch(&pool, &user.0, request).await
}
