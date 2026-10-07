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

// list reviews..
async fn list_reviews(pool: &PgPool, user: &AuthenticatedUser) -> Result<RpcResponse, ApiError> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT
            r.id,
            r.criterion_id,
            c.title,
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
        .map(
            |(id, criterion_id, criterion_title, status, opinion, updated_at)| {
                let status = match status.as_str() {
                    "satisfied" => types::ReviewStatus::Satisfied,
                    "needs_information" => types::ReviewStatus::NeedsInformation,
                    _ => {
                        // This should be impossible because the database has
                        // a CHECK constraint on reviews.status.
                        unreachable!("invalid review status in database")
                    }
                };

                types::ReviewSummaryItem {
                    id,
                    criterion_id,
                    criterion_title,
                    status,
                    opinion,
                    updated_at,
                }
            },
        )
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

    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT
            r.id,
            r.criterion_id,
            c.title,
            c.review_question,
            r.status,
            r.opinion,
            r.created_at::text,
            r.updated_at::text,
            m.id,
            m.title,
            m.file_name,
            m.status
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

    let status = match first.4.as_str() {
        "satisfied" => types::ReviewStatus::Satisfied,
        "needs_information" => types::ReviewStatus::NeedsInformation,
        _ => unreachable!("invalid review status in database"),
    };

    let evidence = rows
        .iter()
        .filter_map(|row| {
            let id = row.8.clone()?;
            let title = row.9.clone()?;
            let file_name = row.10.clone()?;
            let status = row.11.clone()?;

            Some(types::ReviewEvidence {
                id,
                title,
                file_name,
                status,
            })
        })
        .collect();

    let review = types::ReviewDetail {
        id: first.0.clone(),
        criterion_id: first.1.clone(),
        criterion_title: first.2.clone(),
        review_question: first.3.clone(),
        status,
        opinion: first.5.clone(),
        evidence,
        created_at: first.6.clone(),
        updated_at: first.7.clone(),
    };

    let result = types::GetReviewResponse { review };

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

    async fn create_test_investor(pool: &PgPool, user_id: &str) -> AuthenticatedUser {
        sqlx::query(
            "INSERT INTO users (id, email, display_name, password_hash)
         VALUES ($1, $2, $3, $4)",
        )
        .bind(user_id)
        .bind(format!("{user_id}@test.local"))
        .bind("Test Investor")
        .bind("test-password-hash")
        .execute(pool)
        .await
        .expect("failed to create test investor");

        AuthenticatedUser {
            id: user_id.to_string(),
            name: "Test Investor".to_string(),
            role: UserRole::Investor,
            workspace_id: "lighthouse".to_string(),
            workspace_name: "Lighthouse".to_string(),
        }
    }

    async fn delete_test_user(pool: &PgPool, user_id: &str) {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .expect("failed to delete test investor");
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
            .expect("failed to connect to database");

        let user_id = "get-summary-empty-investor";

        let user = create_test_investor(&pool, user_id).await;

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
            serde_json::from_value(response.result).expect("response should deserialize");

        assert_eq!(result.summary.completed, 0);
        assert_eq!(result.summary.remaining, 3);
        assert_eq!(result.summary.satisfied, 0);
        assert_eq!(result.summary.needs_information, 0);

        delete_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    async fn get_summary_counts_needs_information_as_completed() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let user_id = "get-summary-needs-info-investor";
        let review_id = "test-review-summary-needs-information";

        let user = create_test_investor(&pool, user_id).await;

        sqlx::query(
            "INSERT INTO reviews
            (id, workspace_id, user_id, criterion_id, status, opinion)
         VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(review_id)
        .bind("lighthouse")
        .bind(user_id)
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
            .bind(review_id)
            .execute(&pool)
            .await
            .expect("failed to clean up test review");

        delete_test_user(&pool, user_id).await;

        let response = result.expect("get_summary should succeed");

        let result: types::GetSummaryResponse =
            serde_json::from_value(response.result).expect("response should deserialize");

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
            .expect("failed to connect to database");

        let primary_user_id = "get-summary-isolation-primary";
        let peer_user_id = "get-summary-isolation-peer";
        let review_id = "test-review-summary-peer";

        let primary_user = create_test_investor(&pool, primary_user_id).await;

        create_test_investor(&pool, peer_user_id).await;

        sqlx::query(
            "INSERT INTO reviews
            (id, workspace_id, user_id, criterion_id, status, opinion)
         VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(review_id)
        .bind("lighthouse")
        .bind(peer_user_id)
        .bind("business")
        .bind("satisfied")
        .bind("Peer investor review")
        .execute(&pool)
        .await
        .expect("failed to insert test review");

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "get_summary".to_string(),
            params: serde_json::json!({}),
        };

        let result = dispatch(&pool, &primary_user, request).await;

        sqlx::query("DELETE FROM reviews WHERE id = $1")
            .bind(review_id)
            .execute(&pool)
            .await
            .expect("failed to clean up test review");

        delete_test_user(&pool, primary_user_id).await;
        delete_test_user(&pool, peer_user_id).await;

        let response = result.expect("get_summary should succeed");

        let result: types::GetSummaryResponse =
            serde_json::from_value(response.result).expect("response should deserialize");

        assert_eq!(result.summary.completed, 0);
        assert_eq!(result.summary.remaining, 3);
        assert_eq!(result.summary.satisfied, 0);
        assert_eq!(result.summary.needs_information, 0);
    }

    // list review tests..
    #[tokio::test]
    async fn list_reviews_returns_empty_for_investor_without_reviews() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let user_id = "list-reviews-empty-investor";

        let user = create_test_investor(&pool, user_id).await;

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "list_reviews".to_string(),
            params: serde_json::json!({}),
        };

        let response = dispatch(&pool, &user, request)
            .await
            .expect("list_reviews should succeed");

        let result: types::ListReviewsResponse =
            serde_json::from_value(response.result).expect("response should deserialize");

        assert!(result.reviews.is_empty());

        delete_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    async fn list_reviews_returns_investors_reviews() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let user_id = "list-reviews-own-investor";

        let user = create_test_investor(&pool, user_id).await;

        let review_id = "test-list-reviews-investor";

        sqlx::query(
            "INSERT INTO reviews
            (id, workspace_id, user_id, criterion_id, status, opinion)
         VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(review_id)
        .bind("lighthouse")
        .bind(user_id)
        .bind("business")
        .bind("satisfied")
        .bind("Business model is clearly documented.")
        .execute(&pool)
        .await
        .expect("failed to insert test review");

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "list_reviews".to_string(),
            params: serde_json::json!({}),
        };

        let result = dispatch(&pool, &user, request).await;

        sqlx::query("DELETE FROM reviews WHERE id = $1")
            .bind(review_id)
            .execute(&pool)
            .await
            .expect("failed to clean up test review");

        delete_test_user(&pool, user_id).await;

        let response = result.expect("list_reviews should succeed");

        let result: types::ListReviewsResponse =
            serde_json::from_value(response.result).expect("response should deserialize");

        assert_eq!(result.reviews.len(), 1);

        let review = &result.reviews[0];

        assert_eq!(review.id, review_id);
        assert_eq!(review.criterion_id, "business");
        assert_eq!(review.criterion_title, "사업 이해");
        assert_eq!(review.status, types::ReviewStatus::Satisfied);
        assert_eq!(review.opinion, "Business model is clearly documented.");
    }

    #[tokio::test]
    async fn list_reviews_does_not_return_another_investors_reviews() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let own_user_id = "list-reviews-isolation-own";
        let peer_user_id = "list-reviews-isolation-peer";

        let own_review_id = "test-list-reviews-isolation-own";
        let peer_review_id = "test-list-reviews-isolation-peer";

        let user = create_test_investor(&pool, own_user_id).await;

        create_test_investor(&pool, peer_user_id).await;

        sqlx::query(
            "INSERT INTO reviews
            (id, workspace_id, user_id, criterion_id, status, opinion)
         VALUES
            ($1, $2, $3, $4, $5, $6),
            ($7, $2, $8, $9, $10, $11)",
        )
        .bind(own_review_id)
        .bind("lighthouse")
        .bind(own_user_id)
        .bind("revenue")
        .bind("satisfied")
        .bind("Own review")
        .bind(peer_review_id)
        .bind(peer_user_id)
        .bind("team")
        .bind("needs_information")
        .bind("Peer review")
        .execute(&pool)
        .await
        .expect("failed to insert test reviews");

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "list_reviews".to_string(),
            params: serde_json::json!({}),
        };

        let result = dispatch(&pool, &user, request).await;

        sqlx::query("DELETE FROM reviews WHERE id IN ($1, $2)")
            .bind(own_review_id)
            .bind(peer_review_id)
            .execute(&pool)
            .await
            .expect("failed to clean up test reviews");

        delete_test_user(&pool, own_user_id).await;
        delete_test_user(&pool, peer_user_id).await;

        let response = result.expect("list_reviews should succeed");

        let result: types::ListReviewsResponse =
            serde_json::from_value(response.result).expect("response should deserialize");

        assert_eq!(result.reviews.len(), 1);
        assert_eq!(result.reviews[0].id, own_review_id);
    }

    #[tokio::test]
    async fn list_reviews_returns_empty_for_company() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let user = AuthenticatedUser {
            id: "company-user".to_string(),
            name: "Company User".to_string(),
            role: UserRole::Company,
            workspace_id: "lighthouse".to_string(),
            workspace_name: "Lighthouse".to_string(),
        };

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "list_reviews".to_string(),
            params: serde_json::json!({}),
        };

        let response = dispatch(&pool, &user, request)
            .await
            .expect("company list_reviews should succeed");

        let result: types::ListReviewsResponse =
            serde_json::from_value(response.result).expect("response should deserialize");

        assert!(result.reviews.is_empty());
    }

    // get_review tests..

    #[tokio::test]
    async fn get_review_returns_own_review_with_evidence() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let user_id = "get-review-own-investor";
        let review_id = "test-get-review-own";
        let evidence_id = "mat-doc-business";

        let user = create_test_investor(&pool, user_id).await;

        sqlx::query(
            "INSERT INTO reviews
            (id, workspace_id, user_id, criterion_id, status, opinion)
         VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(review_id)
        .bind("lighthouse")
        .bind(user_id)
        .bind("business")
        .bind("satisfied")
        .bind("Business model is clearly documented.")
        .execute(&pool)
        .await
        .expect("failed to insert test review");

        sqlx::query(
            "INSERT INTO review_evidence (review_id, material_id)
         VALUES ($1, $2)",
        )
        .bind(review_id)
        .bind(evidence_id)
        .execute(&pool)
        .await
        .expect("failed to insert test evidence");

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "get_review".to_string(),
            params: serde_json::json!({
                "reviewId": review_id
            }),
        };

        let result = dispatch(&pool, &user, request).await;

        sqlx::query("DELETE FROM reviews WHERE id = $1")
            .bind(review_id)
            .execute(&pool)
            .await
            .expect("failed to clean up test review");

        delete_test_user(&pool, user_id).await;

        let response = result.expect("get_review should succeed");

        let result: types::GetReviewResponse =
            serde_json::from_value(response.result).expect("response should deserialize");

        let review = result.review;

        assert_eq!(review.id, review_id);
        assert_eq!(review.criterion_id, "business");
        assert_eq!(review.criterion_title, "사업 이해");
        assert_eq!(
            review.review_question,
            "사업 모델과 고객·시장에 관한 핵심 내용이 자료로 확인되는가?"
        );
        assert_eq!(review.status, types::ReviewStatus::Satisfied);
        assert_eq!(review.opinion, "Business model is clearly documented.");

        assert_eq!(review.evidence.len(), 1);
        assert_eq!(review.evidence[0].id, evidence_id);
        assert_eq!(review.evidence[0].title, "회사 소개");
        assert_eq!(review.evidence[0].file_name, "company-overview.md");
        assert_eq!(review.evidence[0].status, "ready");
    }

    #[tokio::test]
    async fn get_review_returns_not_found_for_missing_review() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let user_id = "get-review-missing-investor";
        let user = create_test_investor(&pool, user_id).await;

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "get_review".to_string(),
            params: serde_json::json!({
                "reviewId": "review-that-does-not-exist"
            }),
        };

        let result = dispatch(&pool, &user, request).await;

        delete_test_user(&pool, user_id).await;

        let error = result.expect_err("missing review should return an error");

        assert_eq!(error.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_review_returns_not_found_for_another_investor() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let owner_id = "get-review-owner-investor";
        let other_id = "get-review-other-investor";
        let review_id = "test-get-review-private";

        create_test_investor(&pool, owner_id).await;
        let other = create_test_investor(&pool, other_id).await;

        sqlx::query(
            "INSERT INTO reviews
            (id, workspace_id, user_id, criterion_id, status, opinion)
         VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(review_id)
        .bind("lighthouse")
        .bind(owner_id)
        .bind("business")
        .bind("satisfied")
        .bind("Private review")
        .execute(&pool)
        .await
        .expect("failed to insert test review");

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "get_review".to_string(),
            params: serde_json::json!({
                "reviewId": review_id
            }),
        };

        let result = dispatch(&pool, &other, request).await;

        sqlx::query("DELETE FROM reviews WHERE id = $1")
            .bind(review_id)
            .execute(&pool)
            .await
            .expect("failed to clean up test review");

        delete_test_user(&pool, owner_id).await;
        delete_test_user(&pool, other_id).await;

        let error = result.expect_err("another investor should not access the review");

        assert_eq!(error.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_review_forbidden_for_company() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("failed to connect to database");

        let investor_id = "get-review-company-investor";
        let review_id = "test-get-review-company-private";

        create_test_investor(&pool, investor_id).await;

        sqlx::query(
            "INSERT INTO reviews
            (id, workspace_id, user_id, criterion_id, status, opinion)
         VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(review_id)
        .bind("lighthouse")
        .bind(investor_id)
        .bind("business")
        .bind("satisfied")
        .bind("Private investor review")
        .execute(&pool)
        .await
        .expect("failed to insert test review");

        let company = test_user(UserRole::Company, "lighthouse");

        let request = PluginRpcRequest {
            plugin_id: ID.to_string(),
            workspace_id: "lighthouse".to_string(),
            method: "get_review".to_string(),
            params: serde_json::json!({
                "reviewId": review_id
            }),
        };

        let result = dispatch(&pool, &company, request).await;

        sqlx::query("DELETE FROM reviews WHERE id = $1")
            .bind(review_id)
            .execute(&pool)
            .await
            .expect("failed to clean up test review");

        delete_test_user(&pool, investor_id).await;

        let error = result.expect_err("company should not access investor review");

        assert_eq!(error.0, StatusCode::FORBIDDEN);
    }
}
