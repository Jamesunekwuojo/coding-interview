use crate::{
    auth::CurrentUser,
    dataroom,
    error::ApiError,
    types::{DataroomRpcRequest, RpcResponse},
};
use axum::{Extension, Json};
use sqlx::PgPool;
use ts_server_fn::post;

#[post("/api/dataroom/rpc")]
pub async fn dataroom_rpc_handler(
    user: CurrentUser,
    Extension(pool): Extension<PgPool>,
    Json(request): Json<DataroomRpcRequest>,
) -> Result<RpcResponse, ApiError> {
    dataroom::dispatch(&pool, &user.0, request).await
}
