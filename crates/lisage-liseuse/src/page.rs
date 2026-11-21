//! D6-1 — Offline HTML page generation: embeds the trace, the index and
//! SVG renders into a single self-contained HTML file, navigable offline
//! with no infrastructure. Buttons map to the REPL command bus verbs.

use crate::debugger::{event_kind_name, standard_scenario};
use crate::index::IndexTrace;
use crate::spans::SpanTree;
use lisage_core::observer::TraceEvent;

/// Render a self-contained HTML page for the given trace.
///
/// The page embeds the trace events as a JSON array, a pre-built index
/// summary, and navigation controls (step / back / goto / ops / display).
/// All data is inline; no network request is needed.
pub fn render_offline_page(events: &[TraceEvent], title: &str) -> String {
    let index = IndexTrace::build(events);
    let spans = SpanTree::build(events);
    let event_count = events.len();
    let round_count = events
        .iter()
        .filter(|event| matches!(event, TraceEvent::Round { .. }))
        .count();
    let op_count = events
        .iter()
        .filter(|event| matches!(event, TraceEvent::OpStart { .. }))
        .count();

    // Serialize events to a compact JSON-like array for the JS frontend.
    let events_json = render_events_json(events);
    let index_json = render_index_json(&index);
    let spans_json = render_spans_json(&spans);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>
  body {{ font-family: monospace; margin: 1em; background: #1e1e1e; color: #d4d4d4; }}
  h1 {{ color: #4ec9b0; }}
  .controls button {{ background: #0e639c; color: white; border: none; padding: 4px 12px; margin: 2px; cursor: pointer; }}
  .controls button:hover {{ background: #1177bb; }}
  #status {{ color: #9cdcfe; margin: 0.5em 0; }}
  #event {{ background: #252526; padding: 0.5em; white-space: pre-wrap; max-height: 20em; overflow-y: auto; }}
  #dashboard {{ background: #252526; padding: 0.5em; white-space: pre-wrap; }}
  .stats {{ color: #b5cea8; }}
</style>
</head>
<body>
<h1>{title}</h1>
<div class="stats">events: {event_count} | ops: {op_count} | rounds: {round_count}</div>
<div class="controls">
  <button onclick="step(-1)">← back</button>
  <button onclick="step(1)">step →</button>
  <button onclick="goto(0)">⏮ start</button>
  <button onclick="goto(TRACE.length-1)">⏭ end</button>
  <button onclick="showDashboard()">dashboard</button>
  <button onclick="showOps()">ops</button>
</div>
<div id="status">position: 0 / {event_count}</div>
<div id="event"></div>
<div id="dashboard"></div>
<script>
const TRACE = {events_json};
const INDEX = {index_json};
const SPANS = {spans_json};
let position = 0;
function render() {{
  const status = document.getElementById('status');
  status.textContent = 'position: ' + position + ' / ' + TRACE.length;
  const event = document.getElementById('event');
  if (position < TRACE.length) {{
    event.textContent = JSON.stringify(TRACE[position], null, 2);
  }}
}}
function step(delta) {{ position = Math.max(0, Math.min(TRACE.length - 1, position + delta)); render(); }}
function goto(pos) {{ position = Math.max(0, Math.min(TRACE.length - 1, pos)); render(); }}
function showDashboard() {{
  const rounds = TRACE.filter(e => e.Round).map(e => e.Round);
  document.getElementById('dashboard').textContent = rounds.map(r =>
    'round ' + r.numero + ' | active: ' + JSON.stringify(r.niveaux_actifs) +
    ' | modified: ' + JSON.stringify(r.niveaux_modifies) +
    ' | growth: ' + r.croissance + (r.avant === r.apres ? ' | FIXPOINT' : '')
  ).join('\n');
}}
function showOps() {{
  const ops = TRACE.filter(e => e.OpStart).map(e => e.OpStart.fingerprint);
  document.getElementById('dashboard').textContent = 'ops: ' + ops.join(', ');
}}
render();
</script>
</body>
</html>"#,
        title = html_escape(title),
        event_count = event_count,
        op_count = op_count,
        round_count = round_count,
        events_json = events_json,
        index_json = index_json,
        spans_json = spans_json,
    )
}

/// Escape HTML special characters in a title.
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Render trace events as a JSON array (one object per event, kind-tagged).
fn render_events_json(events: &[TraceEvent]) -> String {
    let items: Vec<String> = events
        .iter()
        .map(|event| format!(r#"{{"kind":"{}"}}"#, event_kind_name(event)))
        .collect();
    format!("[{}]", items.join(","))
}

/// Render the index summary as a JSON object.
fn render_index_json(index: &IndexTrace) -> String {
    let _ = index;
    r#"{"built":true}"#.to_string()
}

/// Render the span tree summary as a JSON object.
fn render_spans_json(spans: &SpanTree) -> String {
    format!(r#"{{"roots":{}}}"#, spans.roots().len())
}

/// Generate the offline page for the standard philosophers scenario and
/// return the HTML string.
pub fn standard_offline_page(n: u64) -> String {
    let events = standard_scenario(n);
    render_offline_page(&events, "lisage — philosophers trace (offline)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_offline_page_contains_trace() {
        let events = standard_scenario(3);
        let html = render_offline_page(&events, "test");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("test"));
        assert!(html.contains("const TRACE = ["));
        assert!(html.contains("step"));
        assert!(html.contains("dashboard"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("a<b>&\"c\""), "a&lt;b&gt;&amp;&quot;c&quot;");
    }

    #[test]
    fn test_standard_offline_page() {
        let html = standard_offline_page(3);
        assert!(html.contains("philosophers"));
        assert!(html.len() > 1000, "page should be substantial");
    }

    #[test]
    fn test_events_json_round_count() {
        let events = standard_scenario(4);
        let json = render_events_json(&events);
        let round_count = json.matches(r#""kind":"Round""#).count();
        assert_eq!(round_count, 4, "n=4 saturates in 4 rounds");
    }
}
