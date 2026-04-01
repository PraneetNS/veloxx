use anyhow::Result;
use common::telemetry::TelemetryEvent;
use tracing::debug;

pub struct KafkaProducer;

impl KafkaProducer {
    pub async fn new(_brokers: &str) -> Result<Self> { Ok(Self) }
    pub async fn publish(&self, event: &TelemetryEvent) -> Result<()> {
        debug!(event_id = %event.id, "kafka stub: dropped (no broker)");
        Ok(())
    }
    pub async fn publish_batch(&self, events: &[TelemetryEvent]) -> Result<()> {
        debug!(count = events.len(), "kafka stub: batch dropped");
        Ok(())
    }
}
