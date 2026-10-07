pub mod models;
pub mod types;

use crate::{
    auth::AuthenticatedUser,
    error::ApiError,
    types::{PluginRpcRequest, RpcResponse, UserRole},
};
use sqlx::PgPool;
use types::ReviewHealthResponse;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;

pub const ID: &str = "review";

pub async fn dispatch(
    pool: &PgPool,
    user: &AuthenticatedUser,
    request: PluginRpcRequest,
) -> Result<RpcResponse, ApiError> {
    if request.workspace_id != user.workspace_id {
        return Err(ApiError::forbidden());
    }

    match request.method.as_str() {
        "health" => health(),
        "list_criteria" => list_criteria(pool, user).await,

        "get_summary" => match user.role {
            UserRole::Investor => get_summary(pool, user).await,
            _ => Err(ApiError::forbidden()),
        },

        "list_reviews" => match user.role {
            UserRole::Investor => list_reviews(pool, user).await,
            _ => Ok(RpcResponse {
                result: serde_json::to_value(types::ListReviewsResponse {
                    reviews: Vec::new(),
                })
                .map_err(ApiError::storage)?,
            }),
        },

        "get_review" => match user.role {
            UserRole::Investor => get_review(pool, user, request.params).await,
            _ => Err(ApiError::forbidden()),
        },

        "save_review" => match user.role {
            UserRole::Investor => save_review(pool, user, request.params).await,
            _ => Err(ApiError::forbidden()),
        },
        _ => Err(ApiError::not_implemented("review::dispatch")),
    }
}

fn health() -> Result<RpcResponse, ApiError> {
    Ok(RpcResponse {
        result: serde_json::to_value(ReviewHealthResponse {
            status: "ok".into(),
            plugin_id: ID.into(),
        })
        .map_err(ApiError::storage)?,
    })
}

fn new_review_id() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);

    format!("rev_{}", URL_SAFE_NO_PAD.encode(bytes))
}
// Stub functions..

async fn list_criteria(pool: &PgPool, _user: &AuthenticatedUser) -> Result<RpcResponse, ApiError> {
    let rows = sqlx::query_as::<_, models::ReviewCriterionRow>(
        "SELECT id, title, review_question, display_order
         FROM review_criteria
         ORDER BY display_order ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(ApiError::storage)?;

    let criteria = rows
        .into_iter()
        .map(|row| types::ReviewCriterion {
            id: row.id,
            title: row.title,
            review_question: row.review_question,
            display_order: row.display_order,
        })
        .collect();

    let result = types::ListCriteriaResponse { criteria };

    Ok(RpcResponse {
        result: serde_json::to_value(result).map_err(ApiError::storage)?,
    })
}

async fn get_summary(pool: &PgPool, user: &AuthenticatedUser) -> Result<RpcResponse, ApiError> {
    let row = sqlx::query_as::<_, models::ReviewSummaryCountsRow>(
        "SELECT
            COUNT(*) AS total,
            COUNT(r.id) AS completed,
            COUNT(r.id) FILTER (WHERE r.status = 'satisfied') AS satisfied,
            COUNT(r.id) FILTER (WHERE r.status = 'needs_information') AS needs_information
         FROM review_criteria c
         LEFT JOIN reviews r
           ON r.criterion_id = c.id
          AND r.workspace_id = $1
          AND r.user_id = $2",
    )
    .bind(&user.workspace_id)
    .bind(&user.id)
    .fetch_one(pool)
    .await
    .map_err(ApiError::storage)?;

    let result = types::GetSummaryResponse {
        summary: types::ReviewSummary {
            completed: row.completed as i32,
            remaining: (row.total - row.completed) as i32,
            satisfied: row.satisfied as i32,
            needs_information: row.needs_information as i32,
        },
    };

    Ok(RpcResponse {
        result: serde_json::to_value(result).map_err(ApiError::storage)?,
    })
}

// list reviews..
async fn list_reviews(pool: &PgPool, user: &AuthenticatedUser) -> Result<RpcResponse, ApiError> {
    let rows = sqlx::query_as::<_, models::ReviewListItemRow>(
        "SELECT
            r.id,
            r.criterion_id,
            c.title AS criterion_title,
            r.status,
            r.opinion,
            r.updated_at::text
         FROM reviews r
         JOIN review_criteria c
           ON c.id = r.criterion_id
         WHERE r.workspace_id = $1
           AND r.user_id = $2
         ORDER BY c.display_order ASC, r.updated_at DESC, r.id ASC",
    )
    .bind(&user.workspace_id)
    .bind(&user.id)
    .fetch_all(pool)
    .await
    .map_err(ApiError::storage)?;

    let reviews = rows
        .into_iter()
        .map(|row| {
            let status = match row.status.as_str() {
                "satisfied" => types::ReviewStatus::Satisfied,
                "needs_information" => types::ReviewStatus::NeedsInformation,
                _ => {
                    // This should be impossible because the database has
                    // a CHECK constraint on reviews.status.
                    unreachable!("invalid review status in database")
                }
            };

            types::ReviewSummaryItem {
                id: row.id,
                criterion_id: row.criterion_id,
                criterion_title: row.criterion_title,
                status,
                opinion: row.opinion,
                updated_at: row.updated_at,
            }
        })
        .collect();

    let result = types::ListReviewsResponse { reviews };

    Ok(RpcResponse {
        result: serde_json::to_value(result).map_err(ApiError::storage)?,
    })
}

// get_review stub..
async fn get_review(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: serde_json::Value,
) -> Result<RpcResponse, ApiError> {
    let params: types::GetReviewParams = serde_json::from_value(params)
        .map_err(|_| ApiError::invalid("Invalid review parameters"))?;

    let rows = sqlx::query_as::<_, models::ReviewDetailJoinRow>(
        "SELECT
            r.id,
            r.criterion_id,
            c.title AS criterion_title,
            c.review_question,
            r.status,
            r.opinion,
            r.created_at::text,
            r.updated_at::text,
            m.id AS evidence_id,
            m.title AS evidence_title,
            m.file_name AS evidence_file_name,
            m.status AS evidence_status
         FROM reviews r
         JOIN review_criteria c
           ON c.id = r.criterion_id
         LEFT JOIN review_evidence re
           ON re.review_id = r.id
         LEFT JOIN materials m
           ON m.id = re.material_id
           AND m.workspace_id = r.workspace_id
         WHERE r.id = $1
           AND r.workspace_id = $2
           AND r.user_id = $3
         ORDER BY m.created_at ASC, m.id ASC",
    )
    .bind(&params.review_id)
    .bind(&user.workspace_id)
    .bind(&user.id)
    .fetch_all(pool)
    .await
    .map_err(ApiError::storage)?;

    let Some(first) = rows.first() else {
        return Err(ApiError::not_found());
    };

    let status = match first.status.as_str() {
        "satisfied" => types::ReviewStatus::Satisfied,
        "needs_information" => types::ReviewStatus::NeedsInformation,
        _ => unreachable!("invalid review status in database"),
    };

    let evidence = rows
        .iter()
        .filter_map(|row| {
            let id = row.evidence_id.clone()?;
            let title = row.evidence_title.clone()?;
            let file_name = row.evidence_file_name.clone()?;
            let status = row.evidence_status.clone()?;

            Some(types::ReviewEvidence {
                id,
                title,
                file_name,
                status,
            })
        })
        .collect();

    let review = types::ReviewDetail {
        id: first.id.clone(),
        criterion_id: first.criterion_id.clone(),
        criterion_title: first.criterion_title.clone(),
        review_question: first.review_question.clone(),
        status,
        opinion: first.opinion.clone(),
        evidence,
        created_at: first.created_at.clone(),
        updated_at: first.updated_at.clone(),
    };

    let result = types::GetReviewResponse { review };

    Ok(RpcResponse {
        result: serde_json::to_value(result).map_err(ApiError::storage)?,
    })
}

// save_review..
async fn save_review(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: serde_json::Value,
) -> Result<RpcResponse, ApiError> {
    let params: types::SaveReviewParams = serde_json::from_value(params)
        .map_err(|_| ApiError::invalid("Invalid review parameters."))?;

    if params.opinion.trim().is_empty() {
        return Err(ApiError::invalid("Review opinion cannot be empty."));
    }

    if params.opinion.chars().count() > 2000 {
        return Err(ApiError::invalid(
            "Review opinion cannot exceed 2000 characters.",
        ));
    }

    if params.evidence_material_ids.is_empty() {
        return Err(ApiError::invalid(
            "At least one evidence material is required.",
        ));
    }

    let mut unique_evidence_ids =
        std::collections::HashSet::with_capacity(params.evidence_material_ids.len());

    for material_id in &params.evidence_material_ids {
        if !unique_evidence_ids.insert(material_id) {
            return Err(ApiError::invalid("Evidence material IDs must be unique."));
        }
    }

    let criterion_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM review_criteria
            WHERE id = $1
        )",
    )
    .bind(&params.criterion_id)
    .fetch_one(pool)
    .await
    .map_err(ApiError::storage)?;

    if !criterion_exists {
        return Err(ApiError::invalid("Invalid review criterion."));
    }

    let valid_evidence_ids = sqlx::query_scalar::<_, String>(
        "SELECT id
         FROM materials
         WHERE id = ANY($1)
           AND workspace_id = $2
           AND status = 'ready'",
    )
    .bind(&params.evidence_material_ids)
    .bind(&user.workspace_id)
    .fetch_all(pool)
    .await
    .map_err(ApiError::storage)?;

    if valid_evidence_ids.len() != params.evidence_material_ids.len() {
        return Err(ApiError::invalid(
            "One or more evidence materials are invalid.",
        ));
    }

    let mut tx = pool.begin().await.map_err(ApiError::storage)?;

    let review_id = new_review_id();

    let review_status = match params.status {
        types::ReviewStatus::Satisfied => "satisfied",
        types::ReviewStatus::NeedsInformation => "needs_information",
    };

    let saved_review_id = sqlx::query_scalar::<_, String>(
        "INSERT INTO reviews (
            id,
            workspace_id,
            user_id,
            criterion_id,
            status,
            opinion
         )
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (workspace_id, criterion_id, user_id)
         DO UPDATE SET
            status = EXCLUDED.status,
            opinion = EXCLUDED.opinion,
            updated_at = NOW()
         RETURNING id",
    )
    .bind(&review_id)
    .bind(&user.workspace_id)
    .bind(&user.id)
    .bind(&params.criterion_id)
    .bind(review_status)
    .bind(&params.opinion)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::storage)?;

    sqlx::query(
        "DELETE FROM review_evidence
         WHERE review_id = $1",
    )
    .bind(&saved_review_id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::storage)?;

    for material_id in &params.evidence_material_ids {
        sqlx::query(
            "INSERT INTO review_evidence (review_id, material_id)
             VALUES ($1, $2)",
        )
        .bind(&saved_review_id)
        .bind(material_id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::storage)?;
    }

    let rows = sqlx::query_as::<_, models::ReviewDetailJoinRow>(
        "SELECT
            r.id,
            r.criterion_id,
            c.title AS criterion_title,
            c.review_question,
            r.status,
            r.opinion,
            r.created_at::text,
            r.updated_at::text,
            m.id AS evidence_id,
            m.title AS evidence_title,
            m.file_name AS evidence_file_name,
            m.status AS evidence_status
         FROM reviews r
         JOIN review_criteria c
           ON c.id = r.criterion_id
         LEFT JOIN review_evidence re
           ON re.review_id = r.id
         LEFT JOIN materials m
           ON m.id = re.material_id
          AND m.workspace_id = r.workspace_id
         WHERE r.id = $1
           AND r.workspace_id = $2
           AND r.user_id = $3
         ORDER BY m.created_at ASC, m.id ASC",
    )
    .bind(&saved_review_id)
    .bind(&user.workspace_id)
    .bind(&user.id)
    .fetch_all(&mut *tx)
    .await
    .map_err(ApiError::storage)?;

    let Some(first) = rows.first() else {
        return Err(ApiError::not_found());
    };

    let status = match first.status.as_str() {
        "satisfied" => types::ReviewStatus::Satisfied,
        "needs_information" => types::ReviewStatus::NeedsInformation,
        _ => unreachable!("invalid review status in database"),
    };

    let evidence = rows
        .iter()
        .filter_map(|row| {
            let id = row.evidence_id.clone()?;
            let title = row.evidence_title.clone()?;
            let file_name = row.evidence_file_name.clone()?;
            let status = row.evidence_status.clone()?;

            Some(types::ReviewEvidence {
                id,
                title,
                file_name,
                status,
            })
        })
        .collect();

    let review = types::ReviewDetail {
        id: first.id.clone(),
        criterion_id: first.criterion_id.clone(),
        criterion_title: first.criterion_title.clone(),
        review_question: first.review_question.clone(),
        status,
        opinion: first.opinion.clone(),
        evidence,
        created_at: first.created_at.clone(),
        updated_at: first.updated_at.clone(),
    };

    tx.commit().await.map_err(ApiError::storage)?;

    Ok(RpcResponse {
        result: serde_json::to_value(types::SaveReviewResponse { review })
            .map_err(ApiError::storage)?,
    })
}

#[cfg(test)]
mod tests;
