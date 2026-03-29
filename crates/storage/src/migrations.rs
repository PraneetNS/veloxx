//! Postgres migration runner using sqlx.
//!
//! Migrations live in `crates/storage/migrations/`.

use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, migrate::Migrator};
use tracing::info;
use std::path::Path;

/// Run all pending Postgres migrations.
pub async fn run(database_url: &str) -> Result<()> {
    info!("running postgres migrations");

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(database_url)
        .await
        .context("connect to postgres for migrations")?;

    let migrator = Migrator::new(Path::new("crates/storage/migrations"))
        .await
        .context("load migrations")?;

    migrator.run(&pool).await.context("run migrations")?;

    info!("postgres migrations complete");
    Ok(())
}
