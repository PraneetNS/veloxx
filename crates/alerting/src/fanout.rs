//! Fanout for alert notifications.
//!
//! Delivers alerts to external channels:
//! - Slack Webhooks
//! - PagerDuty Events API v2
//! - Generic Webhooks

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Channel {
    #[serde(rename = "slack")]
    Slack { url: String },
    #[serde(rename = "pagerduty")]
    PagerDuty { routing_key: String },
    #[serde(rename = "webhook")]
    Webhook { url: String },
}

pub struct Fanout {
    http: Client,
}

impl Fanout {
    pub fn new() -> Self {
        Self {
            http: Client::new(),
        }
    }

    pub async fn dispatch(&self, channel: &Channel, title: &str, description: &str) -> Result<()> {
        match channel {
            Channel::Slack { url } => {
                let payload = serde_json::json!({
                    "text": format!("*{}*\n{}", title, description)
                });
                self.http.post(url).json(&payload).send().await
                    .context("slack dispatch")?;
                info!(channel = "slack", "notification sent");
            }
            Channel::PagerDuty { routing_key } => {
                let payload = serde_json::json!({
                    "routing_key": routing_key,
                    "event_action": "trigger",
                    "payload": {
                        "summary": title,
                        "source": "veloxx-alerting",
                        "severity": "critical"
                    }
                });
                self.http.post("https://events.pagerduty.com/v2/enqueue")
                    .json(&payload).send().await
                    .context("pagerduty dispatch")?;
                info!(channel = "pagerduty", "notification sent");
            }
            Channel::Webhook { url } => {
                let payload = serde_json::json!({
                    "title": title,
                    "description": description,
                    "timestamp": chrono::Utc::now()
                });
                self.http.post(url).json(&payload).send().await
                    .context("webhook dispatch")?;
                info!(channel = "webhook", "notification sent");
            }
        }
        Ok(())
    }
}
