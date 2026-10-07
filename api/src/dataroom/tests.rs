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
