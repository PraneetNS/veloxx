//! Kafka consumer for the storage service.
//!
//! Consumes logs, metrics, and traces from their respective topics,
//! writes to ClickHouse, and upserts log vectors into Qdrant.
//!
//! Handles consumer group rebalance and retries with exponential backoff.

use anyhow::Result;
use common::{config::AppConfig, telemetry::TelemetryEvent};
use rdkafka::{
    config::ClientConfig,
    consumer::{CommitMode, Consumer, StreamConsumer},
    error::KafkaError,
    Message,
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::{clickhouse::ClickHouseWriter, qdrant::QdrantWriter};

const MAX_RETRY_DELAY_SECS: u64 = 60;

/// Build a `StreamConsumer` subscribed to the given topics.
fn build_consumer(cfg: &AppConfig, group_suffix: &str) -> Result<StreamConsumer> {
    let group_id = format!("{}-{}", cfg.kafka.group_id_prefix, group_suffix);

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.kafka.brokers)
        .set("group.id", &group_id)
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .set("session.timeout.ms", "30000")
        .set("max.poll.interval.ms", "300000")
        .create()?;

    Ok(consumer)
}

/// Main consumer loop.  Runs until the process exits.
pub async fn run(cfg: AppConfig) -> Result<()> {
    let cfg = Arc::new(cfg);

    let ch_writer = Arc::new(
        ClickHouseWriter::new(cfg.clickhouse.clone()).await?,
    );

    // Spawn the periodic flush task.
    ch_writer.clone().spawn_flush_task();

    let qdrant_writer = Arc::new(QdrantWriter::new(&cfg).await?);

    // Collect all relevant topics.
    let topics = [
        cfg.kafka.topic_logs.as_str(),
        cfg.kafka.topic_metrics.as_str(),
        cfg.kafka.topic_traces.as_str(),
    ];

    let consumer = build_consumer(&cfg, "storage")?;
    consumer.subscribe(&topics)?;

    info!(?topics, "storage consumer subscribed");

    let mut retry_delay = Duration::from_secs(1);

    loop {
        match consumer.recv().await {
            Ok(msg) => {
                // Reset backoff on successful receive.
                retry_delay = Duration::from_secs(1);

                let payload = match msg.payload() {
                    Some(p) => p,
                    None    => { warn!("empty kafka message"); continue; }
                };

                match serde_json::from_slice::<TelemetryEvent>(payload) {
                    Ok(event) => {
                        let ch  = ch_writer.clone();
                        let qd  = qdrant_writer.clone();

                        tokio::spawn(async move {
                            if let Err(e) = ch.buffer(&event).await {
                                error!(error = %e, "clickhouse buffer error");
                            }
                            if let Err(e) = qd.upsert_event(&event).await {
                                // Non-fatal — log and continue.
                                warn!(error = %e, "qdrant upsert error");
                            }
                        });

                        // Async commit after spawning the writes.
                        if let Err(e) = consumer.commit_message(&msg, CommitMode::Async) {
                            warn!(error = %e, "kafka commit failed");
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "deserialize TelemetryEvent failed — skipping");
                    }
                }
            }
            Err(KafkaError::PartitionEOF(_)) => {
                debug!("partition EOF — waiting for more messages");
            }
            Err(e) => {
                error!(error = %e, delay_secs = retry_delay.as_secs(), "kafka receive error");
                sleep(retry_delay).await;
                retry_delay = (retry_delay * 2).min(Duration::from_secs(MAX_RETRY_DELAY_SECS));
            }
        }
    }
}
