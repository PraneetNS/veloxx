//! Kafka consumer for the alerting service.
//!
//! Watches the anomaly topic published by ai-engine and evaluates alert rules.

use anyhow::Result;
use common::config::AppConfig;
use rdkafka::{
    config::ClientConfig,
    consumer::{CommitMode, Consumer, StreamConsumer},
    error::KafkaError,
    Message,
};
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::{
    dedup::DedupWindow,
    fanout::Fanout,
    incident::IncidentStore,
    rules::RuleEngine,
};

const MAX_RETRY_DELAY_SECS: u64 = 60;

/// Main consumer loop.
pub async fn run(cfg: AppConfig, pool: PgPool) -> Result<()> {
    let cfg      = Arc::new(cfg);
    let pool     = Arc::new(pool);
    let dedup    = Arc::new(DedupWindow::new());
    let fanout   = Arc::new(Fanout::new());
    let incident = Arc::new(IncidentStore::new(pool.clone()));
    let engine   = Arc::new(RuleEngine::new(pool.clone()));

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.kafka.brokers)
        .set("group.id", format!("{}-alerting", cfg.kafka.group_id_prefix))
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .set("session.timeout.ms", "30000")
        .create()?;

    consumer.subscribe(&[&cfg.kafka.topic_anomalies])?;
    info!(topic = cfg.kafka.topic_anomalies, "alerting consumer subscribed");

    let mut retry_delay = Duration::from_secs(1);

    loop {
        match consumer.recv().await {
            Ok(msg) => {
                retry_delay = Duration::from_secs(1);

                let payload = match msg.payload() {
                    Some(p) => p,
                    None    => { warn!("empty anomaly message"); continue; }
                };

                let anomaly: serde_json::Value = match serde_json::from_slice(payload) {
                    Ok(v) => v,
                    Err(e) => { error!(error = %e, "anomaly parse error"); continue; }
                };

                let dedup_c   = dedup.clone();
                let fanout_c  = fanout.clone();
                let incident_c = incident.clone();
                let engine_c  = engine.clone();

                tokio::spawn(async move {
                    if let Err(e) = engine_c.evaluate(&anomaly, dedup_c, fanout_c, incident_c).await {
                        error!(error = %e, "rule evaluation error");
                    }
                });

                let _ = consumer.commit_message(&msg, CommitMode::Async);
            }
            Err(KafkaError::PartitionEOF(_)) => debug!("anomaly topic EOF"),
            Err(e) => {
                error!(error = %e, "alerting consumer error");
                sleep(retry_delay).await;
                retry_delay = (retry_delay * 2).min(Duration::from_secs(MAX_RETRY_DELAY_SECS));
            }
        }
    }
}
