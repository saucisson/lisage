//! D6-2 — `lisage-gateway`: loopback WebSocket gateway serving the offline
//! page and replaying a `.jsonl` trace, bound to `127.0.0.1` only with a
//! random token required on every connection.
//!
//! Usage: `lisage-gateway <trace.jsonl> [title]`
//!
//! Prints the HTTP and WebSocket URLs (with token) to stderr.

use lisage_liseuse::gateway::run_gateway;

#[tokio::main]
async fn main() {
    let trace_path = std::env::args()
        .nth(1)
        .expect("usage: lisage-gateway <trace.jsonl> [title]");
    let title = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "lisage — live trace".to_string());
    if let Err(e) = run_gateway(&trace_path, &title).await {
        eprintln!("gateway error: {e}");
        std::process::exit(1);
    }
}
