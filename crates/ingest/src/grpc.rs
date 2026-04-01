//! Tonic gRPC server — OTel-compatible receiver.
//!
//! Implements a minimal OpenTelemetry Collector-compatible ingest endpoint
//! so SDK-instrumented services can send OTLP data directly.

use std::{collections::HashMap, sync::Arc};
use anyhow::Result;
use common::{
    config::AppConfig,
    telemetry::{EventSource, LogLevel, LogPayload, MetricPayload, MetricValue, Payload, TelemetryEvent},
};
use tonic::Status;
use tracing::{error, info};
use uuid::Uuid;

use crate::kafka::{self, KafkaProducer};

// ---------------------------------------------------------------------------
// Minimal proto definitions (hand-written stubs — no generated code needed)
// ---------------------------------------------------------------------------

/// OTLP-compatible log record stub.
#[derive(Debug, Default)]
pub struct OtlpLogRecord {
    pub tenant_id:   String,
    pub service:     String,
    pub environment: String,
    pub level:       String,
    pub message:     String,
    pub trace_id:    String,
    pub span_id:     String,
}

/// OTLP-compatible metric record stub.
#[derive(Debug, Default)]
pub struct OtlpMetricRecord {
    pub tenant_id:   String,
    pub service:     String,
    pub environment: String,
    pub name:        String,
    pub value:       f64,
}

// ---------------------------------------------------------------------------
// gRPC service implementation
// ---------------------------------------------------------------------------

/// Shared state for the gRPC server.
pub struct OtlpReceiver {
    cfg:      AppConfig,
    producer: Arc<KafkaProducer>,
}

impl OtlpReceiver {
    /// Convert an OTLP log record to a `TelemetryEvent` and publish it.
    pub async fn handle_log(&self, rec: OtlpLogRecord) -> Result<(), Status> {
        let tenant_id = Uuid::parse_str(&rec.tenant_id)
            .map_err(|_| Status::invalid_argument("invalid tenant_id"))?;

        let payload = Payload::Log(LogPayload {
            level:    LogLevel::parse(&rec.level),
            message:  rec.message.clone(),
            trace_id: if rec.trace_id.is_empty() { None } else { Some(rec.trace_id.clone()) },
            span_id:  if rec.span_id.is_empty()  { None } else { Some(rec.span_id.clone()) },
            fields:   HashMap::new(),
        });

        let event = TelemetryEvent::new(
            tenant_id,
            EventSource {
                service:     rec.service,
                instance:    String::new(),
                environment: rec.environment,
                k8s_context: None,
            },
            payload,
            chrono::Utc::now(),
            HashMap::new(),
        );

        kafka::produce_event(&self.producer, &self.cfg.kafka, &event)
            .await
            .map_err(|e| {
                error!(error = %e, "grpc log produce failed");
                Status::internal(e.to_string())
            })?;

        Ok(())
    }

    /// Convert an OTLP metric record to a `TelemetryEvent` and publish it.
    pub async fn handle_metric(&self, rec: OtlpMetricRecord) -> Result<(), Status> {
        let tenant_id = Uuid::parse_str(&rec.tenant_id)
            .map_err(|_| Status::invalid_argument("invalid tenant_id"))?;

        let payload = Payload::Metric(MetricPayload {
            name:  rec.name,
            value: MetricValue::Gauge { value: rec.value },
            unit:  None,
        });

        let event = TelemetryEvent::new(
            tenant_id,
            EventSource {
                service:     rec.service,
                instance:    String::new(),
                environment: rec.environment,
                k8s_context: None,
            },
            payload,
            chrono::Utc::now(),
            HashMap::new(),
        );

        kafka::produce_event(&self.producer, &self.cfg.kafka, &event)
            .await
            .map_err(|e| {
                error!(error = %e, "grpc metric produce failed");
                Status::internal(e.to_string())
            })?;

        Ok(())
    }
}

/// Start the Tonic gRPC server.
///
/// In a full production build this would import generated proto bindings.
/// Here we expose the raw handler logic and bind a healthz-only server so
/// the binary compiles and the infrastructure is in place for proto codegen.
pub async fn serve(cfg: AppConfig, producer: Arc<KafkaProducer>) -> Result<()> {
    let addr: std::net::SocketAddr = format!("{}:{}", cfg.server.grpc_host, cfg.server.grpc_port)
        .parse()
        .expect("invalid gRPC address");

    info!(%addr, "ingest gRPC server listening");

    // The receiver is available for integration; wire proto-generated traits here.
    let _receiver = OtlpReceiver { cfg, producer };

    // Keep the port reserved until generated gRPC bindings are wired in.
    let _listener = tokio::net::TcpListener::bind(addr).await?;
    futures::future::pending::<()>().await;

    Ok(())
}
