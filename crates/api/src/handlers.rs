//! Request handlers for the Veloxx REST API.

use axum::{
    extract::{Extension, Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use clickhouse::Client;
use common::config::AppConfig;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::error;
use uuid::Uuid;

use crate::{
    auth::generate_token,
    middleware::AuthClaims,
    ws::handle_ws,
};

/// Shared API state.
#[derive(Clone)]
pub struct ApiState {
    pub cfg:        AppConfig,
    pub db:         PgPool,
    pub ch_client:  Client,
}

// ---------------------------------------------------------------------------
// Auth handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email:    String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token:  String,
    pub refresh_token: String,
    pub tenant_id:     Uuid,
    pub user_id:       Uuid,
}

#[derive(Debug, sqlx::FromRow)]
struct LoginRow {
    id: Uuid,
    tenant_id: Uuid,
    password_hash: String,
    role: String,
    is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, clickhouse::Row)]
struct ApiLogRow {
    tenant_id: String,
    timestamp: u32,
    level: String,
    message: String,
    service: String,
    trace_id: String,
    labels: String,
}

#[derive(Debug, Serialize, Deserialize, clickhouse::Row)]
struct ApiMetricRow {
    tenant_id: String,
    timestamp: u32,
    name: String,
    value: f64,
    service: String,
    labels: String,
}

#[derive(Debug, sqlx::FromRow)]
struct IncidentRow {
    id: Uuid,
    title: String,
    description: Option<String>,
    severity: String,
    status: String,
    context: serde_json::Value,
    opened_at: DateTime<Utc>,
}

/// `POST /api/v1/auth/login`
pub async fn login(
    State(state): State<ApiState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    // Look up the user.
    let row = sqlx::query_as::<_, LoginRow>(
        r#"SELECT u.id, u.tenant_id, u.password_hash, u.role, u.is_active
           FROM users u WHERE u.email = $1"#
    )
    .bind(&req.email)
    .fetch_optional(&state.db)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None)    => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"invalid credentials"}))).into_response(),
        Err(e)      => {
            error!(error = %e, "db error during login");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"internal error"}))).into_response();
        }
    };

    if !row.is_active {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"account disabled"}))).into_response();
    }

    // Verify password.
    let matches = bcrypt::verify(&req.password, &row.password_hash)
        .unwrap_or(false);

    if !matches {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"invalid credentials"}))).into_response();
    }

    let access_token = match generate_token(
        row.id,
        row.tenant_id,
        &req.email,
        &row.role,
        &state.cfg.auth.jwt_secret,
        state.cfg.auth.jwt_expiry_secs,
    ) {
        Ok(t)  => t,
        Err(e) => {
            error!(error = %e, "token generation failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"internal error"}))).into_response();
        }
    };

    // Simple opaque refresh token (UUID, stored/validated separately in prod).
    let refresh_token = Uuid::new_v4().to_string();

    (StatusCode::OK, Json(TokenResponse {
        access_token,
        refresh_token,
        tenant_id: row.tenant_id,
        user_id:   row.id,
    })).into_response()
}

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub q:       Option<String>,
    pub from:    Option<DateTime<Utc>>,
    pub to:      Option<DateTime<Utc>>,
    pub level:   Option<String>,
    pub service: Option<String>,
    pub limit:   Option<u32>,
}

/// `GET /api/v1/:tenant_id/logs`
pub async fn get_logs(
    State(state): State<ApiState>,
    Path(tenant_id): Path<Uuid>,
    Query(q): Query<LogQuery>,
    Extension(AuthClaims(_claims)): Extension<AuthClaims>,
) -> impl IntoResponse {
    let limit  = q.limit.unwrap_or(100).min(1000);
    let from   = q.from.map(|d| d.timestamp()).unwrap_or(0);
    let to     = q.to.map(|d| d.timestamp()).unwrap_or(i64::MAX);
    let level  = q.level.clone().unwrap_or_default();
    let service = q.service.clone().unwrap_or_default();
    let search = q.q.clone().unwrap_or_default();

    // Build a dynamic ClickHouse query.
    let mut sql = format!(
        "SELECT tenant_id, timestamp, level, message, service, trace_id, labels \
         FROM logs \
         WHERE tenant_id = '{}' \
           AND timestamp >= fromUnixTimestamp({}) \
           AND timestamp <= fromUnixTimestamp({})",
        tenant_id, from, to
    );

    if !level.is_empty() {
        sql.push_str(&format!(" AND level = '{}'", level.replace('\'', "''")));
    }
    if !service.is_empty() {
        sql.push_str(&format!(" AND service = '{}'", service.replace('\'', "''")));
    }
    if !search.is_empty() {
        sql.push_str(&format!(" AND positionCaseInsensitive(message, '{}') > 0", search.replace('\'', "''")));
    }
    sql.push_str(&format!(" ORDER BY timestamp DESC LIMIT {}", limit));

    let rows = state
        .ch_client
        .query(&sql)
        .fetch_all::<ApiLogRow>()
        .await;

    match rows {
        Ok(data) => (StatusCode::OK, Json(serde_json::json!({"data": data}))).into_response(),
        Err(e)   => {
            error!(error = %e, "clickhouse logs query failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct MetricQuery {
    pub name: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to:   Option<DateTime<Utc>>,
    pub limit: Option<u32>,
}

/// `GET /api/v1/:tenant_id/metrics`
pub async fn get_metrics(
    State(state): State<ApiState>,
    Path(tenant_id): Path<Uuid>,
    Query(q): Query<MetricQuery>,
    Extension(AuthClaims(_claims)): Extension<AuthClaims>,
) -> impl IntoResponse {
    let limit   = q.limit.unwrap_or(500).min(5000);
    let from    = q.from.map(|d| d.timestamp()).unwrap_or(0);
    let to      = q.to.map(|d| d.timestamp()).unwrap_or(i64::MAX);
    let name    = q.name.clone().unwrap_or_default();

    let mut sql = format!(
        "SELECT tenant_id, timestamp, name, value, service, labels \
         FROM metrics \
         WHERE tenant_id = '{}' \
           AND timestamp >= fromUnixTimestamp({}) \
           AND timestamp <= fromUnixTimestamp({})",
        tenant_id, from, to
    );

    if !name.is_empty() {
        sql.push_str(&format!(" AND name = '{}'", name.replace('\'', "''")));
    }
    sql.push_str(&format!(" ORDER BY timestamp DESC LIMIT {}", limit));

    match state.ch_client.query(&sql).fetch_all::<ApiMetricRow>().await {
        Ok(data) => (StatusCode::OK, Json(serde_json::json!({"data": data}))).into_response(),
        Err(e)   => {
            error!(error = %e, "clickhouse metrics query failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Semantic search
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q:     String,
    pub limit: Option<u32>,
}

/// `GET /api/v1/:tenant_id/search` — semantic vector search via Qdrant.
pub async fn semantic_search(
    State(state): State<ApiState>,
    Path(tenant_id): Path<Uuid>,
    Query(q): Query<SearchQuery>,
    Extension(AuthClaims(_claims)): Extension<AuthClaims>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(10).min(100);

    // 1. Embed the query string via ai-engine.
    let embed_url = format!("{}/embed", state.cfg.ai.engine_url);
    let embed_resp = reqwest::Client::new()
        .post(&embed_url)
        .json(&serde_json::json!({"text": q.q}))
        .send()
        .await;

    let vector: Vec<f32> = match embed_resp {
        Ok(r)  => match r.json::<serde_json::Value>().await {
            Ok(v) => serde_json::from_value(v["vector"].clone()).unwrap_or_default(),
            Err(e) => return (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
        },
        Err(e) => return (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    // 2. Query Qdrant.
    let qdrant_url = format!(
        "http://{}:{}/collections/{}/points/search",
        state.cfg.qdrant.host, state.cfg.qdrant.port, state.cfg.qdrant.collection
    );

    let body = serde_json::json!({
        "vector":       vector,
        "limit":        limit,
        "with_payload": true,
        "filter": {
            "must": [{
                "key":   "tenant_id",
                "match": {"value": tenant_id.to_string()}
            }]
        }
    });

    match reqwest::Client::new().post(&qdrant_url).json(&body).send().await {
        Ok(r)  => {
            let data = r.json::<serde_json::Value>().await.unwrap_or_default();
            (StatusCode::OK, Json(data)).into_response()
        },
        Err(e) => (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Anomalies
// ---------------------------------------------------------------------------

/// `GET /api/v1/:tenant_id/anomalies`
pub async fn get_anomalies(
    State(state): State<ApiState>,
    Path(tenant_id): Path<Uuid>,
    Extension(AuthClaims(_claims)): Extension<AuthClaims>,
) -> impl IntoResponse {
    // Fetch recent open/acknowledged incidents from Postgres.
    let rows = sqlx::query_as::<_, IncidentRow>(
        r#"SELECT id, title, description, severity::text as severity, status::text as status,
                  context, opened_at
           FROM incidents
           WHERE tenant_id = $1
             AND status != 'resolved'
           ORDER BY opened_at DESC
           LIMIT 50"#
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(incidents) => {
            let data: Vec<serde_json::Value> = incidents
                .into_iter()
                .map(|r| serde_json::json!({
                    "id":          r.id,
                    "title":       r.title,
                    "description": r.description,
                    "severity":    r.severity,
                    "status":      r.status,
                    "context":     r.context,
                    "opened_at":   r.opened_at,
                }))
                .collect();
            (StatusCode::OK, Json(serde_json::json!({"data": data}))).into_response()
        },
        Err(e) => {
            error!(error = %e, "anomalies query failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// AI Ask
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AskRequest {
    pub question: String,
}

/// `POST /api/v1/:tenant_id/ask` — LLM explain endpoint.
pub async fn ask_ai(
    State(state): State<ApiState>,
    Path(tenant_id): Path<Uuid>,
    Extension(AuthClaims(_claims)): Extension<AuthClaims>,
    Json(req): Json<AskRequest>,
) -> impl IntoResponse {
    let explain_url = format!("{}/explain", state.cfg.ai.engine_url);

    let body = serde_json::json!({
        "tenant_id": tenant_id,
        "question":  req.question,
    });

    match reqwest::Client::new().post(&explain_url).json(&body).send().await {
        Ok(r)  => {
            let text = r.text().await.unwrap_or_default();
            (StatusCode::OK, Json(serde_json::json!({"answer": text}))).into_response()
        },
        Err(e) => (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

// ---------------------------------------------------------------------------
// WebSocket live stream
// ---------------------------------------------------------------------------

/// `WS /api/v1/:tenant_id/live` — live metric stream.
pub async fn live_stream(
    State(state): State<ApiState>,
    Path(tenant_id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, tenant_id, state))
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

pub async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok", "service": "api"})))
}

pub async fn readyz(State(state): State<ApiState>) -> impl IntoResponse {
    match sqlx::query("SELECT 1").fetch_one(&state.db).await {
        Ok(_)  => (StatusCode::OK,  Json(serde_json::json!({"status": "ready"}))),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"status": "not ready"}))),
    }
}
