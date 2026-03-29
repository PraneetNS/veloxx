FROM rust:1.74-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    g++ \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release --bin storage

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates postgresql-client && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/storage /app/storage
COPY config /app/config
COPY crates/storage/migrations /app/crates/storage/migrations
CMD ["./storage"]
