//! D6-1 — Offline HTML page generation: embeds the full trace (every event
//! with its payload, as JSON), the index summary and the span-tree summary
//! into a single self-contained HTML file, navigable offline with no
//! infrastructure. Buttons map to the REPL command bus verbs.

use crate::debugger::standard_scenario;
use crate::index::IndexTrace;
use crate::spans::SpanTree;
use lisage_core::NodeId;
use lisage_core::observer::{SpanId, TraceEvent};

/// Render a self-contained HTML page for the given trace.
///
/// The page embeds the trace events as a JSON array (one object per event,
/// with all payload fields), the pre-built index summary, and navigation
/// controls (step / back / goto / dashboard / ops). All data is inline;
/// no network request is needed.
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
  const rounds = TRACE.filter(e => e.type === 'Round');
  document.getElementById('dashboard').textContent = rounds.map(r =>
    'round ' + r.round + ' | active: ' + JSON.stringify(r.active_levels) +
    ' | modified: ' + JSON.stringify(r.modified_levels) +
    ' | growth: ' + r.growth + (r.before === r.after ? ' | FIXPOINT' : '')
  ).join('\n');
}}
function showOps() {{
  const ops = TRACE.filter(e => e.type === 'OpStart').map(e => e.fingerprint);
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

/// Escape a string for inclusion in a JSON string literal.
fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out
}

/// JSON for an optional span: `null` or the span identifier.
fn span_json(span: Option<SpanId>) -> String {
    match span {
        Some(span) => span.0.to_string(),
        None => "null".to_string(),
    }
}

/// JSON array of u32 values.
fn u32_array(values: &[u32]) -> String {
    let items: Vec<String> = values.iter().map(u32::to_string).collect();
    format!("[{}]", items.join(","))
}

/// JSON array of node identifiers.
fn node_array(nodes: &[NodeId]) -> String {
    let items: Vec<String> = nodes.iter().map(|node| node.0.to_string()).collect();
    format!("[{}]", items.join(","))
}

/// JSON array of arcs as `[arc, target]` pairs.
fn arcs_json(arcs: &[(u64, NodeId)]) -> String {
    let items: Vec<String> = arcs
        .iter()
        .map(|(arc, target)| format!("[{},{}]", arc, target.0))
        .collect();
    format!("[{}]", items.join(","))
}

/// Render trace events as a JSON array — one object per event, with the
/// `"type"` tag and every payload field (English names).
fn render_events_json(events: &[TraceEvent]) -> String {
    let items: Vec<String> = events.iter().map(event_json).collect();
    format!("[{}]", items.join(","))
}

/// Serialize one event as a JSON object with its full payload.
fn event_json(event: &TraceEvent) -> String {
    match event {
        TraceEvent::SpanOpen { span, kind, lane } => format!(
            r#"{{"type":"SpanOpen","span":{},"kind":"{:?}","lane":{}}}"#,
            span.0, kind, lane
        ),
        TraceEvent::SpanClose { span } => {
            format!(r#"{{"type":"SpanClose","span":{}}}"#, span.0)
        }
        TraceEvent::NodeCreated {
            node,
            level,
            form,
            span,
        } => {
            let level_json = match level {
                Some(level) => level.0.to_string(),
                None => "null".to_string(),
            };
            format!(
                r#"{{"type":"NodeCreated","node":{},"level":{},"form":"{}","span":{}}}"#,
                node.0,
                level_json,
                json_escape(&form.render()),
                span_json(*span)
            )
        }
        TraceEvent::NodeShared { node, span } => format!(
            r#"{{"type":"NodeShared","node":{},"span":{}}}"#,
            node.0,
            span_json(*span)
        ),
        TraceEvent::ReductionApplied { rule, level, span } => format!(
            r#"{{"type":"ReductionApplied","rule":"{:?}","level":{},"span":{}}}"#,
            rule,
            level.0,
            span_json(*span)
        ),
        TraceEvent::DiagramBuilt {
            handle,
            nodes,
            span,
        } => format!(
            r#"{{"type":"DiagramBuilt","controller":{},"anchor":{},"node":{},"nodes":{},"span":{}}}"#,
            handle.controller.0,
            handle.anchor.0,
            handle.node.0,
            nodes,
            span_json(*span)
        ),
        TraceEvent::Attr {
            channel,
            node,
            value,
            span,
        } => format!(
            r#"{{"type":"Attr","channel":"{:?}","node":{},"value":"{}","span":{}}}"#,
            channel,
            node.0,
            json_escape(&value.render()),
            span_json(*span)
        ),
        TraceEvent::MemoHit { node, span } => format!(
            r#"{{"type":"MemoHit","node":{},"span":{}}}"#,
            node.0,
            span_json(*span)
        ),
        TraceEvent::MemoMiss { node, span } => format!(
            r#"{{"type":"MemoMiss","node":{},"span":{}}}"#,
            node.0,
            span_json(*span)
        ),
        TraceEvent::Gc {
            live,
            collected,
            compaction,
            collected_list,
            span,
        } => format!(
            r#"{{"type":"Gc","live":{},"collected":{},"compaction":{},"collected_nodes":{},"span":{}}}"#,
            live,
            collected,
            compaction,
            node_array(collected_list),
            span_json(*span)
        ),
        TraceEvent::Round {
            number,
            active_levels,
            modified_levels,
            before,
            image,
            after,
            growth,
            span,
        } => format!(
            r#"{{"type":"Round","round":{},"active_levels":{},"modified_levels":{},"before":{},"image":{},"after":{},"growth":{},"span":{}}}"#,
            number,
            u32_array(active_levels),
            u32_array(modified_levels),
            before.0,
            image.0,
            after.0,
            growth,
            span_json(*span)
        ),
        TraceEvent::Histogram { classes, span } => {
            let items: Vec<String> = classes
                .iter()
                .map(|entry| {
                    format!(
                        r#"{{"class":{},"live":{},"peak":{},"slack":{}}}"#,
                        entry.class, entry.live, entry.peak, entry.slack
                    )
                })
                .collect();
            format!(
                r#"{{"type":"Histogram","classes":[{}],"span":{}}}"#,
                items.join(","),
                span_json(*span)
            )
        }
        TraceEvent::OpStart { fingerprint, span } => format!(
            r#"{{"type":"OpStart","fingerprint":{},"span":{}}}"#,
            fingerprint,
            span_json(*span)
        ),
        TraceEvent::OpEnd { span } => {
            format!(r#"{{"type":"OpEnd","span":{}}}"#, span_json(*span))
        }
        TraceEvent::VisitEntry { node, span } => format!(
            r#"{{"type":"VisitEnter","node":{},"span":{}}}"#,
            node.0,
            span_json(*span)
        ),
        TraceEvent::VisitExit { node, result, span } => format!(
            r#"{{"type":"VisitExit","node":{},"result":{},"span":{}}}"#,
            node.0,
            result.0,
            span_json(*span)
        ),
        TraceEvent::MemoConsult {
            fingerprint,
            node,
            h,
            version,
            hit,
            span,
        } => format!(
            r#"{{"type":"MemoConsult","fingerprint":{},"node":{},"h":"{}","version":{},"hit":{},"span":{}}}"#,
            fingerprint,
            node.0,
            json_escape(&h.render()),
            version,
            hit,
            span_json(*span)
        ),
        TraceEvent::MemoInsert {
            fingerprint,
            node,
            h,
            version,
            result,
            span,
        } => format!(
            r#"{{"type":"MemoInsert","fingerprint":{},"node":{},"h":"{}","version":{},"result":{},"span":{}}}"#,
            fingerprint,
            node.0,
            json_escape(&h.render()),
            version,
            result.0,
            span_json(*span)
        ),
        TraceEvent::MemoryWrite {
            version_before,
            version_after,
            span,
        } => format!(
            r#"{{"type":"MemoryWrite","version_before":{},"version_after":{},"span":{}}}"#,
            version_before,
            version_after,
            span_json(*span)
        ),
        TraceEvent::StructureChange {
            node,
            classification,
            arcs_before,
            arcs_after,
            span,
        } => format!(
            r#"{{"type":"StructureChange","node":{},"classification":"{:?}","arcs_before":{},"arcs_after":{},"span":{}}}"#,
            node.0,
            classification,
            arcs_json(arcs_before),
            arcs_json(arcs_after),
            span_json(*span)
        ),
    }
}

/// Render the index summary as a JSON object, from the real built index.
fn render_index_json(index: &IndexTrace) -> String {
    format!(
        r#"{{"length":{},"nodes":{},"rounds":{},"ops":{}}}"#,
        index.length(),
        index.node_count(),
        index.round_count(),
        index.op_count()
    )
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
    fn test_json_escape() {
        assert_eq!(json_escape("a\"b\\c\n"), "a\\\"b\\\\c\\n");
        assert_eq!(json_escape("</script>"), "\\u003c/script\\u003e");
    }

    #[test]
    fn test_standard_offline_page() {
        let html = standard_offline_page(3);
        assert!(html.contains("philosophers"));
        assert!(html.len() > 1000, "page should be substantial");
    }

    #[test]
    fn test_events_json_carries_payload() {
        let events = standard_scenario(4);
        let json = render_events_json(&events);
        let round_count = json.matches(r#""type":"Round""#).count();
        assert_eq!(round_count, 4, "n=4 saturates in 4 rounds");
        // Events are not kind-only stubs: payload fields are present.
        assert!(json.contains(r#""round":1"#), "round payload present");
        assert!(json.contains(r#""active_levels":"#), "round levels present");
        assert!(json.contains(r#""fingerprint":"#), "op payload present");
        assert!(json.contains(r#""growth":"#), "growth present");
    }

    #[test]
    fn test_index_json_uses_real_index() {
        let events = standard_scenario(4);
        let index = IndexTrace::build(&events);
        let json = render_index_json(&index);
        assert_eq!(
            json,
            format!(
                r#"{{"length":{},"nodes":{},"rounds":{},"ops":{}}}"#,
                events.len(),
                index.node_count(),
                index.round_count(),
                index.op_count()
            )
        );
        assert!(json.contains(r#""rounds":4"#));
        assert!(json.contains(r#""ops":4"#));
    }
}
