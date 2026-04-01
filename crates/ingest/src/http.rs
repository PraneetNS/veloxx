//! Axum HTTP server for the ingest service.
//!
//! Endpoints:
//!   - `POST /v1/logs`    — accepts JSON / logfmt / plain-text log lines
//!   - `POST /v1/metrics` — accepts Prometheus exposition format
//!   - `GET  /healthz`    — liveness probe

use std::{collections::HashMap, sync::Arc};
use axum::{
    extract::{Extension, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use common::{
    config::AppConfig,
    telemetry::{EventSource, MetricPayload, MetricValue, Payload, TelemetryEvent},
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;

use crate::{kafka::{self, KafkaProducer}, parser, rate_limiter::RateLimiter};

/// Shared application state for the HTTP server.
#[derive(Clone)]
pub struct IngestState {
    pub cfg:          AppConfig,
    pub producer:     Arc<KafkaProducer>,
    pub rate_limiter: RateLimiter,
}

/// Start the Axum HTTP server.
pub async fn serve(cfg: AppConfig, producer: Arc<KafkaProducer>) -> anyhow::Result<()> {
    let addr = format!("{}:{}", cfg.server.http_host, cfg.server.http_port);

    let state = IngestState {
        rate_limiter: RateLimiter::new(1000),
        cfg,
        producer,
    };

    let app = Router::new()
        .route("/v1/logs",    post(ingest_logs))
        .route("/v1/metrics", post(ingest_metrics))
        .route("/healthz",    get(healthz))
        .route("/readyz",     get(readyz))
        .with_state(state)
        .layer(
            tower_http::trace::TraceLayer::new_for_http(),
        );

    info!(addr, "ingest HTTP server listening");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Body for `POST /v1/logs`
#[derive(Debug, Deserialize)]
pub struct IngestLogRequest {
    /// Tenant UUID — required for routing.
    pub tenant_id: Uuid,

    /// Service that emitted the log.
    pub service: String,

    /// Optional instance / pod identifier.
    #[serde(default)]
    pub instance: String,

    /// Environment label.
    #[serde(default = "default_env")]
    pub environment: String,

    /// Raw log lines (one or more).
    pub lines: Vec<String>,
}

fn default_env() -> String {
    "production".to_string()
}

/// Body for `POST /v1/metrics`
#[derive(Debug, Deserialize)]
pub struct IngestMetricsRequest {
    pub tenant_id:    Uuid,
    pub service:      String,
    #[serde(default)]
    pub instance:     String,
    #[serde(default = "default_env")]
    pub environment:  String,
    /// Prometheus exposition format body.
    pub exposition:   String,
}

#[derive(Debug, Serialize)]
struct ApiError {
    error: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Ingest one or more log lines.
async fn ingest_logs(
    State(state): State<IngestState>,
    Json(req): Json<IngestLogRequest>,
) -> impl IntoResponse {
    if !state.rate_limiter.check(req.tenant_id) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiError { error: "rate limit exceeded".into() }),
        ).into_response();
    }

    let source = EventSource {
        service:     req.service.clone(),
        instance:    req.instance.clone(),
        environment: req.environment.clone(),
        k8s_context: None,
    };

    for line in &req.lines {
        let log_payload = parser::parse_log(line);
        let event = TelemetryEvent::new(
            req.tenant_id,
            source.clone(),
            Payload::Log(log_payload),
            chrono::Utc::now(),
            HashMap::new(),
        );

        if let Err(e) = kafka::produce_event(&state.producer, &state.cfg.kafka, &event).await {
            error!(error = %e, "failed to produce log event");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError { error: e.to_string() }),
            ).into_response();
        }
    }

    (StatusCode::ACCEPTED, Json(serde_json::json!({"accepted": req.lines.len()}))).into_response()
}

/// Ingest Prometheus-format metrics.
async fn ingest_metrics(
    State(state): State<IngestState>,
    Json(req): Json<IngestMetricsRequest>,
) -> impl IntoResponse {
    if !state.rate_limiter.check(req.tenant_id) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiError { error: "rate limit exceeded".into() }),
        ).into_response();
    }

    let samples  = parser::parse_prometheus(&req.exposition);
    let source   = EventSource {
        service:     req.service.clone(),
        instance:    req.instance.clone(),
        environment: req.environment.clone(),
        k8s_context: None,
    };
    let mut count = 0usize;

    for sample in &samples {
        let payload = MetricPayload {
            name:  sample.name.clone(),
            value: MetricValue::Gauge { value: sample.value },
            unit:  None,
        };
        let labels: HashMap<String, String> = sample.labels.clone();
        let event = TelemetryEvent::new(
            req.tenant_id,
            source.clone(),
            Payload::Metric(payload),
            chrono::Utc::now(),
            labels,
        );

        if let Err(e) = kafka::produce_event(&state.producer, &state.cfg.kafka, &event).await {
            error!(error = %e, "failed to produce metric event");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError { error: e.to_string() }),
            ).into_response();
        }
        count += 1;
    }

    (StatusCode::ACCEPTED, Json(serde_json::json!({"accepted": count}))).into_response()
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok", "service": "ingest"})))
}

async fn readyz() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ready"})))
}
