//! Ingest service entry-point.
//!
//! Starts two listeners concurrently:
//!   - Axum HTTP server on `:4317` (REST ingest + healthz)
//!   - Tonic gRPC server on `:4318` (OTel-compatible receiver)

use anyhow::Result;
use common::config::AppConfig;
use tracing::info;

mod grpc;
mod http;
mod kafka;
mod parser;
mod rate_limiter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise structured JSON logging.
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!(service = "ingest", "starting Veloxx ingest service");

    let cfg = AppConfig::load()?;

    // Build a shared Kafka producer.
    let producer = kafka::KafkaProducer::new(&cfg.kafka.brokers).await?;
    let producer = std::sync::Arc::new(producer);

    // Concurrently run HTTP and gRPC servers.
    let http_handle = http::serve(cfg.clone(), producer.clone());
    let grpc_handle = grpc::run(cfg.clone());

    tokio::try_join!(http_handle, grpc_handle)?;

    Ok(())
}
