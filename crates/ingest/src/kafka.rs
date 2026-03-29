//! Kafka producer for the ingest service.
//!
//! Uses snappy compression and a 5 ms linger window for batching.
//! Messages are partitioned by `tenant_id + signal_type` to ensure
//! ordering within a tenant-signal shard.

use anyhow::{Context, Result};
use common::{config::KafkaConfig, telemetry::TelemetryEvent};
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use std::time::Duration;
use tracing::{debug, error};

/// Build a new `FutureProducer` from the given Kafka config.
pub fn build_producer(cfg: &KafkaConfig) -> Result<FutureProducer> {
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.brokers)
        .set("message.timeout.ms", "5000")
        .set("compression.type", &cfg.compression)
        .set("linger.ms", cfg.linger_ms.to_string())
        .set("batch.num.messages", "1000")
        .set("acks", "all")
        .set("enable.idempotence", "true")
        .create()
        .context("failed to create Kafka producer")?;

    Ok(producer)
}

/// Produce a `TelemetryEvent` to the appropriate Kafka topic.
///
/// The partition key is `"{tenant_id}:{signal_type}"` for deterministic
/// per-tenant-per-type ordering.
pub async fn produce_event(
    producer: &FutureProducer,
    cfg: &KafkaConfig,
    event: &TelemetryEvent,
) -> Result<()> {
    let topic = match &event.payload {
        common::telemetry::Payload::Log(_)    => &cfg.topic_logs,
        common::telemetry::Payload::Metric(_) => &cfg.topic_metrics,
        common::telemetry::Payload::Trace(_)  => &cfg.topic_traces,
    };

    let signal_type = match &event.payload {
        common::telemetry::Payload::Log(_)    => "log",
        common::telemetry::Payload::Metric(_) => "metric",
        common::telemetry::Payload::Trace(_)  => "trace",
    };

    let key = format!("{}:{}", event.tenant_id, signal_type);
    let payload = serde_json::to_vec(event).context("serialize TelemetryEvent")?;

    let record = FutureRecord::to(topic)
        .key(&key)
        .payload(&payload);

    producer
        .send(record, Duration::from_secs(5))
        .await
        .map_err(|(err, _msg)| {
            error!(error = %err, topic, "kafka produce failed");
            anyhow::anyhow!("kafka produce: {err}")
        })?;

    debug!(topic, key, "event produced");
    Ok(())
}
