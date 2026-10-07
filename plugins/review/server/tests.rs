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

// save_review test...
#[tokio::test]
async fn save_review_forbidden_for_company() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let company = test_user(UserRole::Company, "lighthouse");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "satisfied",
            "opinion": "",
            "evidenceMaterialIds": []
        }),
    };

    let result = dispatch(&pool, &company, request).await;

    let error = result.expect_err("company should be forbidden from saving reviews");

    assert_eq!(error.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn save_review_rejects_empty_opinion() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user = test_user(UserRole::Investor, "lighthouse");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "satisfied",
            "opinion": "",
            "evidenceMaterialIds": ["mat-doc-business"]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    let error = result.expect_err("empty opinion should be rejected");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn save_review_rejects_whitespace_only_opinion() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user = test_user(UserRole::Investor, "lighthouse");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "satisfied",
            "opinion": "   \t  ",
            "evidenceMaterialIds": ["mat-doc-business"]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    let error = result.expect_err("whitespace-only opinion should be rejected");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn save_review_rejects_opinion_over_2000_characters() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user = test_user(UserRole::Investor, "lighthouse");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "satisfied",
            "opinion": "a".repeat(2001),
            "evidenceMaterialIds": ["mat-doc-business"]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    let error = result.expect_err("long opinion should be rejected");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn save_review_rejects_empty_evidence() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user = test_user(UserRole::Investor, "lighthouse");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "satisfied",
            "opinion": "Valid opinion",
            "evidenceMaterialIds": []
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    let error = result.expect_err("empty evidence should be rejected");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn save_review_rejects_duplicate_evidence() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user = test_user(UserRole::Investor, "lighthouse");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "satisfied",
            "opinion": "Valid opinion",
            "evidenceMaterialIds": [
                "mat-doc-business",
                "mat-doc-business"
            ]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    let error = result.expect_err("duplicate evidence should be rejected");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn save_review_rejects_invalid_status() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user = test_user(UserRole::Investor, "lighthouse");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "approved",
            "opinion": "Valid opinion",
            "evidenceMaterialIds": ["mat-doc-business"]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    let error = result.expect_err("invalid status should be rejected");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn save_review_rejects_invalid_criterion() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user = test_user(UserRole::Investor, "lighthouse");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "does-not-exist",
            "status": "satisfied",
            "opinion": "Valid opinion",
            "evidenceMaterialIds": ["mat-doc-business"]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    let error = result.expect_err("invalid criterion should be rejected");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn save_review_rejects_missing_evidence_material() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user = test_user(UserRole::Investor, "lighthouse");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "satisfied",
            "opinion": "Valid opinion",
            "evidenceMaterialIds": ["material-does-not-exist"]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    let error = result.expect_err("missing material should be rejected");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn save_review_rejects_processing_evidence_material() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user = test_user(UserRole::Investor, "lighthouse");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "satisfied",
            "opinion": "Valid opinion",
            "evidenceMaterialIds": ["mat-doc-pipeline"]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    let error = result.expect_err("processing material should be rejected");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn save_review_rejects_failed_evidence_material() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user = test_user(UserRole::Investor, "lighthouse");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "satisfied",
            "opinion": "Valid opinion",
            "evidenceMaterialIds": ["mat-doc-revenue"]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    let error = result.expect_err("failed material should be rejected");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn save_review_rejects_cross_workspace_evidence() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let workspace_id = "test-other-workspace";
    let material_id = "test-other-workspace-material";

    let user = test_user(UserRole::Investor, "lighthouse");

    sqlx::query(
        "INSERT INTO workspaces (id, name)
     VALUES ($1, $2)",
    )
    .bind(workspace_id)
    .bind("Test Other Workspace")
    .execute(&pool)
    .await
    .expect("failed to create test workspace");

    sqlx::query(
        "INSERT INTO materials
        (id, workspace_id, uploader_id, title, file_name, content, status)
     VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(material_id)
    .bind(workspace_id)
    .bind("company-user")
    .bind("Other Workspace Material")
    .bind("other.md")
    .bind("Other workspace content")
    .bind("ready")
    .execute(&pool)
    .await
    .expect("failed to create cross-workspace material");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "satisfied",
            "opinion": "Valid opinion",
            "evidenceMaterialIds": [material_id]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    sqlx::query("DELETE FROM materials WHERE id = $1")
        .bind(material_id)
        .execute(&pool)
        .await
        .expect("failed to clean up test material");

    sqlx::query("DELETE FROM workspaces WHERE id = $1")
        .bind(workspace_id)
        .execute(&pool)
        .await
        .expect("failed to clean up test workspace");

    let error = result.expect_err("cross-workspace evidence should be rejected");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn save_review_creates_new_review_with_evidence() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user_id = "save-review-create-investor";
    let user = create_test_investor(&pool, user_id).await;

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "satisfied",
            "opinion": "The business model is clearly documented.",
            "evidenceMaterialIds": ["mat-doc-business"]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    let response = result.expect("save_review should succeed");

    let result: types::SaveReviewResponse =
        serde_json::from_value(response.result)
            .expect("response should deserialize");

    let review = result.review;

    assert!(review.id.starts_with("rev_"));
    assert_eq!(review.criterion_id, "business");
    assert_eq!(review.criterion_title, "사업 이해");
    assert_eq!(
        review.review_question,
        "사업 모델과 고객·시장에 관한 핵심 내용이 자료로 확인되는가?"
    );
    assert_eq!(review.status, types::ReviewStatus::Satisfied);
    assert_eq!(
        review.opinion,
        "The business model is clearly documented."
    );

    assert_eq!(review.evidence.len(), 1);
    assert_eq!(review.evidence[0].id, "mat-doc-business");
    assert_eq!(review.evidence[0].title, "회사 소개");
    assert_eq!(review.evidence[0].file_name, "company-overview.md");
    assert_eq!(review.evidence[0].status, "ready");

    let saved_review = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id, workspace_id, user_id, criterion_id, status
         FROM reviews
         WHERE id = $1",
    )
    .bind(&review.id)
    .fetch_one(&pool)
    .await
    .expect("saved review should exist in database");

    assert_eq!(saved_review.0, review.id);
    assert_eq!(saved_review.1, "lighthouse");
    assert_eq!(saved_review.2, user_id);
    assert_eq!(saved_review.3, "business");
    assert_eq!(saved_review.4, "satisfied");

    let evidence_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM review_evidence
         WHERE review_id = $1",
    )
    .bind(&review.id)
    .fetch_one(&pool)
    .await
    .expect("failed to count saved evidence");

    assert_eq!(evidence_count, 1);

    sqlx::query("DELETE FROM reviews WHERE id = $1")
        .bind(&review.id)
        .execute(&pool)
        .await
        .expect("failed to clean up test review");

    delete_test_user(&pool, user_id).await;
}

#[tokio::test]
async fn save_review_updates_existing_review_and_replaces_evidence() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user_id = "save-review-update-investor";
    let review_id = "save-review-update-existing";

    // Clean up leftovers from a previous interrupted test run.
    sqlx::query("DELETE FROM reviews WHERE id = $1")
        .bind(review_id)
        .execute(&pool)
        .await
        .expect("failed to clean up previous test review");

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("failed to clean up previous test user");

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
    .bind("Initial opinion")
    .execute(&pool)
    .await
    .expect("failed to insert existing review");

    sqlx::query(
        "INSERT INTO review_evidence (review_id, material_id)
         VALUES ($1, $2)",
    )
    .bind(review_id)
    .bind("mat-doc-business")
    .execute(&pool)
    .await
    .expect("failed to insert initial evidence");

    let original_timestamps = sqlx::query_as::<_, (String, String)>(
        "SELECT created_at::text, updated_at::text
         FROM reviews
         WHERE id = $1",
    )
    .bind(review_id)
    .fetch_one(&pool)
    .await
    .expect("failed to fetch original timestamps");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "needs_information",
            "opinion": "Updated opinion after reviewing more information.",
            "evidenceMaterialIds": ["mat-doc-team"]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    let response = result.expect("save_review update should succeed");

    let result: types::SaveReviewResponse =
        serde_json::from_value(response.result)
            .expect("response should deserialize");

    let review = result.review;

    assert_eq!(review.id, review_id);
    assert_eq!(review.status, types::ReviewStatus::NeedsInformation);
    assert_eq!(
        review.opinion,
        "Updated opinion after reviewing more information."
    );

    assert_eq!(review.evidence.len(), 1);
    assert_eq!(review.evidence[0].id, "mat-doc-team");
    assert_eq!(review.evidence[0].title, "팀 소개");
    assert_eq!(review.evidence[0].file_name, "team.md");
    assert_eq!(review.evidence[0].status, "ready");

    assert_eq!(review.created_at, original_timestamps.0);
    assert_ne!(review.updated_at, original_timestamps.1);

    let database_row = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT
            id,
            workspace_id,
            user_id,
            criterion_id,
            status,
            opinion
         FROM reviews
         WHERE workspace_id = $1
           AND user_id = $2
           AND criterion_id = $3",
    )
    .bind("lighthouse")
    .bind(user_id)
    .bind("business")
    .fetch_one(&pool)
    .await
    .expect("updated review should exist");

    assert_eq!(database_row.0, review_id);
    assert_eq!(database_row.1, "lighthouse");
    assert_eq!(database_row.2, user_id);
    assert_eq!(database_row.3, "business");
    assert_eq!(database_row.4, "needs_information");
    assert_eq!(
        database_row.5,
        "Updated opinion after reviewing more information."
    );

    let evidence_ids = sqlx::query_as::<_, (String,)>(
        "SELECT material_id
         FROM review_evidence
         WHERE review_id = $1
         ORDER BY material_id",
    )
    .bind(review_id)
    .fetch_all(&pool)
    .await
    .expect("failed to fetch updated evidence");

    assert_eq!(evidence_ids, vec![("mat-doc-team".to_string(),)]);

    let review_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM reviews
         WHERE workspace_id = $1
           AND user_id = $2
           AND criterion_id = $3",
    )
    .bind("lighthouse")
    .bind(user_id)
    .bind("business")
    .fetch_one(&pool)
    .await
    .expect("failed to count reviews");

    assert_eq!(review_count, 1);

    sqlx::query("DELETE FROM reviews WHERE id = $1")
        .bind(review_id)
        .execute(&pool)
        .await
        .expect("failed to clean up test review");

    delete_test_user(&pool, user_id).await;
}

#[tokio::test]
async fn save_review_rolls_back_review_and_evidence_on_database_failure() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user_id = "save-review-rollback-investor";
    let review_id = "save-review-rollback-existing";
    let trigger_function = "test_fail_save_review";
    let trigger_name = "test_fail_save_review_trigger";

    // Clean up leftovers from a previous interrupted test run.
    sqlx::query("DELETE FROM reviews WHERE id = $1")
        .bind(review_id)
        .execute(&pool)
        .await
        .expect("failed to clean up previous test review");

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("failed to clean up previous test user");

    let user = create_test_investor(&pool, user_id).await;

    // an existing review with its original evidence.
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
    .bind("Original opinion")
    .execute(&pool)
    .await
    .expect("failed to insert existing review");

    sqlx::query(
        "INSERT INTO review_evidence (review_id, material_id)
         VALUES ($1, $2)",
    )
    .bind(review_id)
    .bind("mat-doc-business")
    .execute(&pool)
    .await
    .expect("failed to insert original evidence");

    // temporary trigger function.
    sqlx::query(&format!(
        r#"
        CREATE OR REPLACE FUNCTION {trigger_function}()
        RETURNS trigger AS $$
        BEGIN
            IF NEW.id = '{review_id}' THEN
                RAISE EXCEPTION 'intentional test failure for rollback';
            END IF;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql
        "#
    ))
    .execute(&pool)
    .await
    .expect("failed to create rollback test function");

    // the trigger separately
    sqlx::query(&format!(
        r#"
        CREATE TRIGGER {trigger_name}
        AFTER INSERT OR UPDATE ON reviews
        FOR EACH ROW
        EXECUTE FUNCTION {trigger_function}()
        "#
    ))
    .execute(&pool)
    .await
    .expect("failed to create rollback test trigger");

    let request = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "needs_information",
            "opinion": "This update must be rolled back.",
            "evidenceMaterialIds": ["mat-doc-team"]
        }),
    };

    let result = dispatch(&pool, &user, request).await;

    // db failure converted into a storage error.
    assert!(result.is_err());

    // temporary trigger removed before inspecting the final state.
    sqlx::query(&format!(
        "DROP TRIGGER {trigger_name} ON reviews"
    ))
    .execute(&pool)
    .await
    .expect("failed to remove rollback test trigger");

    sqlx::query(&format!(
        "DROP FUNCTION {trigger_function}()"
    ))
    .execute(&pool)
    .await
    .expect("failed to remove rollback test function");

    // The original review still exist unchanged.
    let review_row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, status, opinion
         FROM reviews
         WHERE id = $1",
    )
    .bind(review_id)
    .fetch_one(&pool)
    .await
    .expect("original review should still exist");

    assert_eq!(review_row.0, review_id);
    assert_eq!(review_row.1, "satisfied");
    assert_eq!(review_row.2, "Original opinion");

    // The original evidence also still exist.
    let evidence_ids = sqlx::query_as::<_, (String,)>(
        "SELECT material_id
         FROM review_evidence
         WHERE review_id = $1
         ORDER BY material_id",
    )
    .bind(review_id)
    .fetch_all(&pool)
    .await
    .expect("failed to fetch original evidence");

    assert_eq!(
        evidence_ids,
        vec![("mat-doc-business".to_string(),)]
    );

    // attempted replacement evidence was not persisted...
    let replacement_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM review_evidence
         WHERE review_id = $1
           AND material_id = $2",
    )
    .bind(review_id)
    .bind("mat-doc-team")
    .fetch_one(&pool)
    .await
    .expect("failed to check replacement evidence");

    assert_eq!(replacement_count, 0);

    sqlx::query("DELETE FROM reviews WHERE id = $1")
        .bind(review_id)
        .execute(&pool)
        .await
        .expect("failed to clean up test review");

    delete_test_user(&pool, user_id).await;
}

#[tokio::test]
async fn save_review_handles_concurrent_saves_for_same_criterion() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let user_id = "save-review-concurrency-investor";
    let review_id_prefix = "save-review-concurrency";

    sqlx::query(
        "DELETE FROM reviews
         WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("failed to clean up previous test reviews");

    sqlx::query(
        "DELETE FROM users
         WHERE id = $1",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("failed to clean up previous test user");

    let user = create_test_investor(&pool, user_id).await;

    let request_one = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "satisfied",
            "opinion": "Concurrent save from request one.",
            "evidenceMaterialIds": ["mat-doc-business"]
        }),
    };

    let request_two = PluginRpcRequest {
        plugin_id: ID.to_string(),
        workspace_id: "lighthouse".to_string(),
        method: "save_review".to_string(),
        params: serde_json::json!({
            "criterionId": "business",
            "status": "needs_information",
            "opinion": "Concurrent save from request two.",
            "evidenceMaterialIds": ["mat-doc-team"]
        }),
    };

    let (result_one, result_two) = tokio::join!(
        dispatch(&pool, &user, request_one),
        dispatch(&pool, &user, request_two)
    );

    assert!(
        result_one.is_ok(),
        "first concurrent save should succeed: {:?}",
        result_one.err()
    );

    assert!(
        result_two.is_ok(),
        "second concurrent save should succeed: {:?}",
        result_two.err()
    );

    // exactly one review for this
    // workspace + investor + criterion combination.
    let review_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM reviews
         WHERE workspace_id = $1
           AND user_id = $2
           AND criterion_id = $3",
    )
    .bind("lighthouse")
    .bind(user_id)
    .bind("business")
    .fetch_one(&pool)
    .await
    .expect("failed to count concurrent reviews");

    assert_eq!(
        review_count, 1,
        "concurrent saves must result in exactly one review"
    );

    let saved_reviews = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, status, opinion
         FROM reviews
         WHERE workspace_id = $1
           AND user_id = $2
           AND criterion_id = $3",
    )
    .bind("lighthouse")
    .bind(user_id)
    .bind("business")
    .fetch_all(&pool)
    .await
    .expect("failed to fetch concurrent reviews");

    assert_eq!(saved_reviews.len(), 1);

    let saved_review = &saved_reviews[0];

    assert!(
        (saved_review.1 == "satisfied" && saved_review.2 == "Concurrent save from request one.")
            || (saved_review.1 == "needs_information"
                && saved_review.2 == "Concurrent save from request two."),
        "final review must match one of the concurrent saves"
    );

    let evidence_rows = sqlx::query_as::<_, (String,)>(
        "SELECT material_id
         FROM review_evidence
         WHERE review_id = $1",
    )
    .bind(&saved_review.0)
    .fetch_all(&pool)
    .await
    .expect("failed to fetch concurrent evidence");

    assert_eq!(
        evidence_rows.len(), 1,
        "final evidence count must be exactly 1"
    );

    assert!(
        evidence_rows[0].0 == "mat-doc-business"
            || evidence_rows[0].0 == "mat-doc-team",
        "final evidence must come from one of the concurrent saves"
    );

    // Cleanup.
    sqlx::query(
        "DELETE FROM reviews
         WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("failed to clean up concurrent test reviews");

    delete_test_user(&pool, user_id).await;

    let _ = review_id_prefix;
}
