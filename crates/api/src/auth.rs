//! JWT authentication helpers.
//!
//! Issues and validates HS256 JWTs.  The `tenant_id` claim ensures that
//! every token is tied to exactly one tenant.

use anyhow::Result;
use chrono::{Duration, Utc};
use common::errors::VeloxxError;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT claims embedded in every access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — user UUID.
    pub sub: String,

    /// Tenant UUID — must match the path parameter on every request.
    pub tenant_id: String,

    /// User email.
    pub email: String,

    /// User role.
    pub role: String,

    /// Expiry (Unix timestamp).
    pub exp: i64,

    /// Issued-at (Unix timestamp).
    pub iat: i64,
}

/// Generate an access token for the given user.
pub fn generate_token(
    user_id:   Uuid,
    tenant_id: Uuid,
    email:     &str,
    role:      &str,
    secret:    &str,
    ttl_secs:  u64,
) -> Result<String> {
    let now = Utc::now();
    let claims = Claims {
        sub:       user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        email:     email.to_owned(),
        role:      role.to_owned(),
        iat:       now.timestamp(),
        exp:       (now + Duration::seconds(ttl_secs as i64)).timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

/// Validate and decode a Bearer token.
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, VeloxxError> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| VeloxxError::InvalidToken)?;

    Ok(data.claims)
}
