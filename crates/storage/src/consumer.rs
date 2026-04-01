use anyhow::Result;
use common::config::AppConfig;
use tracing::info;

pub async fn run(_cfg: AppConfig) -> Result<()> {
    info!("storage consumer: stub mode (no Kafka)");
    loop { tokio::time::sleep(tokio::time::Duration::from_secs(60)).await; }
}
