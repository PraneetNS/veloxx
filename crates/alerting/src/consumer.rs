//! Alerting consumer stub for local development.

use anyhow::Result;
use common::config::AppConfig;
use sqlx::PgPool;
use tracing::info;

/// Main consumer loop.
pub async fn run(_cfg: AppConfig, _pool: PgPool) -> Result<()> {
    info!("alerting consumer: stub mode (no Kafka in local dev)");
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
