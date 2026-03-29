//! Axum middleware for JWT authentication and tenant isolation.
//!
//! Extracts the Bearer token from the `Authorization` header, validates it
//! and compares `tenant_id` in the claims against the `:tenant_id` path
//! parameter.  Rejects with 401 or 403 on any mismatch.

use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use common::config::AppConfig;
use serde_json::json;
use tracing::warn;
use uuid::Uuid;

use crate::auth::{validate_token, Claims};

/// Axum extension key for the authenticated claims.
#[derive(Debug, Clone)]
pub struct AuthClaims(pub Claims);

/// Middleware that validates the JWT and enforces tenant isolation.
pub async fn auth_middleware(
    State(cfg): State<AppConfig>,
    Path(tenant_id): Path<Uuid>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    // Extract Bearer token.
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let token = match token {
        Some(t) => t.to_owned(),
        None    => {
            warn!("missing Authorization header");
            return (StatusCode::UNAUTHORIZED, Json(json!({"error": "missing bearer token"}))).into_response();
        }
    };

    // Validate token.
    let claims = match validate_token(&token, &cfg.auth.jwt_secret) {
        Ok(c)  => c,
        Err(_) => {
            warn!("invalid JWT");
            return (StatusCode::UNAUTHORIZED, Json(json!({"error": "invalid or expired token"}))).into_response();
        }
    };

    // Enforce tenant isolation.
    let token_tenant = match claims.tenant_id.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => {
            warn!("tenant_id in token is not a valid UUID");
            return (StatusCode::UNAUTHORIZED, Json(json!({"error": "invalid tenant claim"}))).into_response();
        }
    };

    if token_tenant != tenant_id {
        warn!(
            token_tenant = %token_tenant,
            path_tenant  = %tenant_id,
            "tenant mismatch — access denied"
        );
        return (StatusCode::FORBIDDEN, Json(json!({"error": "tenant access denied"}))).into_response();
    }

    req.extensions_mut().insert(AuthClaims(claims));
    next.run(req).await
}
