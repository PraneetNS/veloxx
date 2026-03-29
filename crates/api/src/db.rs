//! Database connection pool.

use anyhow::Result;
use common::config::PostgresConfig;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::info;

/// Connect to Postgres and return a connection pool.
pub async fn connect(cfg: &PostgresConfig) -> Result<PgPool> {
    info!("connecting to postgres");
    let pool = PgPoolOptions::new()
        .max_connections(cfg.max_connections)
        .connect(&cfg.url)
        .await?;
    info!("postgres connection pool ready");
    Ok(pool)
}
