//! Alerting service entry-point.

use anyhow::Result;
use common::config::AppConfig;
use tracing::info;

mod consumer;
mod dedup;
mod fanout;
mod incident;
mod rules;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!(service = "alerting", "starting Veloxx alerting service");

    let cfg  = AppConfig::load()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(cfg.postgres.max_connections)
        .connect(&cfg.postgres.url)
        .await?;

    consumer::run(cfg, pool).await?;
    Ok(())
}
