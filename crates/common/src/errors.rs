//! Veloxx unified error types.

use thiserror::Error;
use uuid::Uuid;

/// Top-level error enum for all Veloxx services.
#[derive(Debug, Error)]
pub enum VeloxxError {
    // -----------------------------------------------------------------------
    // Auth / Tenant
    // -----------------------------------------------------------------------
    #[error("JWT token is invalid or expired")]
    InvalidToken,

    #[error("tenant {0} not found")]
    TenantNotFound(Uuid),

    #[error("access denied: tenant mismatch")]
    TenantAccessDenied,

    #[error("tenant {0} has exceeded its rate limit")]
    RateLimitExceeded(Uuid),

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------
    #[error("validation error: {0}")]
    Validation(String),

    #[error("payload too large: {size} bytes, limit {limit} bytes")]
    PayloadTooLarge { size: usize, limit: usize },

    // -----------------------------------------------------------------------
    // I/O + Infrastructure
    // -----------------------------------------------------------------------
    #[error("kafka producer error: {0}")]
    KafkaProducer(String),

    #[error("kafka consumer error: {0}")]
    KafkaConsumer(String),

    #[error("clickhouse error: {0}")]
    ClickHouse(String),

    #[error("postgres error: {0}")]
    Postgres(#[from] sqlx::Error),

    #[error("redis error: {0}")]
    Redis(String),

    #[error("qdrant error: {0}")]
    Qdrant(String),

    // -----------------------------------------------------------------------
    // AI / External
    // -----------------------------------------------------------------------
    #[error("ai-engine gRPC error: {0}")]
    AiEngine(String),

    #[error("LLM API error: {0}")]
    LlmApi(String),

    // -----------------------------------------------------------------------
    // Alerting / Incident
    // -----------------------------------------------------------------------
    #[error("alert rule {0} not found")]
    AlertRuleNotFound(Uuid),

    #[error("notification delivery failed: {channel} — {reason}")]
    NotificationFailed { channel: String, reason: String },

    // -----------------------------------------------------------------------
    // Generic
    // -----------------------------------------------------------------------
    #[error("not found: {0}")]
    NotFound(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl VeloxxError {
    /// HTTP status code appropriate for this error variant.
    pub fn status_code(&self) -> u16 {
        match self {
            VeloxxError::InvalidToken
            | VeloxxError::TenantAccessDenied          => 401,
            VeloxxError::TenantNotFound(_)
            | VeloxxError::AlertRuleNotFound(_)
            | VeloxxError::NotFound(_)                 => 404,
            VeloxxError::RateLimitExceeded(_)          => 429,
            VeloxxError::Validation(_)
            | VeloxxError::PayloadTooLarge { .. }      => 400,
            _                                          => 500,
        }
    }
}

/// A convenient alias used throughout the workspace.
pub type VeloxxResult<T> = Result<T, VeloxxError>;
