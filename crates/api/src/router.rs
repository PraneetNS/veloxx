//! Axum router construction.

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use clickhouse::Client;
use common::config::AppConfig;
use sqlx::PgPool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    handlers::{self, ApiState},
    middleware::auth_middleware,
};

/// Build the full Axum router.
pub async fn build(cfg: AppConfig, db: PgPool) -> anyhow::Result<Router> {
    let ch_client = Client::default()
        .with_url(&cfg.clickhouse.url)
        .with_database(&cfg.clickhouse.database)
        .with_user(&cfg.clickhouse.username)
        .with_password(&cfg.clickhouse.password);

    let state = ApiState { cfg: cfg.clone(), db, ch_client };

    // Protected tenant routes — all require a valid JWT with matching tenant_id.
    let tenant_routes = Router::new()
        .route("/logs",      get(handlers::get_logs))
        .route("/metrics",   get(handlers::get_metrics))
        .route("/search",    get(handlers::semantic_search))
        .route("/anomalies", get(handlers::get_anomalies))
        .route("/ask",       post(handlers::ask_ai))
        .route("/live",      get(handlers::live_stream))
        .layer(middleware::from_fn_with_state(cfg.clone(), auth_middleware));

    let app = Router::new()
        // Auth routes (no auth middleware).
        .route("/api/v1/auth/login",   post(handlers::login))
        // Health probes (no auth).
        .route("/healthz",             get(handlers::healthz))
        .route("/readyz",              get(handlers::readyz))
        // Tenant-scoped routes.
        .nest("/api/v1/:tenant_id", tenant_routes)
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    Ok(app)
}
