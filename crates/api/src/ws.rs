//! WebSocket handler for the live metric stream.
//!
//! Each connected client subscribes to a tenant's metric stream.
//! A background task polls ClickHouse for recent metrics every second
//! and broadcasts them to all connected clients.

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::time::{interval, Duration};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::handlers::ApiState;

/// Handle a WebSocket connection for the live metric stream.
pub async fn handle_ws(mut socket: WebSocket, tenant_id: Uuid, state: ApiState) {
    debug!(tenant_id = %tenant_id, "ws client connected");

    let mut ticker   = interval(Duration::from_secs(1));
    let mut closed   = false;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                // Query the last 10 metric rows for this tenant.
                let sql = format!(
                    "SELECT name, value, service, timestamp \
                     FROM metrics \
                     WHERE tenant_id = '{}' \
                     ORDER BY timestamp DESC LIMIT 10",
                    tenant_id
                );

                match state.ch_client.query(&sql).fetch_all::<serde_json::Value>().await {
                    Ok(rows) => {
                        let msg = serde_json::to_string(&serde_json::json!({"metrics": rows}))
                            .unwrap_or_default();
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            debug!("ws client disconnected");
                            closed = true;
                        }
                    }
                    Err(e) => warn!(error = %e, "ws clickhouse query failed"),
                }
            }

            msg = socket.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        debug!("ws client closed");
                        closed = true;
                    }
                    Some(Ok(Message::Ping(p))) => {
                        let _ = socket.send(Message::Pong(p)).await;
                    }
                    _ => {}
                }
            }
        }

        if closed { break; }
    }
}
