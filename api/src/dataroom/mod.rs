pub mod models;
pub mod types;

use crate::{
    auth::AuthenticatedUser,
    error::ApiError,
    types::{DataroomRpcRequest, RpcResponse, UserRole},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
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
        "list_materials" => list_materials(pool, user, request.params).await,

        "get_material" => get_material(pool, user, request.params).await,

        "register_material" => match user.role {
            UserRole::Company => register_material(pool, user, request.params).await,
            _ => Err(ApiError::forbidden()),
        },

        _ => Err(ApiError::not_implemented("dataroom::dispatch")),
    }
}

fn new_material_id() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);

    format!("mat_{}", URL_SAFE_NO_PAD.encode(bytes))
}

async fn list_materials(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: serde_json::Value,
) -> Result<RpcResponse, ApiError> {
    let params: types::ListMaterialsParams = serde_json::from_value(params)
        .map_err(|_| ApiError::invalid("Invalid list parameters."))?;

    let search = params.search.filter(|value| !value.trim().is_empty());

    let rows = if let Some(search) = search {
        let pattern = format!("%{}%", search.trim());

        sqlx::query_as::<_, (String, String, String, String, String)>(
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
        sqlx::query_as::<_, (String, String, String, String, String)>(
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
        .map(|(id, title, file_name, status, created_at)| {
            let status = match status.as_str() {
                "ready" => types::MaterialStatus::Ready,
                "processing" => types::MaterialStatus::Processing,
                "failed" => types::MaterialStatus::Failed,
                _ => {
                    return Err(ApiError::storage("Invalid material status in database."));
                }
            };

            Ok(types::MaterialSummary {
                id,
                title,
                file_name,
                status,
                created_at,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    let result = types::ListMaterialsResponse { materials };

    Ok(RpcResponse {
        result: serde_json::to_value(result)
            .map_err(|_| ApiError::storage("Failed to serialize materials."))?,
    })
}

// get material
async fn get_material(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: serde_json::Value,
) -> Result<RpcResponse, ApiError> {
    let params: types::GetMaterialParams = serde_json::from_value(params)
        .map_err(|_| ApiError::invalid("Invalid get material parameters."))?;

    let row = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
        "SELECT
            id,
            title,
            file_name,
            status,
            content,
            created_at::text,
            updated_at::text
         FROM materials
         WHERE id = $1
           AND workspace_id = $2",
    )
    .bind(&params.material_id)
    .bind(&user.workspace_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::storage)?
    .ok_or_else(ApiError::not_found)?;

    let (id, title, file_name, status, content, created_at, updated_at) = row;

    let status = match status.as_str() {
        "ready" => types::MaterialStatus::Ready,
        "processing" => types::MaterialStatus::Processing,
        "failed" => types::MaterialStatus::Failed,
        _ => {
            return Err(ApiError::storage("Invalid material status in database."));
        }
    };

    let result = types::GetMaterialResponse {
        material: types::MaterialDetail {
            id,
            title,
            file_name,
            status,
            content,
            created_at,
            updated_at,
        },
    };

    Ok(RpcResponse {
        result: serde_json::to_value(result)
            .map_err(|_| ApiError::storage("Failed to serialize material."))?,
    })
}

async fn register_material(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: serde_json::Value,
) -> Result<RpcResponse, ApiError> {
    let params: types::RegisterMaterialParams = serde_json::from_value(params)
        .map_err(|_| ApiError::invalid("Invalid register material parameters."))?;

    if params.title.trim().is_empty() {
        return Err(ApiError::invalid("Material title cannot be empty."));
    }

    if !params.file_name.to_ascii_lowercase().ends_with(".txt")
        && !params.file_name.to_ascii_lowercase().ends_with(".md")
    {
        return Err(ApiError::invalid(
            "Material file must be a .txt or .md file.",
        ));
    }

    let material_id = new_material_id();

    let row = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
        "INSERT INTO materials (
            id,
            workspace_id,
            uploader_id,
            title,
            file_name,
            content,
            status
         )
         VALUES ($1, $2, $3, $4, $5, $6, 'ready')
         RETURNING
            id,
            title,
            file_name,
            status,
            content,
            created_at::text,
            updated_at::text",
    )
    .bind(&material_id)
    .bind(&user.workspace_id)
    .bind(&user.id)
    .bind(params.title.trim())
    .bind(&params.file_name)
    .bind(&params.content)
    .fetch_one(pool)
    .await
    .map_err(ApiError::storage)?;

    let (id, title, file_name, status, content, created_at, updated_at) = row;

    let status = match status.as_str() {
        "ready" => types::MaterialStatus::Ready,
        "processing" => types::MaterialStatus::Processing,
        "failed" => types::MaterialStatus::Failed,
        _ => {
            return Err(ApiError::storage("Invalid material status in database."));
        }
    };

    let result = types::RegisterMaterialResponse {
        material: types::MaterialDetail {
            id,
            title,
            file_name,
            status,
            content,
            created_at,
            updated_at,
        },
    };

    Ok(RpcResponse {
        result: serde_json::to_value(result)
            .map_err(|_| ApiError::storage("Failed to serialize material."))?,
    })
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

    #[tokio::test]
    async fn gets_material_from_authenticated_workspace() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let user = test_user(UserRole::Company, "lighthouse");

        let params = serde_json::json!({
            "materialId": "mat-doc-business"
        });

        let result = get_material(&pool, &user, params)
            .await
            .expect("material should be returned");

        let response: types::GetMaterialResponse =
            serde_json::from_value(result.result).expect("invalid response");

        assert_eq!(response.material.id, "mat-doc-business");
        assert_eq!(response.material.title, "회사 소개");
        assert_eq!(response.material.file_name, "company-overview.md");
        assert!(matches!(
            response.material.status,
            types::MaterialStatus::Ready
        ));
        assert!(!response.material.content.is_empty());
    }

    #[tokio::test]
    async fn returns_not_found_for_missing_material() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let user = test_user(UserRole::Company, "lighthouse");

        let params = serde_json::json!({
            "materialId": "mat-does-not-exist"
        });

        let error = get_material(&pool, &user, params)
            .await
            .expect_err("missing material should return an error");

        assert_eq!(error.0, StatusCode::NOT_FOUND);
        assert_eq!(error.1, "not_found");
    }

    #[tokio::test]
    async fn does_not_return_material_from_another_workspace() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let user = test_user(UserRole::Company, "another-workspace");

        let params = serde_json::json!({
            "materialId": "mat-doc-business"
        });

        let error = get_material(&pool, &user, params)
            .await
            .expect_err("cross-workspace material should not be returned");

        assert_eq!(error.0, StatusCode::NOT_FOUND);
        assert_eq!(error.1, "not_found");
    }

    #[tokio::test]
    async fn company_can_register_material() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let user = AuthenticatedUser {
            id: "company-user".to_string(),
            name: "Test Company User".to_string(),
            role: UserRole::Company,
            workspace_id: "lighthouse".to_string(),
            workspace_name: "Lighthouse".to_string(),
        };

        let params = serde_json::json!({
            "title": "Test Material",
            "fileName": "test-material.md",
            "content": "# Test Material\n\nThis is test content."
        });

        let result = register_material(&pool, &user, params)
            .await
            .expect("company should be able to register a material");

        let response: types::RegisterMaterialResponse =
            serde_json::from_value(result.result).expect("invalid response");

        assert!(response.material.id.starts_with("mat_"));
        assert_eq!(response.material.title, "Test Material");
        assert_eq!(response.material.file_name, "test-material.md");
        assert_eq!(
            response.material.content,
            "# Test Material\n\nThis is test content."
        );
        assert!(matches!(
            response.material.status,
            types::MaterialStatus::Ready
        ));

        let (workspace_id, uploader_id, status): (String, String, String) = sqlx::query_as(
            "SELECT workspace_id, uploader_id, status
             FROM materials
             WHERE id = $1",
        )
        .bind(&response.material.id)
        .fetch_one(&pool)
        .await
        .expect("registered material should exist");

        assert_eq!(workspace_id, "lighthouse");
        assert_eq!(uploader_id, "company-user");
        assert_eq!(status, "ready");

        sqlx::query("DELETE FROM materials WHERE id = $1")
            .bind(&response.material.id)
            .execute(&pool)
            .await
            .expect("failed to clean up test material");
    }

    #[tokio::test]
    async fn rejects_empty_material_title() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let user = test_user(UserRole::Company, "lighthouse");

        let params = serde_json::json!({
            "title": "   ",
            "fileName": "test.md",
            "content": "Some content"
        });

        let error = register_material(&pool, &user, params)
            .await
            .expect_err("whitespace-only title should be rejected");

        assert_eq!(error.0, StatusCode::BAD_REQUEST);
        assert_eq!(error.1, "invalid_input");
    }

    #[tokio::test]
    async fn rejects_unsupported_material_file_type() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let user = test_user(UserRole::Company, "lighthouse");

        let params = serde_json::json!({
            "title": "Invalid File",
            "fileName": "document.pdf",
            "content": "Some content"
        });

        let error = register_material(&pool, &user, params)
            .await
            .expect_err("unsupported file type should be rejected");

        assert_eq!(error.0, StatusCode::BAD_REQUEST);
        assert_eq!(error.1, "invalid_input");
    }
}
