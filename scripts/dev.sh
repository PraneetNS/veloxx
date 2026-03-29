#!/usr/bin/env bash
set -e

# Veloxx Development Script (Bash)
#
# Starts the infrastructure stack and then watches all the Rust crates.

echo "--- 1. Starting Infra Stack (Redpanda, ClickHouse, Postgres, Qdrant...) ---"
docker-compose -f infra/docker-compose.yml up -d redpanda clickhouse postgres qdrant redis

echo "--- 2. Waiting for Postgres to start ---"
until docker exec -it $(docker-compose -f infra/docker-compose.yml ps -q postgres) pg_isready -U veloxx -d veloxx; do
  >&2 echo "Postgres is still booting..."
  sleep 2
done

echo "--- 3. Running Migrations ---"
./scripts/migrate.sh

echo "--- 4. Seeding Database ---"
./scripts/seed.sh

echo "--- 5. Starting Veloxx Microservices ---"
docker-compose -f infra/docker-compose.yml up --build
echo "--- Done! ---"
