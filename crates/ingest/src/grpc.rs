use anyhow::Result;
use common::config::AppConfig;
use tracing::info;

pub async fn run(_cfg: AppConfig) -> Result<()> {
    info!("gRPC: stub mode");
    loop { tokio::time::sleep(tokio::time::Duration::from_secs(60)).await; }
}
