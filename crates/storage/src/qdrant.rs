//! Qdrant upsert for semantic vector storage.
//!
//! Log messages are embedded via the `ai-engine` `/embed` endpoint and the
//! resulting 384-dim vectors are upserted into the configured Qdrant collection.

use anyhow::{Context, Result};
use common::{config::AppConfig, telemetry::{Payload, TelemetryEvent}};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// ai-engine embed response
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct EmbedRequest {
    text: String,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    vector: Vec<f32>,
}

// ---------------------------------------------------------------------------
// Qdrant REST types (simplified)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct UpsertRequest {
    points: Vec<QdrantPoint>,
}

#[derive(Debug, Serialize)]
struct QdrantPoint {
    id:      String,
    vector:  Vec<f32>,
    payload: serde_json::Value,
}

// ---------------------------------------------------------------------------
// QdrantWriter
// ---------------------------------------------------------------------------

/// Qdrant upsert client.
pub struct QdrantWriter {
    http:           Client,
    qdrant_base:    String,
    collection:     String,
    ai_engine_url:  String,
}

impl QdrantWriter {
    /// Create a new writer, ensuring the collection exists.
    pub async fn new(cfg: &AppConfig) -> Result<Self> {
        let http        = Client::new();
        let qdrant_base = format!("http://{}:{}", cfg.qdrant.host, cfg.qdrant.port);
        let collection  = cfg.qdrant.collection.clone();
        let vector_size = cfg.qdrant.vector_size;

        // Create collection if needed.
        let create_url  = format!("{qdrant_base}/collections/{collection}");
        let body = serde_json::json!({
            "vectors": { "size": vector_size, "distance": "Cosine" }
        });

        let resp = http.put(&create_url).json(&body).send().await
            .context("qdrant create collection")?;

        if resp.status().is_success() || resp.status().as_u16() == 409 {
            info!(collection, "qdrant collection ready");
        } else {
            let status = resp.status();
            let text   = resp.text().await.unwrap_or_default();
            error!(status = %status, body = text, "qdrant collection create failed");
        }

        Ok(Self {
            http,
            qdrant_base,
            collection,
            ai_engine_url: cfg.ai.engine_url.clone(),
        })
    }

    /// Embed a log message and upsert it into Qdrant.
    ///
    /// No-ops for non-log payloads.
    pub async fn upsert_event(&self, event: &TelemetryEvent) -> Result<()> {
        let log = match &event.payload {
            Payload::Log(l) => l,
            _               => return Ok(()),
        };

        // 1. Embed via ai-engine
        let embed_url = format!("{}/embed", self.ai_engine_url);
        let embed_resp: EmbedResponse = self
            .http
            .post(&embed_url)
            .json(&EmbedRequest { text: log.message.clone() })
            .send()
            .await
            .context("ai-engine embed request")?
            .json()
            .await
            .context("ai-engine embed response parse")?;

        // 2. Upsert into Qdrant
        let point = QdrantPoint {
            id:     event.id.to_string(),
            vector: embed_resp.vector,
            payload: serde_json::json!({
                "tenant_id":   event.tenant_id,
                "service":     event.source.service,
                "environment": event.source.environment,
                "level":       log.level.as_str(),
                "message":     log.message,
                "timestamp":   event.timestamp.timestamp(),
            }),
        };

        let upsert_url = format!("{}/collections/{}/points", self.qdrant_base, self.collection);
        self.http
            .put(&upsert_url)
            .json(&UpsertRequest { points: vec![point] })
            .send()
            .await
            .context("qdrant upsert")?;

        debug!(event_id = %event.id, "qdrant upsert ok");
        Ok(())
    }
}
