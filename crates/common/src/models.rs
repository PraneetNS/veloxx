//! Tenant and user domain models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------

/// Subscription plan that gates features and limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    Free,
    Pro,
    Enterprise,
}

impl Default for Plan {
    fn default() -> Self {
        Plan::Free
    }
}

// ---------------------------------------------------------------------------
// TenantLimits
// ---------------------------------------------------------------------------

/// Quota limits enforced at ingest and query time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantLimits {
    /// Maximum ingest events per second.
    pub ingest_rate_per_second: u32,

    /// Maximum data retention in days.
    pub retention_days: u32,

    /// Maximum seats / active users.
    pub max_users: u32,

    /// Maximum simultaneous WebSocket connections.
    pub max_ws_connections: u32,
}

impl TenantLimits {
    /// Default limits for the Free plan.
    pub fn free() -> Self {
        Self {
            ingest_rate_per_second: 100,
            retention_days:         7,
            max_users:              3,
            max_ws_connections:     5,
        }
    }

    /// Default limits for the Pro plan.
    pub fn pro() -> Self {
        Self {
            ingest_rate_per_second: 5_000,
            retention_days:         30,
            max_users:              25,
            max_ws_connections:     50,
        }
    }

    /// Default limits for the Enterprise plan.
    pub fn enterprise() -> Self {
        Self {
            ingest_rate_per_second: 100_000,
            retention_days:         365,
            max_users:              u32::MAX,
            max_ws_connections:     u32::MAX,
        }
    }

    /// Return default limits for a given plan.
    pub fn for_plan(plan: &Plan) -> Self {
        match plan {
            Plan::Free       => Self::free(),
            Plan::Pro        => Self::pro(),
            Plan::Enterprise => Self::enterprise(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tenant
// ---------------------------------------------------------------------------

/// A Veloxx tenant (organisation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id:         Uuid,
    pub name:       String,
    pub slug:       String,
    pub plan:       Plan,
    pub limits:     TenantLimits,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active:  bool,
}

// ---------------------------------------------------------------------------
// User
// ---------------------------------------------------------------------------

/// A user who belongs to exactly one tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id:            Uuid,
    pub tenant_id:     Uuid,
    pub email:         String,
    /// bcrypt-hashed password — never expose in API responses.
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role:          UserRole,
    pub created_at:    DateTime<Utc>,
    pub is_active:     bool,
}

/// Role-based access control within a tenant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Member,
    Viewer,
}
