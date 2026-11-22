//! D6-3 DoD: VS Code -- breakpoint " round 12 " operational on
//! philosophers. We simulate the DAP message exchange that VS Code would
//! send and assert the breakpoint at round 12 is hit.

use lisage_liseuse::dap::{DapServer, json_to_string, read_frame};
use lisage_liseuse::json::Json;

fn request(seq: i64, command: &str, args: &str) -> Json {
    lisage_liseuse::json::parse(&format!(
        r#"{{"seq":{seq},"type":"request","command":"{command}","arguments":{args}}}"#
    ))
    .unwrap()
}

/// DoD: full DAP session -- initialize → launch → setBreakpoints(round 12)
/// → configurationDone → continue → stopped at breakpoint round 12.
#[test]
fn test_d6_3_dod_breakpoint_round_12() {
    let mut server = DapServer::new();

    // 1. initialize
    let frames = server.handle(&request(1, "initialize", "{}"));
    assert!(frames[0].contains(r#""success":true"#));

    // 2. launch
    let frames = server.handle(&request(2, "launch", r#"{"program":"philosophers"}"#));
    assert!(frames.iter().any(|f| f.contains(r#""command":"launch""#)));
    assert!(
        frames
            .iter()
            .any(|f| f.contains(r#""event":"initialized""#))
    );

    // 3. setBreakpoints: round 12 on philosophers source
    let frames = server.handle(&request(
        3,
        "setBreakpoints",
        r#"{"source":{"name":"philosophers"},"breakpoints":[{"line":12}]}"#,
    ));
    assert!(
        frames[0].contains(r#""verified":true"#),
        "breakpoint verified"
    );

    // 4. configurationDone
    let frames = server.handle(&request(4, "configurationDone", "{}"));
    assert!(frames.iter().any(|f| f.contains(r#""event":"stopped""#)));

    // 5. continue → must stop at breakpoint round 12
    let frames = server.handle(&request(5, "continue", "{}"));
    let all = frames.join("");
    assert!(
        all.contains(r#""reason":"breakpoint""#),
        "must stop at breakpoint: {all}"
    );

    // 6. stackTrace at the breakpoint
    let frames = server.handle(&request(6, "stackTrace", r#"{"threadId":1}"#));
    assert!(
        frames[0].contains("stackFrames"),
        "stackTrace returns frames"
    );

    // 7. variables at the breakpoint
    let frames = server.handle(&request(7, "variables", r#"{"variablesReference":1}"#));
    assert!(frames[0].contains("position"));
    assert!(frames[0].contains("round"));

    // 8. next (step) works after breakpoint
    let frames = server.handle(&request(8, "next", r#"{"threadId":1}"#));
    assert!(frames.iter().any(|f| f.contains(r#""reason":"step""#)));
}

/// DAP frame parsing: a well-formed Content-Length frame is parsed.
#[test]
fn test_d6_3_frame_roundtrip() {
    let body = r#"{"seq":1,"type":"request","command":"initialize"}"#;
    let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let mut reader = std::io::BufReader::new(frame.as_bytes());
    let msg = read_frame(&mut reader).unwrap();
    assert_eq!(
        msg.get("command").and_then(|c| c.as_str()),
        Some("initialize")
    );
}

/// json_to_string produces valid parseable JSON.
#[test]
fn test_d6_3_json_serialization() {
    let obj = Json::Obj(vec![
        ("key".to_string(), Json::Str("value".to_string())),
        ("num".to_string(), Json::Num(42)),
    ]);
    let s = json_to_string(&obj);
    let parsed = lisage_liseuse::json::parse(&s).unwrap();
    assert_eq!(parsed.get("key").and_then(|v| v.as_str()), Some("value"));
    assert_eq!(parsed.get("num").and_then(|v| v.as_num()), Some(42));
}

/// Unknown commands get a failure response, not a crash.
#[test]
fn test_d6_3_unknown_command_safe() {
    let mut server = DapServer::new();
    let frames = server.handle(&request(1, "teleport", "{}"));
    assert!(frames[0].contains(r#""success":false"#));
}
