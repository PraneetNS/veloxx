//! ClickHouse writer.
//!
//! Events are buffered in memory and flushed either when the batch reaches
//! `flush_batch_size` rows or every `flush_interval_secs` seconds —
//! whichever comes first.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clickhouse::{Client, Row};
use common::{
    config::ClickHouseConfig,
    telemetry::{LogPayload, MetricPayload, MetricValue, Payload, TelemetryEvent},
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::Mutex,
    time::{interval, Instant},
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

/// A row in the `logs` ClickHouse table.
#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct LogRow {
    pub tenant_id:  String,
    pub timestamp:  u32,   // Unix epoch seconds (Date32 in ClickHouse)
    pub level:      String,
    pub message:    String,
    pub service:    String,
    pub instance:   String,
    pub environment:String,
    pub trace_id:   String,
    pub labels:     String, // JSON-serialized HashMap
}

/// A row in the `metrics` ClickHouse table.
#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct MetricRow {
    pub tenant_id:   String,
    pub timestamp:   u32,
    pub name:        String,
    pub value:       f64,
    pub service:     String,
    pub environment: String,
    pub labels:      String,
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

/// Buffered ClickHouse writer.
pub struct ClickHouseWriter {
    client:       Client,
    cfg:          ClickHouseConfig,
    log_buf:      Arc<Mutex<Vec<LogRow>>>,
    metric_buf:   Arc<Mutex<Vec<MetricRow>>>,
}

impl ClickHouseWriter {
    /// Create a writer and ensure the schema exists.
    pub async fn new(cfg: ClickHouseConfig) -> Result<Self> {
        let client = Client::default()
            .with_url(&cfg.url)
            .with_database(&cfg.database)
            .with_user(&cfg.username)
            .with_password(&cfg.password);

        let writer = Self {
            cfg,
            client,
            log_buf:    Arc::new(Mutex::new(Vec::new())),
            metric_buf: Arc::new(Mutex::new(Vec::new())),
        };

        writer.ensure_schema().await?;
        Ok(writer)
    }

    /// Create tables if they do not already exist.
    async fn ensure_schema(&self) -> Result<()> {
        self.client
            .query(
                "CREATE TABLE IF NOT EXISTS logs (
                    tenant_id   String,
                    timestamp   DateTime,
                    level       LowCardinality(String),
                    message     String,
                    service     String,
                    instance    String,
                    environment LowCardinality(String),
                    trace_id    String,
                    labels      String
                )
                ENGINE = ReplicatedMergeTree('/clickhouse/tables/{shard}/logs', '{replica}')
                PARTITION BY toYYYYMM(timestamp)
                ORDER BY (tenant_id, timestamp)
                TTL timestamp + INTERVAL 30 DAY",
            )
            .execute()
            .await
            .context("create logs table")?;

        self.client
            .query(
                "CREATE TABLE IF NOT EXISTS metrics (
                    tenant_id   String,
                    timestamp   DateTime,
                    name        LowCardinality(String),
                    value       Float64,
                    service     String,
                    environment LowCardinality(String),
                    labels      String
                )
                ENGINE = ReplicatedMergeTree('/clickhouse/tables/{shard}/metrics', '{replica}')
                PARTITION BY toYYYYMM(timestamp)
                ORDER BY (tenant_id, name, timestamp)
                TTL timestamp + INTERVAL 30 DAY",
            )
            .execute()
            .await
            .context("create metrics table")?;

        info!("clickhouse schema ready");
        Ok(())
    }

    /// Buffer a `TelemetryEvent`.  Flushes automatically when batch is full.
    pub async fn buffer(&self, event: &TelemetryEvent) -> Result<()> {
        match &event.payload {
            Payload::Log(log) => {
                let row = self.to_log_row(event, log);
                let mut buf = self.log_buf.lock().await;
                buf.push(row);
                if buf.len() >= self.cfg.flush_batch_size {
                    let rows = std::mem::take(&mut *buf);
                    drop(buf);
                    self.flush_logs(rows).await?;
                }
            }
            Payload::Metric(metric) => {
                let row = self.to_metric_row(event, metric);
                let mut buf = self.metric_buf.lock().await;
                buf.push(row);
                if buf.len() >= self.cfg.flush_batch_size {
                    let rows = std::mem::take(&mut *buf);
                    drop(buf);
                    self.flush_metrics(rows).await?;
                }
            }
            Payload::Trace(_) => {
                // Traces stored in Qdrant / future Tempo-compatible store.
                debug!("trace payload — skipping ClickHouse write");
            }
        }
        Ok(())
    }

    /// Flush remaining buffers to ClickHouse.  Called on interval.
    pub async fn flush_all(&self) -> Result<()> {
        let log_rows = {
            let mut buf = self.log_buf.lock().await;
            std::mem::take(&mut *buf)
        };
        if !log_rows.is_empty() {
            self.flush_logs(log_rows).await?;
        }

        let metric_rows = {
            let mut buf = self.metric_buf.lock().await;
            std::mem::take(&mut *buf)
        };
        if !metric_rows.is_empty() {
            self.flush_metrics(metric_rows).await?;
        }

        Ok(())
    }

    async fn flush_logs(&self, rows: Vec<LogRow>) -> Result<()> {
        let n = rows.len();
        let mut insert = self.client.insert("logs")?;
        for row in rows {
            insert.write(&row).await.context("ch insert log row")?;
        }
        insert.end().await.context("ch flush logs")?;
        info!(rows = n, "flushed logs to clickhouse");
        Ok(())
    }

    async fn flush_metrics(&self, rows: Vec<MetricRow>) -> Result<()> {
        let n = rows.len();
        let mut insert = self.client.insert("metrics")?;
        for row in rows {
            insert.write(&row).await.context("ch insert metric row")?;
        }
        insert.end().await.context("ch flush metrics")?;
        info!(rows = n, "flushed metrics to clickhouse");
        Ok(())
    }

    fn to_log_row(&self, ev: &TelemetryEvent, log: &LogPayload) -> LogRow {
        LogRow {
            tenant_id:   ev.tenant_id.to_string(),
            timestamp:   ev.timestamp.timestamp() as u32,
            level:       log.level.to_string(),
            message:     log.message.clone(),
            service:     ev.source.service.clone(),
            instance:    ev.source.instance.clone(),
            environment: ev.source.environment.clone(),
            trace_id:    log.trace_id.clone().unwrap_or_default(),
            labels:      serde_json::to_string(&ev.labels).unwrap_or_default(),
        }
    }

    fn to_metric_row(&self, ev: &TelemetryEvent, metric: &MetricPayload) -> MetricRow {
        let value = match &metric.value {
            MetricValue::Gauge   { value } => *value,
            MetricValue::Counter { value } => *value,
            MetricValue::Histogram { sum, .. } => *sum,
            MetricValue::Summary   { sum, .. } => *sum,
        };
        MetricRow {
            tenant_id:   ev.tenant_id.to_string(),
            timestamp:   ev.timestamp.timestamp() as u32,
            name:        metric.name.clone(),
            value,
            service:     ev.source.service.clone(),
            environment: ev.source.environment.clone(),
            labels:      serde_json::to_string(&ev.labels).unwrap_or_default(),
        }
    }

    /// Spawn a background task that flushes the buffers on the configured
    /// interval.  Returns a handle that can be awaited to join.
    pub fn spawn_flush_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let interval_secs = self.cfg.flush_interval_secs;
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));
            loop {
                ticker.tick().await;
                if let Err(e) = self.flush_all().await {
                    error!(error = %e, "clickhouse flush error");
                }
            }
        })
    }
}
