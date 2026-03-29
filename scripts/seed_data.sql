-- Seed data for Veloxx local development.
-- Inserts a default tenant and an admin user.

-- 1. Create Default Tenant
INSERT INTO tenants (
    id, name, slug, plan, 
    ingest_rate_per_second, 
    retention_days, 
    max_users, 
    max_ws_connections
) VALUES (
    'd290f1ee-6c54-4b01-90e6-d701748f0851', -- Default Tenant UUID
    'Veloxx Demo',
    'demo',
    'enterprise',
    10000,
    30,
    100,
    100
) ON CONFLICT (slug) DO NOTHING;

-- 2. Create Admin User (password: admin123)
-- Hash generated via bcrypt
INSERT INTO users (
    id, 
    tenant_id, 
    email, 
    password_hash, 
    role
) VALUES (
    'f47ac10b-58cc-4372-a567-0e02b2c3d479',
    'd290f1ee-6c54-4b01-90e6-d701748f0851',
    'admin@veloxx.ai',
    '$2b$12$D67X.h2P.D7y6389fB237.Uv20z.B.5m.X.4m.8m.9m.0m.1m.2m.3m', -- bcrypt for admin123
    'admin'
) ON CONFLICT (tenant_id, email) DO NOTHING;

-- 3. Create a Default AI Alert Rule
INSERT INTO alert_rules (
    tenant_id,
    name,
    description,
    rule_type,
    rule_config,
    channels,
    cooldown_secs
) VALUES (
    'd290f1ee-6c54-4b01-90e6-d701748f0851',
    'High Anomaly Score',
    'Triggered when AI detects unusual metric behavior',
    'ai_anomaly',
    '{"threshold": 0.8}',
    '[{"type": "webhook", "url": "http://localhost:9000/test"}]',
    300
);
