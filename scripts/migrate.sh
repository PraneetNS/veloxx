#!/usr/bin/env bash
set -e

# Run sqlx migrations for Postgres.
# Requires sqlx-cli to be installed: cargo install sqlx-cli

DATABASE_URL="postgres://veloxx:password@localhost:5432/veloxx"

echo "Checking database connection..."
until psql "$DATABASE_URL" -c '\q'; do
  >&2 echo "Postgres is unavailable - sleeping"
  sleep 1
done

echo "Running migrations..."
sqlx migrate run --database-url "$DATABASE_URL" --source crates/storage/migrations

echo "Database is ready!"
