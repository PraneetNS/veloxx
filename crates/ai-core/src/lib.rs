//! # ai-core
//!
//! Rust client for the Veloxx `ai-engine` HTTP service.
//!
//! Provides typed wrappers around the `/detect`, `/embed`, and `/explain`
//! endpoints so other crates can call the AI service without dealing with
//! raw HTTP.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::debug;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

/// Request body for `POST /detect`.
#[derive(Debug, Serialize)]
pub struct DetectRequest {
    /// Metric name being analysed.
    pub metric_name: String,

    /// Ordered list of recent values (oldest → newest).
    pub values: Vec<f64>,

    /// Tenant context (informational).
    pub tenant_id: uuid::Uuid,
}

/// Response from `POST /detect`.
#[derive(Debug, Deserialize)]
pub struct DetectResponse {
    /// Continuous anomaly score 0.0 – 1.0.
    pub anomaly_score: f64,

    /// `true` if the engine considers this an anomaly.
    pub is_anomaly: bool,

    /// Human-readable reason (optional).
    pub reason: Option<String>,
}

/// Request body for `POST /embed`.
#[derive(Debug, Serialize)]
pub struct EmbedRequest {
    pub text: String,
}

/// Response from `POST /embed`.
#[derive(Debug, Deserialize)]
pub struct EmbedResponse {
    /// 384-dimensional float vector produced by all-MiniLM-L6-v2.
    pub vector: Vec<f32>,
}

/// Request body for `POST /explain`.
#[derive(Debug, Serialize)]
pub struct ExplainRequest {
    pub service:      String,
    pub question:     String,
    pub metric_data:  serde_json::Value,
    pub recent_logs:  Vec<String>,
}

/// Response from `POST /explain`.
#[derive(Debug, Deserialize)]
pub struct ExplainResponse {
    /// LLM-generated root-cause explanation in plain English.
    pub explanation: String,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// HTTP client for the `ai-engine` service.
#[derive(Clone)]
pub struct AiEngineClient {
    http:     reqwest::Client,
    base_url: String,
}

impl AiEngineClient {
    /// Create a new client targeting `base_url` (e.g. `"http://ai-engine:8000"`).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http:     reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// Call `/detect` to score a metric time-series for anomalies.
    pub async fn detect(&self, req: &DetectRequest) -> Result<DetectResponse> {
        let url = format!("{}/detect", self.base_url);
        debug!(url, "calling ai-engine /detect");

        let resp: DetectResponse = self
            .http
            .post(&url)
            .json(req)
            .send()
            .await
            .context("ai-engine /detect request")?
            .json()
            .await
            .context("ai-engine /detect response parse")?;

        Ok(resp)
    }

    /// Call `/embed` to get a 384-dim vector for `text`.
    pub async fn embed(&self, text: &str) -> Result<EmbedResponse> {
        let url = format!("{}/embed", self.base_url);
        debug!(url, "calling ai-engine /embed");

        let resp: EmbedResponse = self
            .http
            .post(&url)
            .json(&EmbedRequest { text: text.to_owned() })
            .send()
            .await
            .context("ai-engine /embed request")?
            .json()
            .await
            .context("ai-engine /embed response parse")?;

        Ok(resp)
    }

    /// Call `/explain` to get an LLM-generated root-cause explanation.
    pub async fn explain(&self, req: &ExplainRequest) -> Result<ExplainResponse> {
        let url = format!("{}/explain", self.base_url);
        debug!(url, "calling ai-engine /explain");

        let resp: ExplainResponse = self
            .http
            .post(&url)
            .json(req)
            .send()
            .await
            .context("ai-engine /explain request")?
            .json()
            .await
            .context("ai-engine /explain response parse")?;

        Ok(resp)
    }

    /// Health-check the ai-engine service.
    pub async fn health(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        let status = self.http.get(&url).send().await?.status();
        Ok(status.is_success())
    }
}
