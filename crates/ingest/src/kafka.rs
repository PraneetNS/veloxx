//! Kafka producer for the ingest service.
//!
//! Local development currently runs in stub mode so the workspace can build
//! without native Kafka toolchain dependencies on Windows.

use anyhow::Result;
use common::{config::KafkaConfig, telemetry::TelemetryEvent};
use tracing::debug;

#[derive(Debug, Default)]
pub struct KafkaProducer;

/// Build a new stub producer from the given Kafka config.
pub fn build_producer(_cfg: &KafkaConfig) -> Result<KafkaProducer> {
    Ok(KafkaProducer)
}

/// Accept a `TelemetryEvent` without forwarding it to Kafka.
pub async fn produce_event(
    _producer: &KafkaProducer,
    cfg: &KafkaConfig,
    event: &TelemetryEvent,
) -> Result<()> {
    let topic = match &event.payload {
        common::telemetry::Payload::Log(_) => &cfg.topic_logs,
        common::telemetry::Payload::Metric(_) => &cfg.topic_metrics,
        common::telemetry::Payload::Trace(_) => &cfg.topic_traces,
    };

    let signal_type = match &event.payload {
        common::telemetry::Payload::Log(_) => "log",
        common::telemetry::Payload::Metric(_) => "metric",
        common::telemetry::Payload::Trace(_) => "trace",
    };

    let key = format!("{}:{}", event.tenant_id, signal_type);
    debug!(topic, key, "event accepted by stub Kafka producer");
    Ok(())
}
