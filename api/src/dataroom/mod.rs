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

        sqlx::query_as::<_, models::MaterialSummaryRow>(
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
        sqlx::query_as::<_, models::MaterialSummaryRow>(
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
        .map(|row| {
            let status = match row.status.as_str() {
                "ready" => types::MaterialStatus::Ready,
                "processing" => types::MaterialStatus::Processing,
                "failed" => types::MaterialStatus::Failed,
                _ => {
                    return Err(ApiError::storage("Invalid material status in database."));
                }
            };

            Ok(types::MaterialSummary {
                id: row.id,
                title: row.title,
                file_name: row.file_name,
                status,
                created_at: row.created_at,
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

    let row = sqlx::query_as::<_, models::MaterialDetailRow>(
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

    let status = match row.status.as_str() {
        "ready" => types::MaterialStatus::Ready,
        "processing" => types::MaterialStatus::Processing,
        "failed" => types::MaterialStatus::Failed,
        _ => {
            return Err(ApiError::storage("Invalid material status in database."));
        }
    };

    let result = types::GetMaterialResponse {
        material: types::MaterialDetail {
            id: row.id,
            title: row.title,
            file_name: row.file_name,
            status,
            content: row.content,
            created_at: row.created_at,
            updated_at: row.updated_at,
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

    let row = sqlx::query_as::<_, models::MaterialDetailRow>(
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

    let status = match row.status.as_str() {
        "ready" => types::MaterialStatus::Ready,
        "processing" => types::MaterialStatus::Processing,
        "failed" => types::MaterialStatus::Failed,
        _ => {
            return Err(ApiError::storage("Invalid material status in database."));
        }
    };

    let result = types::RegisterMaterialResponse {
        material: types::MaterialDetail {
            id: row.id,
            title: row.title,
            file_name: row.file_name,
            status,
            content: row.content,
            created_at: row.created_at,
            updated_at: row.updated_at,
        },
    };

    Ok(RpcResponse {
        result: serde_json::to_value(result)
            .map_err(|_| ApiError::storage("Failed to serialize material."))?,
    })
}

#[cfg(test)]
mod tests;

