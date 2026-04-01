//! Persistent incident store using Postgres.
//!
//! Creates and updates lifecycle of incidents: `open` → `acknowledged` → `resolved`.

use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;
use tracing::info;

#[derive(sqlx::FromRow)]
struct IncidentIdRow {
    id: Uuid,
}

pub struct IncidentStore {
    pool: std::sync::Arc<PgPool>,
}

impl IncidentStore {
    pub fn new(pool: std::sync::Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Open a new incident if it doesn't already exist for this tenant and alert rule.
    pub async fn open(
        &self,
        tenant_id: Uuid,
        alert_rule_id: Uuid,
        title: &str,
        description: &str,
        context: &serde_json::Value,
    ) -> Result<Uuid> {
        info!(tenant_id = %tenant_id, alert_rule_id = %alert_rule_id, "opening incident");

        let id = sqlx::query_as::<_, IncidentIdRow>(
            r#"INSERT INTO incidents (tenant_id, alert_rule_id, title, description, context, status)
               VALUES ($1, $2, $3, $4, $5, 'open')
               RETURNING id"#
        )
        .bind(tenant_id)
        .bind(alert_rule_id)
        .bind(title)
        .bind(description)
        .bind(context)
        .fetch_one(&*self.pool)
        .await
        .context("db: open incident")?;

        info!(incident_id = %id.id, "incident opened");
        Ok(id.id)
    }

    /// Resolve an open incident.
    pub async fn resolve(&self, incident_id: Uuid, resolved_by: Option<Uuid>) -> Result<()> {
        sqlx::query(
            r#"UPDATE incidents
               SET status = 'resolved', resolved_at = NOW(), resolved_by = $1
               WHERE id = $2"#
        )
        .bind(resolved_by)
        .bind(incident_id)
        .execute(&*self.pool)
        .await
        .context("db: resolve incident")?;

        info!(incident_id = %incident_id, "incident resolved");
        Ok(())
    }
}
