-- Migration 001: initial schema
-- Veloxx metadata database

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ---------------------------------------------------------------------------
-- Enums
-- ---------------------------------------------------------------------------

CREATE TYPE plan AS ENUM ('free', 'pro', 'enterprise');
CREATE TYPE user_role AS ENUM ('admin', 'member', 'viewer');
CREATE TYPE alert_severity AS ENUM ('info', 'warning', 'critical');
CREATE TYPE incident_status AS ENUM ('open', 'acknowledged', 'resolved');

-- ---------------------------------------------------------------------------
-- Tenants
-- ---------------------------------------------------------------------------

CREATE TABLE tenants (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                    TEXT NOT NULL,
    slug                    TEXT NOT NULL UNIQUE,
    plan                    plan NOT NULL DEFAULT 'free',
    ingest_rate_per_second  INTEGER NOT NULL DEFAULT 100,
    retention_days          INTEGER NOT NULL DEFAULT 7,
    max_users               INTEGER NOT NULL DEFAULT 3,
    max_ws_connections      INTEGER NOT NULL DEFAULT 5,
    is_active               BOOLEAN NOT NULL DEFAULT TRUE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tenants_slug ON tenants(slug);

-- ---------------------------------------------------------------------------
-- Users
-- ---------------------------------------------------------------------------

CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email           TEXT NOT NULL,
    password_hash   TEXT NOT NULL,
    role            user_role NOT NULL DEFAULT 'member',
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at   TIMESTAMPTZ,

    UNIQUE (tenant_id, email)
);

CREATE INDEX idx_users_tenant_id ON users(tenant_id);
CREATE INDEX idx_users_email     ON users(email);

-- ---------------------------------------------------------------------------
-- Alert Rules
-- ---------------------------------------------------------------------------

CREATE TABLE alert_rules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    description     TEXT,
    severity        alert_severity NOT NULL DEFAULT 'warning',
    -- Rule type: 'threshold' | 'ai_anomaly'
    rule_type       TEXT NOT NULL DEFAULT 'threshold',
    -- JSON-encoded rule config (metric name, comparator, threshold, window_secs, etc.)
    rule_config     JSONB NOT NULL DEFAULT '{}',
    -- Notification channels JSON array: [{type: "slack", url: "..."}, ...]
    channels        JSONB NOT NULL DEFAULT '[]',
    -- Seconds before the same alert can fire again.
    cooldown_secs   INTEGER NOT NULL DEFAULT 300,
    is_enabled      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alert_rules_tenant_id ON alert_rules(tenant_id);

-- ---------------------------------------------------------------------------
-- Incidents
-- ---------------------------------------------------------------------------

CREATE TABLE incidents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    alert_rule_id   UUID REFERENCES alert_rules(id) ON DELETE SET NULL,
    title           TEXT NOT NULL,
    description     TEXT,
    severity        alert_severity NOT NULL DEFAULT 'warning',
    status          incident_status NOT NULL DEFAULT 'open',
    -- JSON snapshot of the anomaly / metric values that triggered the alert.
    context         JSONB NOT NULL DEFAULT '{}',
    opened_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    acknowledged_at TIMESTAMPTZ,
    resolved_at     TIMESTAMPTZ,
    acknowledged_by UUID REFERENCES users(id) ON DELETE SET NULL,
    resolved_by     UUID REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_incidents_tenant_id    ON incidents(tenant_id);
CREATE INDEX idx_incidents_status       ON incidents(status);
CREATE INDEX idx_incidents_opened_at    ON incidents(opened_at DESC);

-- ---------------------------------------------------------------------------
-- Refresh tokens
-- ---------------------------------------------------------------------------

CREATE TABLE refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
