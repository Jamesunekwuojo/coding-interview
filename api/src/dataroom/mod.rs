pub mod models;
pub mod types;
use crate::{
    auth::AuthenticatedUser,
    error::ApiError,
    types::{DataroomRpcRequest, RpcResponse},
};
use sqlx::PgPool;

pub async fn dispatch(
    _pool: &PgPool,
    _user: &AuthenticatedUser,
    _request: DataroomRpcRequest,
) -> Result<RpcResponse, ApiError> {
    Err(ApiError::not_implemented("dataroom::dispatch"))
}
