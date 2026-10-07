pub mod models;
pub mod types;

use crate::{
    auth::AuthenticatedUser,
    error::ApiError,
    types::{PluginRpcRequest, RpcResponse, UserRole},
};
use sqlx::PgPool;
use types::ReviewHealthResponse;

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
        // "list_reviews" => list_reviews(pool, user).await,
        // "get_review" => match user.role {

        //     UserRole::Investor => get_review(pool, user, request.params).await,
        //     _ => Err(ApiError::forbidden())
        // }
        // "save_review" => match user.role{

        //     UserRole::Investor => save_review(pool, user, request.params).await,
        //     _ => Err(ApiError::forbidden())
        // }
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

// Stub functions..

async fn list_criteria(pool: &PgPool, _user: &AuthenticatedUser) -> Result<RpcResponse, ApiError> {
    let rows = sqlx::query_as::<_, (String, String, String, i16)>(
        "SELECT id, title, review_question, display_order
         FROM review_criteria
         ORDER BY display_order ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(ApiError::storage)?;

    let criteria = rows
        .into_iter()
        .map(
            |(id, title, review_question, display_order)| types::ReviewCriterion {
                id,
                title,
                review_question,
                display_order,
            },
        )
        .collect();

    let result = types::ListCriteriaResponse { criteria };

    Ok(RpcResponse {
        result: serde_json::to_value(result).map_err(ApiError::storage)?,
    })
}

async fn get_summary(pool: &PgPool, user: &AuthenticatedUser) -> Result<RpcResponse, ApiError> {
    let row = sqlx::query_as::<_, (i64, i64, i64, i64)>(
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

    let (total, completed, satisfied, needs_information) = row;

    let result = types::GetSummaryResponse {
        summary: types::ReviewSummary {
            completed: completed as i32,
            remaining: total as i32 - completed as i32,
            satisfied: satisfied as i32,
            needs_information: needs_information as i32,
        },
    };

    Ok(RpcResponse {
        result: serde_json::to_value(result).map_err(ApiError::storage)?,
    })
}

// tests..

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
        let user = test_user(UserRole::Investor, "workspace-a");

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "workspace-b".to_string(),
            method: "list_criteria".to_string(),
            params: serde_json::json!({}),
        };

        let result = dispatch(&pool, &user, request).await;
        let error = result.expect_err("cross-workspace request should be rejected");

        assert_eq!(error.0, StatusCode::FORBIDDEN);
        assert_eq!(error.1, "forbidden");
    }

    #[tokio::test]
    async fn list_criteria_returns_fixed_criteria_in_display_order() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to test database");

        let user = test_user(UserRole::Investor, "lighthouse");

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "list_criteria".to_string(),
            params: serde_json::json!({}),
        };

        let response = dispatch(&pool, &user, request)
            .await
            .expect("list_criteria should succeed");

        let result: types::ListCriteriaResponse =
            serde_json::from_value(response.result).expect("invalid list_criteria response");

        let ids: Vec<&str> = result
            .criteria
            .iter()
            .map(|criterion| criterion.id.as_str())
            .collect();

        assert_eq!(ids, vec!["business", "team", "revenue"]);

        let display_orders: Vec<i16> = result
            .criteria
            .iter()
            .map(|criterion| criterion.display_order)
            .collect();

        assert_eq!(display_orders, vec![1, 2, 3]);
    }

    // get summary tests..
    #[tokio::test]
    async fn get_summary_returns_all_criteria_as_remaining_for_new_investor() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to test database");

        let user = test_user(UserRole::Investor, "lighthouse");

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "get_summary".to_string(),
            params: serde_json::json!({}),
        };

        let response = dispatch(&pool, &user, request)
            .await
            .expect("get_summary should succeed");

        let result: types::GetSummaryResponse =
            serde_json::from_value(response.result).expect("invalid get_summary response");

        assert_eq!(result.summary.completed, 0);
        assert_eq!(result.summary.remaining, 3);
        assert_eq!(result.summary.satisfied, 0);
        assert_eq!(result.summary.needs_information, 0);
    }

    #[tokio::test]
    async fn get_summary_counts_needs_information_as_completed() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to test database");

        let user = AuthenticatedUser {
            id: "investor-user".to_string(),
            name: "Investor User".to_string(),
            role: UserRole::Investor,
            workspace_id: "lighthouse".to_string(),
            workspace_name: "Lighthouse".to_string(),
        };

        sqlx::query(
            "INSERT INTO reviews (
            id,
            workspace_id,
            user_id,
            criterion_id,
            status,
            opinion
         )
         VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind("test-review-summary-needs-information")
        .bind("lighthouse")
        .bind(&user.id)
        .bind("business")
        .bind("needs_information")
        .bind("추가 확인이 필요합니다.")
        .execute(&pool)
        .await
        .expect("failed to insert test review");

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "get_summary".to_string(),
            params: serde_json::json!({}),
        };

        let result = dispatch(&pool, &user, request).await;

        sqlx::query("DELETE FROM reviews WHERE id = $1")
            .bind("test-review-summary-needs-information")
            .execute(&pool)
            .await
            .expect("failed to clean up test review");

        let response = result.expect("get_summary should succeed");

        let result: types::GetSummaryResponse =
            serde_json::from_value(response.result).expect("invalid get_summary response");

        assert_eq!(result.summary.completed, 1);
        assert_eq!(result.summary.remaining, 2);
        assert_eq!(result.summary.satisfied, 0);
        assert_eq!(result.summary.needs_information, 1);
    }

    #[tokio::test]
    async fn get_summary_ignores_reviews_from_another_investor() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to test database");

        let primary_investor = AuthenticatedUser {
            id: "investor-user".to_string(),
            name: "Primary Investor".to_string(),
            role: UserRole::Investor,
            workspace_id: "lighthouse".to_string(),
            workspace_name: "Lighthouse".to_string(),
        };

        sqlx::query(
            "INSERT INTO reviews (
            id,
            workspace_id,
            user_id,
            criterion_id,
            status,
            opinion
         )
         VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind("test-review-summary-peer")
        .bind("lighthouse")
        .bind("investor-peer")
        .bind("business")
        .bind("satisfied")
        .bind("Peer investor review")
        .execute(&pool)
        .await
        .expect("failed to insert peer review");

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "get_summary".to_string(),
            params: serde_json::json!({}),
        };

        let result = dispatch(&pool, &primary_investor, request).await;

        sqlx::query("DELETE FROM reviews WHERE id = $1")
            .bind("test-review-summary-peer")
            .execute(&pool)
            .await
            .expect("failed to clean up peer review");

        let response = result.expect("get_summary should succeed");

        let result: types::GetSummaryResponse =
            serde_json::from_value(response.result).expect("invalid get_summary response");

        assert_eq!(result.summary.completed, 0);
        assert_eq!(result.summary.remaining, 3);
        assert_eq!(result.summary.satisfied, 0);
        assert_eq!(result.summary.needs_information, 0);
    }
}
