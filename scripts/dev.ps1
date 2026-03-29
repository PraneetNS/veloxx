# Veloxx Development Script (PowerShell)
# 
# Starts the infrastructure stack and watches for changes in all Rust crates.

# 1. Spin up infra stack
Write-Host "--- 1. Starting Infra Stack (Redpanda, ClickHouse, Postgres, Qdrant...) ---" -ForegroundColor Cyan
docker-compose -f infra/docker-compose.yml up -d redpanda clickhouse postgres qdrant redis

# 2. Open dashboard in background
Write-Host "--- 2. Starting Frontend Dashboard ---" -ForegroundColor Cyan
cd dashboard; npm run dev & ; cd ..

# 3. Watch Rust backend services
Write-Host "--- 3. Starting Rust Services (Watcher) ---" -ForegroundColor Cyan
# Run multiple cargo-watch split by command (or just run docker compose with all services)
docker-compose -f infra/docker-compose.yml up --build
