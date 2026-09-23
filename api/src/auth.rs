pub mod models;

use std::sync::Arc;

use axum::{
    Router,
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use sqlx::PgPool;
use tower_cookies::{
    Cookie, CookieManagerLayer, Cookies, Key,
    cookie::{SameSite, time::Duration as CookieDuration},
};

use crate::{
    error::ApiError,
    types::{HostSession, UserRole, Viewer, Workspace},
};
use models::SessionUserRow;

const SESSION_COOKIE: &str = "dr_session";
const SESSION_MAX_AGE_DAYS: i64 = 7;
const DEVELOPMENT_SIGNING_KEY: &str =
    "dataroom-interview-local-session-signing-key-change-this-before-deployment";

#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub id: String,
    pub name: String,
    pub role: UserRole,
    pub workspace_id: String,
    pub workspace_name: String,
}

impl AuthenticatedUser {
    pub fn session(&self) -> HostSession {
        HostSession {
            user: Viewer {
                id: self.id.clone(),
                name: self.name.clone(),
                role: self.role.clone(),
            },
            workspace: Workspace {
                id: self.workspace_id.clone(),
                name: self.workspace_name.clone(),
            },
        }
    }
}

pub struct CurrentUser(pub AuthenticatedUser);
pub struct OptionalUser(pub Option<AuthenticatedUser>);

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .map(Self)
            .ok_or_else(ApiError::unauthorized)
    }
}

impl<S: Send + Sync> FromRequestParts<S> for OptionalUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(parts.extensions.get::<AuthenticatedUser>().cloned()))
    }
}

pub struct AuthState {
    pool: PgPool,
    key: Key,
}

impl AuthState {
    pub fn new(pool: PgPool) -> Self {
        let signing_key = std::env::var("SESSION_SIGNING_KEY")
            .unwrap_or_else(|_| DEVELOPMENT_SIGNING_KEY.to_string());
        assert!(
            signing_key.len() >= 64,
            "SESSION_SIGNING_KEY must contain at least 64 bytes."
        );
        Self {
            pool,
            key: Key::from(signing_key.as_bytes()),
        }
    }

    pub fn session_id(&self, cookies: &Cookies) -> Option<String> {
        cookies
            .signed(&self.key)
            .get(SESSION_COOKIE)
            .map(|cookie| cookie.value().to_string())
            .filter(|id| !id.is_empty())
    }

    pub fn issue(&self, cookies: &Cookies, session_id: &str) {
        let cookie = Cookie::build((SESSION_COOKIE, session_id.to_string()))
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .secure(false)
            .max_age(CookieDuration::days(SESSION_MAX_AGE_DAYS))
            .build();
        cookies.signed(&self.key).add(cookie);
    }

    pub fn clear(&self, cookies: &Cookies) {
        let cookie = Cookie::build((SESSION_COOKIE, ""))
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .secure(false)
            .build();
        cookies.signed(&self.key).remove(cookie);
    }

    async fn resolve(&self, session_id: &str) -> Result<Option<AuthenticatedUser>, sqlx::Error> {
        let row = sqlx::query_as::<_, SessionUserRow>(
            "SELECT u.id, u.display_name, wu.role, w.id AS workspace_id, w.name AS workspace_name
             FROM sessions s
             JOIN users u ON u.id = s.user_id
             JOIN workspace_users wu ON wu.user_id = u.id
             JOIN workspaces w ON w.id = wu.workspace_id
             WHERE s.id = $1 AND s.expires_at > NOW()
             ORDER BY w.id
             LIMIT 1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let role = match row.role.as_str() {
            "company" => UserRole::Company,
            "investor" => UserRole::Investor,
            _ => return Ok(None),
        };
        Ok(Some(AuthenticatedUser {
            id: row.id,
            name: row.display_name,
            role,
            workspace_id: row.workspace_id,
            workspace_name: row.workspace_name,
        }))
    }
}

pub fn new_session_id() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

async fn resolve_session(
    State(state): State<Arc<AuthState>>,
    cookies: Cookies,
    mut request: Request,
    next: Next,
) -> Response {
    if let Some(session_id) = state.session_id(&cookies) {
        match state.resolve(&session_id).await {
            Ok(Some(user)) => {
                request.extensions_mut().insert(user);
            }
            Ok(None) => {}
            Err(error) => eprintln!("Session resolution failed: {error}"),
        }
    }
    next.run(request).await
}

pub fn session_layer(router: Router, pool: PgPool) -> Router {
    let state = Arc::new(AuthState::new(pool));
    router
        .layer(axum::Extension(state.clone()))
        .layer(axum::middleware::from_fn_with_state(state, resolve_session))
        .layer(CookieManagerLayer::new())
}
