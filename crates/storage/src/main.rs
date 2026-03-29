//! Storage service entry-point.
//!
//! Consumes `TelemetryEvent` messages from Kafka and writes them to
//! ClickHouse (logs + metrics) and Qdrant (vector index for semantic search).
//! Also runs Postgres migrations on startup.

use anyhow::Result;
use common::config::AppConfig;
use tracing::info;

mod clickhouse;
mod consumer;
mod migrations;
mod qdrant;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!(service = "storage", "starting Veloxx storage service");

    let cfg = AppConfig::load()?;

    // Run Postgres migrations before accepting any events.
    migrations::run(&cfg.postgres.url).await?;

    // Start Kafka consumers (logs + metrics).
    consumer::run(cfg).await?;

    Ok(())
}
