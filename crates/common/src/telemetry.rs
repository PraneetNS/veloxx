//! Telemetry event types — the core data model for all signals.

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The top-level container for every signal (log / metric / trace).
///
/// Every query MUST be scoped by `tenant_id` to enforce multi-tenancy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Unique event identifier (UUIDv4).
    pub id: Uuid,

    /// Owning tenant — all reads and writes must match this.
    pub tenant_id: Uuid,

    /// Where the signal originated.
    pub source: EventSource,

    /// The actual signal data.
    pub payload: Payload,

    /// When the signal was emitted by the source.
    pub timestamp: DateTime<Utc>,

    /// When Veloxx ingested the event.
    pub ingested_at: DateTime<Utc>,

    /// Arbitrary key-value labels attached by the producer.
    pub labels: HashMap<String, String>,
}

impl TelemetryEvent {
    /// Create a new event with `ingested_at` set to now.
    pub fn new(
        tenant_id: Uuid,
        source: EventSource,
        payload: Payload,
        timestamp: DateTime<Utc>,
        labels: HashMap<String, String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id,
            source,
            payload,
            timestamp,
            ingested_at: Utc::now(),
            labels,
        }
    }
}

/// Source metadata identifying the originating process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSource {
    /// Logical service name (e.g. `"checkout-service"`).
    pub service: String,

    /// Container / pod / VM instance identifier.
    pub instance: String,

    /// Deployment environment (e.g. `"production"`, `"staging"`).
    pub environment: String,

    /// Kubernetes context or cluster name, if applicable.
    pub k8s_context: Option<String>,
}

/// Discriminated union over all signal types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Payload {
    Log(LogPayload),
    Metric(MetricPayload),
    Trace(TracePayload),
}

// ---------------------------------------------------------------------------
// Log
// ---------------------------------------------------------------------------

/// Structured log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogPayload {
    /// Parsed severity level.
    pub level: LogLevel,

    /// The raw / rendered log message.
    pub message: String,

    /// Optional trace correlation identifier.
    pub trace_id: Option<String>,

    /// Optional span identifier.
    pub span_id: Option<String>,

    /// Additional structured fields (logfmt / JSON extra keys).
    pub fields: HashMap<String, serde_json::Value>,
}

/// Log severity level with loose parsing support.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    /// Parse common level strings case-insensitively, including abbreviations
    /// such as `"ERR"`, `"WARN"`, `"WARNING"`, `"CRIT"`, etc.
    pub fn parse(s: &str) -> Self {
        match s.to_uppercase().trim() {
            "TRACE" | "TRC"                      => LogLevel::Trace,
            "DEBUG" | "DBG"                      => LogLevel::Debug,
            "INFO"  | "INF" | "INFORMATION"      => LogLevel::Info,
            "WARN"  | "WARNING" | "WRN"          => LogLevel::Warn,
            "ERROR" | "ERR" | "ERRO"             => LogLevel::Error,
            "FATAL" | "CRIT" | "CRITICAL" | "PANIC" => LogLevel::Fatal,
            _                                    => LogLevel::Info,
        }
    }

    /// Returns the canonical uppercase string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info  => "INFO",
            LogLevel::Warn  => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Metric
// ---------------------------------------------------------------------------

/// A single metric observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPayload {
    /// Metric name (e.g. `"http_request_duration_seconds"`).
    pub name: String,

    /// The metric value variant.
    pub value: MetricValue,

    /// Optional unit (e.g. `"ms"`, `"bytes"`).
    pub unit: Option<String>,
}

/// Discriminated union of Prometheus-compatible metric types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum MetricValue {
    /// A point-in-time value that can go up or down.
    Gauge { value: f64 },

    /// A monotonically increasing counter.
    Counter { value: f64 },

    /// Bucketed distribution — (le boundary, count) pairs.
    Histogram {
        buckets: Vec<(f64, u64)>,
        sum:     f64,
        count:   u64,
    },

    /// Pre-computed quantiles (quantile, value) pairs.
    Summary {
        quantiles: Vec<(f64, f64)>,
        sum:       f64,
        count:     u64,
    },
}

// ---------------------------------------------------------------------------
// Trace
// ---------------------------------------------------------------------------

/// Distributed trace span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracePayload {
    /// W3C / B3 trace identifier.
    pub trace_id: String,

    /// This span's identifier.
    pub span_id: String,

    /// Parent span identifier, if any.
    pub parent_span_id: Option<String>,

    /// Human-readable operation name.
    pub operation_name: String,

    /// Span start time.
    pub start_time: DateTime<Utc>,

    /// Span end time.
    pub end_time: DateTime<Utc>,

    /// Span status — `"OK"`, `"ERROR"`, etc.
    pub status: String,

    /// Key-value attributes / baggage.
    pub attributes: HashMap<String, serde_json::Value>,
}
