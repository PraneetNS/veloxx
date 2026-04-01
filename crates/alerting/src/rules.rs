//! Alert rule evaluation engine.
//!
//! Evaluates incoming anomalies from the anomaly detection service against
//! the configured alert rules in Postgres.

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use tracing::{debug, error, info};
use serde::{Deserialize, Serialize};

use crate::{dedup::DedupWindow, fanout::Fanout, incident::IncidentStore, fanout::Channel};

#[derive(Debug, Serialize, Deserialize)]
pub struct AlertRule {
    pub id:            Uuid,
    pub tenant_id:     Uuid,
    pub name:          String,
    pub description:   Option<String>,
    pub severity:      String,
    pub rule_type:     String,
    pub rule_config:   serde_json::Value,
    pub channels:      Vec<Channel>,
    pub cooldown_secs: i32,
    pub is_enabled:    bool,
}

pub struct RuleEngine {
    pool: Arc<PgPool>
}

#[derive(Debug, sqlx::FromRow)]
struct AlertRuleRow {
    id: Uuid,
    tenant_id: Uuid,
    name: String,
    description: Option<String>,
    severity: String,
    rule_type: String,
    rule_config: serde_json::Value,
    channels: serde_json::Value,
    cooldown_secs: i32,
    is_enabled: bool,
}

impl RuleEngine {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Evaluate an incoming anomaly against active alert rules.
    pub async fn evaluate(
        &self,
        anomaly: &serde_json::Value,
        dedup: Arc<DedupWindow>,
        fanout: Arc<Fanout>,
        incident: Arc<IncidentStore>,
    ) -> Result<()> {
        let tenant_id_str = anomaly.get("tenant_id").and_then(|v| v.as_str())
            .context("missing tenant_id in anomaly")?;
        let tenant_id = Uuid::parse_str(tenant_id_str)?;

        let metric_name = anomaly.get("metric_name").and_then(|v| v.as_str())
            .unwrap_or("unknown");

        // Fetch enabled rules for this tenant.
        let rules = sqlx::query_as::<_, AlertRuleRow>(
            r#"SELECT id, tenant_id, name, description,
                      severity::text as severity,
                      rule_type, rule_config, channels, cooldown_secs, is_enabled
               FROM alert_rules
               WHERE tenant_id = $1 AND is_enabled = TRUE"#
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await?;

        for row in rules {
            // Rule logic: if AI anomaly rule, and anomaly score is high enough.
            if row.rule_type == "ai_anomaly" {
                let threshold = row.rule_config.get("threshold").and_then(|v| v.as_f64()).unwrap_or(0.8);
                let score = anomaly.get("anomaly_score").and_then(|v| v.as_f64()).unwrap_or(0.0);

                if score >= threshold {
                    if dedup.allow(row.id, row.cooldown_secs as u64) {
                        info!(rule = %row.name, "alert rule matched — firing alert");

                        let title = format!("Anomaly detected: {}", row.name);
                        let fallback_desc = format!("Metric: {}", metric_name);
                        let desc = row.description.as_deref().unwrap_or(&fallback_desc);

                        // Store incident.
                        incident.open(tenant_id, row.id, &title, desc, anomaly).await?;

                        // Fanout notifications.
                        let channels: Vec<Channel> = serde_json::from_value(row.channels)?;
                        for channel in &channels {
                            if let Err(e) = fanout.dispatch(channel, &title, desc).await {
                                error!(error = %e, "dispatch failed");
                            }
                        }
                    } else {
                        debug!("skipping alert — in cooldown");
                    }
                }
            }
        }
        Ok(())
    }
}
