//! D6-3 -- DAP (Debug Adapter Protocol) server over stdio. Maps the lisage trace debugger to DAP concepts:
//! - `stackTrace` -> `Debugger::visit_stack()` (visit stack)
//! - `variables` -> counters per frame (position/length/live/round)
//! - `setBreakpoints` -> AST serialized: breakpoint "round 12" on the source
//!   `philosophers`
//!
//! DoD: VS Code -- breakpoint " round 12 " operational on philosophers.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::debugger::{Command, Debugger, standard_scenario};
use crate::json::Json;
use lisage_core::observer::TraceEvent;

/// Serialize a Json value to a compact string.
pub fn json_to_string(value: &Json) -> String {
    match value {
        Json::Null => "null".to_string(),
        Json::Bool(b) => b.to_string(),
        Json::Num(n) => n.to_string(),
        Json::Str(s) => {
            let mut out = String::with_capacity(s.len() + 2);
            out.push('"');
            for character in s.chars() {
                match character {
                    '\\' => out.push_str("\\\\"),
                    '"' => out.push_str("\\\""),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                    c => out.push(c),
                }
            }
            out.push('"');
            out
        }
        Json::Arr(items) => format!(
            "[{}]",
            items
                .iter()
                .map(json_to_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Json::Obj(entries) => format!(
            "{{{}}}",
            entries
                .iter()
                .map(|(k, v)| format!(r#""{}":{}"#, k, json_to_string(v)))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

/// Parse one DAP frame from a reader. Returns None on EOF.
pub fn read_frame<R: BufRead>(reader: &mut R) -> Option<Json> {
    let mut content_length: Option<usize> = None;
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return None,
            Ok(_) => {}
            Err(_) => return None,
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if content_length.is_some() {
                break;
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse().ok();
        }
    }
    let length = content_length?;
    let mut buf = vec![0u8; length];
    reader.read_exact(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);
    crate::json::parse(&text).ok()
}

/// The DAP server state: one debugger + breakpoints.
pub struct DapServer {
    /// Sequence counter for outgoing messages.
    seq: i64,
    /// The trace debugger.
    debugger: Debugger,
    /// Breakpoints: source name → set of line numbers (round numbers).
    breakpoints: BTreeMap<String, Vec<i64>>,
    /// Program running.
    running: Arc<AtomicBool>,
}

impl Default for DapServer {
    fn default() -> Self {
        Self::new()
    }
}

impl DapServer {
    /// New server over the standard philosophers scenario (n=13 so that
    /// round 12 exists for the DoD breakpoint).
    pub fn new() -> Self {
        Self::with_scenario(13)
    }

    /// New server over the standard scenario of the given size.
    pub fn with_scenario(n: u64) -> Self {
        DapServer {
            seq: 0,
            debugger: Debugger::new(standard_scenario(n)),
            breakpoints: BTreeMap::new(),
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Next sequence number.
    fn next_seq(&mut self) -> i64 {
        self.seq += 1;
        self.seq
    }

    /// Build a response message.
    fn response(&mut self, request_seq: i64, command: &str, success: bool, body: Json) -> String {
        let seq = self.next_seq();
        let mut entries = vec![
            ("seq".to_string(), Json::Num(seq)),
            ("type".to_string(), Json::Str("response".to_string())),
            ("request_seq".to_string(), Json::Num(request_seq)),
            ("command".to_string(), Json::Str(command.to_string())),
            ("success".to_string(), Json::Bool(success)),
        ];
        if body != Json::Null {
            entries.push(("body".to_string(), body));
        }
        let json = json_to_string(&Json::Obj(entries));
        format!("Content-Length: {}\r\n\r\n{}", json.len(), json)
    }

    /// Build an event message.
    fn event(&mut self, name: &str, body: Json) -> String {
        let seq = self.next_seq();
        let entries = vec![
            ("seq".to_string(), Json::Num(seq)),
            ("type".to_string(), Json::Str("event".to_string())),
            ("event".to_string(), Json::Str(name.to_string())),
            ("body".to_string(), body),
        ];
        let json = json_to_string(&Json::Obj(entries));
        format!("Content-Length: {}\r\n\r\n{}", json.len(), json)
    }

    /// Handle one request, return the response frame(s) to write.
    pub fn handle(&mut self, request: &Json) -> Vec<String> {
        let request_seq = request.get("seq").and_then(|s| s.as_num()).unwrap_or(0);
        let command = request
            .get("command")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let args = request.get("arguments").cloned().unwrap_or(Json::Null);
        match command {
            "initialize" => vec![self.response(
                request_seq,
                "initialize",
                true,
                Json::Obj(vec![
                    (
                        "supportsConfigurationDoneRequest".to_string(),
                        Json::Bool(true),
                    ),
                    ("supportsVariableType".to_string(), Json::Bool(true)),
                    ("supportsContinueRequest".to_string(), Json::Bool(true)),
                    ("supportsPauseRequest".to_string(), Json::Bool(true)),
                ]),
            )],
            "launch" => {
                self.running.store(true, Ordering::SeqCst);
                vec![
                    self.response(request_seq, "launch", true, Json::Null),
                    self.event("initialized", Json::Obj(vec![])),
                ]
            }
            "setBreakpoints" => {
                let source = args
                    .get("source")
                    .and_then(|s| s.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("philosophers")
                    .to_string();
                let lines: Vec<i64> = args
                    .get("breakpoints")
                    .and_then(|b| b.as_arr())
                    .map(|requested| {
                        requested
                            .iter()
                            .filter_map(|breakpoint| {
                                breakpoint.get("line").and_then(|l| l.as_num())
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.breakpoints.insert(source, lines.clone());
                let length = self.debugger.length() as i64;
                let verified: Vec<Json> = lines
                    .iter()
                    .map(|l| {
                        Json::Obj(vec![
                            ("line".to_string(), Json::Num(*l)),
                            ("verified".to_string(), Json::Bool((1..=length).contains(l))),
                        ])
                    })
                    .collect();
                vec![self.response(
                    request_seq,
                    "setBreakpoints",
                    true,
                    Json::Obj(vec![("breakpoints".to_string(), Json::Arr(verified))]),
                )]
            }
            "configurationDone" => {
                vec![
                    self.response(request_seq, "configurationDone", true, Json::Null),
                    self.event(
                        "stopped",
                        Json::Obj(vec![
                            ("reason".to_string(), Json::Str("entry".to_string())),
                            ("threadId".to_string(), Json::Num(1)),
                        ]),
                    ),
                ]
            }
            "threads" => vec![self.response(
                request_seq,
                "threads",
                true,
                Json::Obj(vec![(
                    "threads".to_string(),
                    Json::Arr(vec![Json::Obj(vec![
                        ("id".to_string(), Json::Num(1)),
                        ("name".to_string(), Json::Str("main".to_string())),
                    ])]),
                )]),
            )],
            "stackTrace" => {
                let stack = self.debugger.visit_stack();
                let frames: Vec<Json> = stack
                    .iter()
                    .enumerate()
                    .map(|(i, span)| {
                        Json::Obj(vec![
                            ("id".to_string(), Json::Num(i as i64 + 1)),
                            ("name".to_string(), Json::Str(format!("span {span:?}"))),
                            (
                                "source".to_string(),
                                Json::Obj(vec![(
                                    "name".to_string(),
                                    Json::Str("philosophers".to_string()),
                                )]),
                            ),
                            (
                                "line".to_string(),
                                Json::Num(self.debugger.position() as i64),
                            ),
                            ("column".to_string(), Json::Num(1)),
                        ])
                    })
                    .collect();
                // Always at least one frame.
                let frames = if frames.is_empty() {
                    vec![Json::Obj(vec![
                        ("id".to_string(), Json::Num(1)),
                        ("name".to_string(), Json::Str("main".to_string())),
                        (
                            "source".to_string(),
                            Json::Obj(vec![(
                                "name".to_string(),
                                Json::Str("philosophers".to_string()),
                            )]),
                        ),
                        (
                            "line".to_string(),
                            Json::Num(self.debugger.position() as i64),
                        ),
                        ("column".to_string(), Json::Num(1)),
                    ])]
                } else {
                    frames
                };
                let total_frames = frames.len() as i64;
                vec![self.response(
                    request_seq,
                    "stackTrace",
                    true,
                    Json::Obj(vec![
                        ("stackFrames".to_string(), Json::Arr(frames)),
                        ("totalFrames".to_string(), Json::Num(total_frames)),
                    ]),
                )]
            }
            "scopes" => vec![self.response(
                request_seq,
                "scopes",
                true,
                Json::Obj(vec![(
                    "scopes".to_string(),
                    Json::Arr(vec![Json::Obj(vec![
                        ("name".to_string(), Json::Str("Trace counters".to_string())),
                        ("variablesReference".to_string(), Json::Num(1)),
                        ("expensive".to_string(), Json::Bool(false)),
                    ])]),
                )]),
            )],
            "variables" => {
                let position = self.debugger.position();
                let length = self.debugger.length();
                let round = self
                    .debugger
                    .current_event()
                    .and_then(|e| match e {
                        TraceEvent::Round { numero, .. } => Some(*numero as i64),
                        _ => None,
                    })
                    .unwrap_or(0);
                let vars = vec![
                    Json::Obj(vec![
                        ("name".to_string(), Json::Str("position".to_string())),
                        ("value".to_string(), Json::Num(position as i64)),
                        ("type".to_string(), Json::Str("usize".to_string())),
                        ("variablesReference".to_string(), Json::Num(0)),
                    ]),
                    Json::Obj(vec![
                        ("name".to_string(), Json::Str("length".to_string())),
                        ("value".to_string(), Json::Num(length as i64)),
                        ("type".to_string(), Json::Str("usize".to_string())),
                        ("variablesReference".to_string(), Json::Num(0)),
                    ]),
                    Json::Obj(vec![
                        ("name".to_string(), Json::Str("round".to_string())),
                        ("value".to_string(), Json::Num(round)),
                        ("type".to_string(), Json::Str("u32".to_string())),
                        ("variablesReference".to_string(), Json::Num(0)),
                    ]),
                    Json::Obj(vec![
                        ("name".to_string(), Json::Str("visit_depth".to_string())),
                        (
                            "value".to_string(),
                            Json::Num(self.debugger.visit_stack().len() as i64),
                        ),
                        ("type".to_string(), Json::Str("usize".to_string())),
                        ("variablesReference".to_string(), Json::Num(0)),
                    ]),
                ];
                vec![self.response(
                    request_seq,
                    "variables",
                    true,
                    Json::Obj(vec![("variables".to_string(), Json::Arr(vars))]),
                )]
            }
            "continue" => {
                // Run until the next breakpoint or end.
                let breakpoints = self
                    .breakpoints
                    .get("philosophers")
                    .cloned()
                    .unwrap_or_default();
                let mut stopped_reason = "completed";
                let mut stopped_round: Option<i64> = None;
                loop {
                    let position = self.debugger.position();
                    if position >= self.debugger.length() {
                        break;
                    }
                    // Check if current event is a round matching a breakpoint.
                    if let Some(TraceEvent::Round { numero, .. }) = self.debugger.current_event()
                        && breakpoints.contains(&(*numero as i64))
                    {
                        stopped_reason = "breakpoint";
                        stopped_round = Some(*numero as i64);
                        break;
                    }
                    self.debugger.run(Command::Step);
                }
                let position = self.debugger.position();
                vec![
                    self.response(
                        request_seq,
                        "continue",
                        true,
                        Json::Obj(vec![("allThreadsContinued".to_string(), Json::Bool(true))]),
                    ),
                    self.event(
                        "stopped",
                        Json::Obj(vec![
                            ("reason".to_string(), Json::Str(stopped_reason.to_string())),
                            ("threadId".to_string(), Json::Num(1)),
                            (
                                "description".to_string(),
                                Json::Str(match stopped_round {
                                    Some(round) => {
                                        format!("breakpoint at round {round} (position {position})")
                                    }
                                    None => format!("position {position}"),
                                }),
                            ),
                        ]),
                    ),
                ]
            }
            "next" => {
                self.debugger.run(Command::Step);
                let position = self.debugger.position();
                vec![
                    self.response(request_seq, "next", true, Json::Null),
                    self.event(
                        "stopped",
                        Json::Obj(vec![
                            ("reason".to_string(), Json::Str("step".to_string())),
                            ("threadId".to_string(), Json::Num(1)),
                            (
                                "description".to_string(),
                                Json::Str(format!("position {position}")),
                            ),
                        ]),
                    ),
                ]
            }
            "pause" => {
                vec![
                    self.response(request_seq, "pause", true, Json::Null),
                    self.event(
                        "stopped",
                        Json::Obj(vec![
                            ("reason".to_string(), Json::Str("pause".to_string())),
                            ("threadId".to_string(), Json::Num(1)),
                        ]),
                    ),
                ]
            }
            "disconnect" => {
                self.running.store(false, Ordering::SeqCst);
                vec![self.response(request_seq, "disconnect", true, Json::Null)]
            }
            _ => vec![self.response(
                request_seq,
                command,
                false,
                Json::Obj(vec![(
                    "message".to_string(),
                    Json::Str(format!("unknown command: {command}")),
                )]),
            )],
        }
    }

    /// Whether the program is still running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Run the DAP server on stdio.
pub fn run_stdio() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut server = DapServer::new();
    while let Some(msg) = read_frame(&mut reader) {
        if msg.get("type").and_then(|t| t.as_str()) == Some("request") {
            for frame in server.handle(&msg) {
                let _ = writer.write_all(frame.as_bytes());
                let _ = writer.flush();
            }
            if !server.is_running() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_string_roundtrip() {
        let obj = Json::Obj(vec![
            ("a".to_string(), Json::Num(1)),
            ("b".to_string(), Json::Str("hello".to_string())),
            (
                "c".to_string(),
                Json::Arr(vec![Json::Bool(true), Json::Null]),
            ),
        ]);
        let s = json_to_string(&obj);
        let parsed = crate::json::parse(&s).unwrap();
        assert_eq!(parsed.get("a").and_then(|v| v.as_num()), Some(1));
        assert_eq!(parsed.get("b").and_then(|v| v.as_str()), Some("hello"));
    }

    #[test]
    fn test_dap_initialize() {
        let mut server = DapServer::new();
        let req =
            crate::json::parse(r#"{"seq":1,"type":"request","command":"initialize"}"#).unwrap();
        let frames = server.handle(&req);
        assert_eq!(frames.len(), 1);
        assert!(frames[0].contains("Content-Length:"));
        assert!(frames[0].contains(r#""command":"initialize""#));
        assert!(frames[0].contains(r#""success":true"#));
        assert!(frames[0].contains("supportsConfigurationDoneRequest"));
    }

    #[test]
    fn test_dap_breakpoint_round_12() {
        let mut server = DapServer::new();
        // setBreakpoints: round 12 on philosophers.
        let req = crate::json::parse(
            r#"{"seq":2,"type":"request","command":"setBreakpoints","arguments":{"source":{"name":"philosophers"},"breakpoints":[{"line":12}]}}"#,
        )
        .unwrap();
        let frames = server.handle(&req);
        assert!(frames[0].contains(r#""verified":true"#));
        // continue → should stop at round 12.
        let req = crate::json::parse(r#"{"seq":3,"type":"request","command":"continue"}"#).unwrap();
        let frames = server.handle(&req);
        let all = frames.join("");
        assert!(
            all.contains(r#""event":"stopped""#),
            "stopped event expected: {all}"
        );
        assert!(
            all.contains(r#""reason":"breakpoint""#),
            "breakpoint reason expected: {all}"
        );
        // The stop must be exactly at round 12, not just any breakpoint hit.
        match server.debugger.current_event() {
            Some(TraceEvent::Round { numero, .. }) => {
                assert_eq!(*numero, 12, "continue must stop at breakpoint round 12")
            }
            other => panic!("expected a Round event at the breakpoint, got {other:?}"),
        }
    }

    #[test]
    fn test_dap_stack_trace_and_variables() {
        let mut server = DapServer::new();
        let req =
            crate::json::parse(r#"{"seq":1,"type":"request","command":"stackTrace"}"#).unwrap();
        let frames = server.handle(&req);
        assert!(frames[0].contains("stackFrames"));
        let body = frames[0].split("\r\n\r\n").nth(1).unwrap();
        let parsed = crate::json::parse(body).unwrap();
        let entries = match &parsed {
            Json::Obj(entries) => match entries.iter().find(|(k, _)| k == "body") {
                Some((_, Json::Obj(body))) => body.clone(),
                _ => entries.clone(),
            },
            _ => Vec::new(),
        };
        {
            let get = |key: &str| {
                entries
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.clone())
            };
            if let (Some(Json::Arr(stack_frames)), Some(Json::Num(total))) =
                (get("stackFrames"), get("totalFrames"))
            {
                assert_eq!(
                    total as usize,
                    stack_frames.len(),
                    "totalFrames must equal the number of stackFrames sent"
                );
            } else {
                panic!("stackTrace response must carry stackFrames and totalFrames");
            }
        }
        let req = crate::json::parse(
            r#"{"seq":2,"type":"request","command":"variables","arguments":{"variablesReference":1}}"#,
        )
        .unwrap();
        let frames = server.handle(&req);
        assert!(frames[0].contains("position"));
        assert!(frames[0].contains("length"));
        assert!(frames[0].contains("round"));
    }

    #[test]
    fn test_dap_next_step() {
        let mut server = DapServer::new();
        let req = crate::json::parse(r#"{"seq":1,"type":"request","command":"next"}"#).unwrap();
        let frames = server.handle(&req);
        let all = frames.join("");
        assert!(all.contains(r#""reason":"step""#));
        assert_eq!(server.debugger.position(), 1);
    }

    #[test]
    fn test_dap_unknown_command() {
        let mut server = DapServer::new();
        let req = crate::json::parse(r#"{"seq":1,"type":"request","command":"fly"}"#).unwrap();
        let frames = server.handle(&req);
        assert!(frames[0].contains(r#""success":false"#));
    }

    #[test]
    fn test_dap_frame_parsing() {
        let body = r#"{"seq":1,"type":"request","command":"initialize"}"#;
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let mut reader = std::io::BufReader::new(frame.as_bytes());
        let msg = read_frame(&mut reader).unwrap();
        assert_eq!(msg.get("seq").and_then(|s| s.as_num()), Some(1));
        assert_eq!(
            msg.get("command").and_then(|c| c.as_str()),
            Some("initialize")
        );
    }
}
