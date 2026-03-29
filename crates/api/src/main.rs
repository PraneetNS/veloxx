//! Veloxx REST API service entry-point.

use anyhow::Result;
use common::config::AppConfig;
use tracing::info;

mod auth;
mod db;
mod handlers;
mod middleware;
mod router;
mod ws;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!(service = "api", "starting Veloxx API service");

    let cfg = AppConfig::load()?;
    let addr = format!("{}:{}", cfg.server.http_host, cfg.server.http_port);

    let pool = db::connect(&cfg.postgres).await?;
    let app  = router::build(cfg, pool).await?;

    info!(addr, "api HTTP server listening");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
