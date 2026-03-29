//! Application configuration loaded from env vars and TOML file.
//!
//! All environment variables must be prefixed with `VELOXX__` and use
//! double-underscore (`__`) as a hierarchy separator.
//!
//! Example: `VELOXX__KAFKA__BROKERS=redpanda:9092`

use anyhow::Result;
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};

/// Complete application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server:     ServerConfig,
    pub kafka:      KafkaConfig,
    pub clickhouse: ClickHouseConfig,
    pub postgres:   PostgresConfig,
    pub redis:      RedisConfig,
    pub qdrant:     QdrantConfig,
    pub ai:         AiConfig,
    pub auth:       AuthConfig,
    pub log:        LogConfig,
}

impl AppConfig {
    /// Load config from `config/default.toml` overlaid by `VELOXX__*` env vars.
    pub fn load() -> Result<Self> {
        let cfg = Config::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(
                Environment::with_prefix("VELOXX")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        Ok(cfg.try_deserialize()?)
    }
}

// ---------------------------------------------------------------------------
// Sub-configs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// HTTP/REST listen address.
    pub http_host: String,
    pub http_port: u16,

    /// gRPC listen address.
    pub grpc_host: String,
    pub grpc_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Comma-separated list of Kafka broker addresses.
    pub brokers: String,

    /// Consumer group prefix (service name appended at runtime).
    pub group_id_prefix: String,

    /// Producer linger in milliseconds (batching window).
    pub linger_ms: u32,

    /// Compression codec for producer (`snappy`, `lz4`, `gzip`, `none`).
    pub compression: String,

    /// Topic for logs.
    pub topic_logs: String,

    /// Topic for metrics.
    pub topic_metrics: String,

    /// Topic for traces.
    pub topic_traces: String,

    /// Topic for AI-published anomaly events.
    pub topic_anomalies: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickHouseConfig {
    pub url:      String,
    pub database: String,
    pub username: String,
    pub password: String,

    /// Flush buffer size in events.
    pub flush_batch_size: usize,

    /// Flush interval in seconds.
    pub flush_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    pub url: String,

    /// Maximum connection pool size.
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    pub host: String,
    pub port: u16,

    /// gRPC port for Qdrant.
    pub grpc_port: u16,

    /// Collection name for log vectors.
    pub collection: String,

    /// Vector dimensionality (must match embedding model).
    pub vector_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Base URL of the ai-engine FastAPI service.
    pub engine_url: String,

    /// gRPC address for the ai-engine.
    pub engine_grpc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// HS256 JWT signing secret.
    pub jwt_secret: String,

    /// Token expiry in seconds.
    pub jwt_expiry_secs: u64,

    /// Refresh token expiry in seconds.
    pub refresh_expiry_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// `trace`, `debug`, `info`, `warn`, `error`.
    pub level: String,

    /// `json` or `pretty`.
    pub format: String,
}
