
pub mod models;
pub mod types;

use crate::{
    auth::AuthenticatedUser,
    error::ApiError,
    types::{DataroomRpcRequest, RpcResponse, UserRole},
};
use sqlx::PgPool;

pub async fn dispatch(
    pool: &PgPool,
    user: &AuthenticatedUser,
    request: DataroomRpcRequest,
) -> Result<RpcResponse, ApiError> {
    // Workspace is the DataRoom security boundary.
    if request.workspace_id != user.workspace_id {
        return Err(ApiError::forbidden());
    }

    match request.method.as_str() {
        "list_materials" => {
            list_materials(pool, user, request.params).await
        }

        "get_material" => {
            get_material(pool, user, request.params).await
        }

        "register_material" => {
           match user.role {
            UserRole::Company => register_material(pool, user, request.params).await,
            _ => Err(ApiError::forbidden()),
           }
        }

        _ => Err(ApiError::not_implemented("dataroom::dispatch")),
    }
}



async fn list_materials(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: serde_json::Value,
) -> Result<RpcResponse, ApiError> {
    let params: types::ListMaterialsParams =
        serde_json::from_value(params).map_err(|_| ApiError::invalid("Invalid list parameters."))?;

    let search = params
        .search
        .filter(|value| !value.trim().is_empty());

    let rows = if let Some(search) = search {
        let pattern = format!("%{}%", search.trim());

        sqlx::query_as::<_, (
            String,
            String,
            String,
            String,
            String,
        )>(
            "SELECT id, title, file_name, status, created_at::text
             FROM materials
             WHERE workspace_id = $1
               AND title ILIKE $2
             ORDER BY created_at DESC, id ASC",
        )
        .bind(&user.workspace_id)
        .bind(pattern)
        .fetch_all(pool)
        .await
        .map_err(ApiError::storage)?
    } else {
        sqlx::query_as::<_, (
            String,
            String,
            String,
            String,
            String,
        )>(
            "SELECT id, title, file_name, status, created_at::text
             FROM materials
             WHERE workspace_id = $1
             ORDER BY created_at DESC, id ASC",
        )
        .bind(&user.workspace_id)
        .fetch_all(pool)
        .await
        .map_err(ApiError::storage)?
    };

    let materials = rows
        .into_iter()
        .map(
            |(id, title, file_name, status, created_at)| {
                let status = match status.as_str() {
                    "ready" => types::MaterialStatus::Ready,
                    "processing" => types::MaterialStatus::Processing,
                    "failed" => types::MaterialStatus::Failed,
                    _ => {
                        return Err(ApiError::storage(
                            "Invalid material status in database.",
                        ));
                    }
                };

                Ok(types::MaterialSummary {
                    id,
                    title,
                    file_name,
                    status,
                    created_at,
                })
            },
        )
        .collect::<Result<Vec<_>, ApiError>>()?;

    let result = types::ListMaterialsResponse { materials };

    Ok(RpcResponse {
        result: serde_json::to_value(result)
            .map_err(|_| ApiError::storage("Failed to serialize materials."))?,
    })
}

async fn get_material(
    _pool: &PgPool,
    _user: &AuthenticatedUser,
    _params: serde_json::Value,
) -> Result<RpcResponse, ApiError> {
    Err(ApiError::not_implemented("dataroom::get_material"))
}

async fn register_material(
    _pool: &PgPool,
    _user: &AuthenticatedUser,
    _params: serde_json::Value,
) -> Result<RpcResponse, ApiError> {
    Err(ApiError::not_implemented("dataroom::register_material"))
}




#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    fn test_user(role: UserRole, workspace_id: &str) -> AuthenticatedUser {
        AuthenticatedUser {
            id: "test-user".to_string(),
            name: "Test User".to_string(),
            role,
            workspace_id: workspace_id.to_string(),
            workspace_name: "Test Workspace".to_string(),
        }
    }

    #[tokio::test]
    async fn rejects_cross_workspace_requests_before_dispatch() {
        let pool = PgPool::connect_lazy("postgres://invalid/unused").unwrap();

        let user = test_user(UserRole::Company, "workspace-a");

        let request = DataroomRpcRequest {
            workspace_id: "workspace-b".to_string(),
            method: "list_materials".to_string(),
            params: serde_json::json!({
                "search": "anything"
            }),
        };

        let result = dispatch(&pool, &user, request).await;

        let error = result.expect_err("cross-workspace request should be rejected");

        assert_eq!(error.0, StatusCode::FORBIDDEN);
        assert_eq!(error.1, "forbidden");
    }

    #[tokio::test]
    async fn rejects_investor_material_registration_before_input_validation() {
        let pool = PgPool::connect_lazy("postgres://invalid/unused").unwrap();

        let user = test_user(UserRole::Investor, "workspace-a");

        let request = DataroomRpcRequest {
            workspace_id: "workspace-a".to_string(),
            method: "register_material".to_string(),
            params: serde_json::json!({
                "title": "",
                "file_name": "invalid.pdf",
                "content": ""
            }),
        };

        let result = dispatch(&pool, &user, request).await;

        let error = result.expect_err("investor registration should be rejected");

        assert_eq!(error.0, StatusCode::FORBIDDEN);
        assert_eq!(error.1, "forbidden");
    }
}