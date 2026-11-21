//! D6-1 DoD: a committed, self-contained HTML page, openable offline
//! without infrastructure, on the philosophers trace.

use lisage_liseuse::debugger::standard_scenario;
use lisage_liseuse::page::{render_offline_page, standard_offline_page};

/// DoD: the page is self-contained (no external URLs), contains the
/// trace data inline, and has navigation controls.
#[test]
fn test_d6_1_offline_page_self_contained() {
    let html = standard_offline_page(4);
    // No external resources: no http:// or https:// in src/href attributes.
    assert!(!html.contains(r#"src="http"#), "no external script");
    assert!(!html.contains(r#"href="http"#), "no external stylesheet");
    // Trace data is embedded inline, with full payload (not kind-only).
    assert!(html.contains("const TRACE = ["), "trace embedded");
    assert!(html.contains(r#""type":"Round""#), "round events embedded");
    assert!(html.contains(r#""round":1"#), "round payload embedded");
    assert!(html.contains(r#""fingerprint":"#), "op payload embedded");
    assert!(html.contains(r#""length":"#), "real index embedded");
    assert!(html.contains("const INDEX ="), "index embedded");
    assert!(html.contains("const SPANS ="), "spans embedded");
    // Navigation controls present.
    assert!(html.contains("step(-1)"), "back button");
    assert!(html.contains("step(1)"), "step button");
    assert!(html.contains("goto("), "goto control");
    assert!(html.contains("showDashboard()"), "dashboard button");
    assert!(html.contains("showOps()"), "ops button");
}

/// DoD: page on the philosophers trace (n=4) is substantial and mentions
/// the scenario.
#[test]
fn test_d6_1_philosophers_page() {
    let events = standard_scenario(4);
    let html = render_offline_page(&events, "philosophers n=4");
    assert!(html.contains("philosophers n=4"));
    assert!(
        html.len() > 5000,
        "page should embed the full trace, got {} bytes",
        html.len()
    );
    // 4 rounds in the n=4 saturation.
    assert!(html.contains("rounds: 4"), "dashboard shows 4 rounds");
}

/// The page is valid HTML (starts with doctype, has html/head/body).
#[test]
fn test_d6_1_valid_html_structure() {
    let html = standard_offline_page(3);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<html"));
    assert!(html.contains("<head>"));
    assert!(html.contains("<body>"));
    assert!(html.contains("</html>"));
}

/// DoD (issue #174): the page is COMMITTED as a self-contained artifact,
/// openable offline on the philosophers trace — `crates/lisage-liseuse/offline/philosophers-n4.html`
/// inside the liseuse crate.
#[test]
fn test_d6_1_committed_page_artifact() {
    let artifact = concat!(env!("CARGO_MANIFEST_DIR"), "/offline/philosophers-n4.html");
    let html = std::fs::read_to_string(artifact).expect("committed page artifact exists");
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(!html.contains(r#"src="http"#), "no external script");
    assert!(!html.contains(r#"href="http"#), "no external stylesheet");
    assert!(html.contains("const TRACE = ["), "trace embedded");
    assert!(html.contains(r#""type":"Round""#), "round events embedded");
    // The n=4 philosophers saturation embeds exactly 4 rounds.
    assert_eq!(html.matches(r#""type":"Round""#).count(), 4);
}
