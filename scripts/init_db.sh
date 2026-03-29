#!/usr/bin/env bash
set -e

# Run migrations and seed data for Veloxx local dev.

DB_URL="postgres://veloxx:password@localhost:5432/veloxx"

echo "--- 1. Migrating Postgres ---"
sqlx migrate run --database-url "$DB_URL" --source crates/storage/migrations

echo "--- 2. Seeding ClickHouse ---"
# Create ClickHouse tables (handled by storage service main, but we can call it here if needed)

echo "--- 3. Seeding Default Tenant ---"
psql "$DB_URL" -f scripts/seed_data.sql

echo "--- Done! ---"
echo "Tenant: demo"
echo "User: admin@veloxx.ai"
echo "Password: admin123"
