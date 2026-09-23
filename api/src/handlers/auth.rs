use std::sync::Arc;

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{Extension, Json};
use sqlx::PgPool;
use tower_cookies::Cookies;
use ts_server_fn::{get, post};

use crate::{
    auth::{AuthState, OptionalUser, models::LoginUserRow, new_session_id},
    error::ApiError,
    types::{GetSessionResponse, HostSession, LoginRequest, LogoutResponse},
};

#[post("/api/auth/login")]
pub async fn login_handler(
    Extension(pool): Extension<PgPool>,
    Extension(auth): Extension<Arc<AuthState>>,
    cookies: Cookies,
    Json(input): Json<LoginRequest>,
) -> Result<HostSession, ApiError> {
    let email = input.email.trim().to_lowercase();
    if email.is_empty() || input.password.is_empty() {
        return Err(ApiError::invalid("Email and password are required."));
    }
    let row = sqlx::query_as::<_, LoginUserRow>(
        "SELECT u.id, u.password_hash, u.display_name, wu.role,
                w.id AS workspace_id, w.name AS workspace_name
         FROM users u
         JOIN workspace_users wu ON wu.user_id = u.id
         JOIN workspaces w ON w.id = wu.workspace_id
         WHERE u.email = $1
         ORDER BY w.id
         LIMIT 1",
    )
    .bind(email)
    .fetch_optional(&pool)
    .await
    .map_err(ApiError::storage)?
    .ok_or_else(ApiError::invalid_credentials)?;
    let parsed = PasswordHash::new(&row.password_hash).map_err(ApiError::storage)?;
    Argon2::default()
        .verify_password(input.password.as_bytes(), &parsed)
        .map_err(|_| ApiError::invalid_credentials())?;
    let role = match row.role.as_str() {
        "company" => crate::types::UserRole::Company,
        "investor" => crate::types::UserRole::Investor,
        _ => return Err(ApiError::storage("Unknown workspace role.")),
    };
    let user = crate::auth::AuthenticatedUser {
        id: row.id,
        name: row.display_name,
        role,
        workspace_id: row.workspace_id,
        workspace_name: row.workspace_name,
    };
    if let Some(previous) = auth.session_id(&cookies) {
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(previous)
            .execute(&pool)
            .await
            .map_err(ApiError::storage)?;
    }
    let session_id = new_session_id();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, expires_at)
         VALUES ($1, $2, NOW() + INTERVAL '7 days')",
    )
    .bind(&session_id)
    .bind(&user.id)
    .execute(&pool)
    .await
    .map_err(ApiError::storage)?;
    auth.issue(&cookies, &session_id);
    Ok(user.session())
}

#[get("/api/auth/session")]
pub async fn get_session_handler(user: OptionalUser) -> GetSessionResponse {
    GetSessionResponse {
        session: user.0.map(|user| user.session()),
    }
}

#[post("/api/auth/logout")]
pub async fn logout_handler(
    Extension(pool): Extension<PgPool>,
    Extension(auth): Extension<Arc<AuthState>>,
    cookies: Cookies,
) -> Result<LogoutResponse, ApiError> {
    if let Some(session_id) = auth.session_id(&cookies) {
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(session_id)
            .execute(&pool)
            .await
            .map_err(ApiError::storage)?;
    }
    auth.clear(&cookies);
    Ok(LogoutResponse { ok: true })
}
