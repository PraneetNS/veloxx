# Veloxx 🚀

**Veloxx** is a production-grade, AI-native observability platform built for the modern era. It provides a unified ecosystem for ingesting, storing, and analyzing telemetry data (logs, metrics, and traces) with a focus on high performance, multi-tenancy, and AI-driven insights.

![Veloxx Architecture](https://img.shields.io/badge/Architecture-Distributed-blue)
![Tech Stack](https://img.shields.io/badge/Rust-2021-orange)
![Tech Stack](https://img.shields.io/badge/Next.js-14-black)
![Tech Stack](https://img.shields.io/badge/ClickHouse-Enabling-green)

---

## 🌟 Key Features

-   **Unified Signal Ingestion:** Industry-standard OpenTelemetry (OTLP) support for logs, metrics, and traces.
-   **AI-Native Insights:** Built-in vector search (Qdrant) and Python-based AI engine for pattern detection and root-cause analysis.
-   **High-Performance Storage:** Powered by ClickHouse for lightning-fast time-series queries and analytical workloads.
-   **Multi-tenant by Design:** Every signal is scoped by `tenant_id` at the core level, ensuring data isolation and security.
-   **Scalable Architecture:** Microservices-based design using Redpanda (Kafka) for event streaming and Rust for low-latency processing.
-   **Modern Dashboard:** A sleek, reactive Next.js dashboard for real-time visualization and alerting.

---


---

## 🛠️ Tech Stack

-   **Backend:** Rust (Tokio, Axum, Tonic, Sqlx)
-   **AI Engine:** Python (FastAPI, PyTorch/Transformers)
-   **Frontend:** Next.js (TypeScript, Tailwind CSS, ShadcnUI)
-   **Data Layers:**
    -   **ClickHouse:** Analytical data & logs.
    -   **PostgreSQL:** Relational metadata.
    -   **Qdrant:** Vector embeddings.
    -   **Redpanda:** High-throughput event streaming.
    -   **Redis:** Caching and real-time state.

---

## 🚀 Getting Started

### Prerequisites

-   [Docker](https://www.docker.com/) & Docker Compose
-   [Rust](https://www.rust-lang.org/) (if building locally)
-   [Node.js](https://nodejs.org/) (if running dashboard locally)

### Quick Start with Docker

The entire platform can be brought up using the provided infrastructure configuration:

```bash
# Clone the repository
git clone https://github.com/veloxx/veloxx.git
cd veloxx

# Start the entire stack
docker-compose -f infra/docker-compose.yml up --build
```

Once running, the services will be available at:
-   **Dashboard:** `http://localhost:3000`
-   **API Gateway:** `http://localhost:8080`
-   **OTLP Ingest (gRPC):** `localhost:4317`
-   **OTLP Ingest (HTTP):** `localhost:4318`

---

## 📂 Project Structure

```text
.
├── crates/
│   ├── ai-core/      # Shared AI logic and embedding consumers
│   ├── alerting/     # Real-time alerting engine
│   ├── api/          # Multi-tenant REST/WebSocket API
│   ├── common/       # Core telemetry models and utilities
│   ├── ingest/       # OTLP ingestion service
│   └── storage/      # Multi-sink storage consumer (ClickHouse, Postgres, Qdrant)
├── services/
│   └── ai-engine/    # Python FastAPI service for AI processing
├── dashboard/        # Next.js frontend application
├── infra/            # Docker Compose and infrastructure config
├── k8s/              # Kubernetes deployment manifests
└── scripts/          # Utility and management script
