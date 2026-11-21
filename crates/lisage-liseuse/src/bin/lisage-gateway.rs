//! D6-2 — `lisage-gateway`: loopback WebSocket gateway serving the offline
//! page and replaying a `.jsonl` trace, bound to `127.0.0.1` only with a
//! random token required on every connection.
//!
//! Usage: `lisage-gateway <trace.jsonl> [title]`
//!
//! Prints the HTTP and WebSocket URLs (with token) to stderr.

use std::sync::Arc;

use lisage_core::observer::NoOpObserver;
use lisage_liseuse::gateway::{GatewayEvents, run_gateway};

#[tokio::main]
async fn main() {
    let trace_path = std::env::args()
        .nth(1)
        .expect("usage: lisage-gateway <trace.jsonl> [title]");
    let title = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "lisage — live trace".to_string());
    let trace_path_label = trace_path.clone();
    let events = GatewayEvents {
        announce: Box::new(move |port, token, event_count| {
            eprintln!("lisage-gateway: http://127.0.0.1:{port}/?token={token}");
            eprintln!("lisage-gateway: ws://127.0.0.1:{port}/ws?token={token}");
            eprintln!("lisage-gateway: {event_count} events loaded from {trace_path_label}");
        }),
        connection_error: Box::new(|message| eprintln!("connection error: {message}")),
    };
    if let Err(error) = run_gateway(&trace_path, &title, Arc::new(NoOpObserver), events).await {
        eprintln!("gateway error: {error}");
        std::process::exit(1);
    }
}
