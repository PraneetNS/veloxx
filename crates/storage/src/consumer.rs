use anyhow::Result;
use common::config::AppConfig;
use tracing::info;

pub async fn run(cfg: AppConfig) -> Result<()> {
    info!("storage consumer: stub mode (no Kafka in local dev)");
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
