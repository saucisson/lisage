//! D6-3 -- DAP (Debug Adapter Protocol) server over stdio. Maps the lisage trace debugger to DAP concepts:
//! - `stackTrace` -> `Debugger::visit_stack()` (visit stack)
//! - `variables` -> counters per frame (position/length/live/round)
//! - `setBreakpoints` -> AST serialized: breakpoint "round 12" on the source
//!   `philosophers`
//!
//! DoD: VS Code -- breakpoint " round 12 " operational on philosophers.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::debugger::{Command, Debugger, standard_scenario};
use crate::json::Json;
use lisage_core::NodeId;
use lisage_core::observer::{ChannelAttr, NoOpObserver, Observer, TraceEvent, TraceVal};
use std::fmt;
use std::sync::Arc;

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

/// Typed DAP transport/parse error — causes preserved, nothing swallowed.
#[derive(Debug)]
pub enum DapError {
    /// Underlying I/O failure (stdin/stdout).
    Io(std::io::Error),
    /// A header line could not be parsed (bad Content-Length).
    MalformedHeader(String),
    /// The frame body is not valid JSON.
    MalformedBody(String),
}

impl fmt::Display for DapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DapError::Io(error) => write!(formatter, "I/O error: {error}"),
            DapError::MalformedHeader(line) => write!(formatter, "malformed DAP header: {line:?}"),
            DapError::MalformedBody(cause) => write!(formatter, "malformed DAP body: {cause}"),
        }
    }
}

impl std::error::Error for DapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DapError::Io(error) => Some(error),
            _ => None,
        }
    }
}

/// Narrate one DEBUG event through the Observer channel (ADR-0005/ADR-0100).
fn narrate(observer: &dyn Observer, message: String) {
    observer.on_event(&TraceEvent::Attr {
        channel: ChannelAttr::Synthesized,
        node: NodeId(0),
        value: TraceVal::texte(message),
        span: None,
    });
}

/// Parse one DAP frame from a reader. `Ok(None)` on clean EOF.
pub fn read_frame<R: BufRead>(reader: &mut R) -> Result<Option<Json>, DapError> {
    let mut content_length: Option<usize> = None;
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return Ok(None),
            Ok(_) => {}
            Err(error) => return Err(DapError::Io(error)),
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if content_length.is_some() {
                break;
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(
                rest.trim()
                    .parse()
                    .map_err(|_| DapError::MalformedHeader(trimmed.to_string()))?,
            );
        }
    }
    let length = content_length.expect("header loop breaks only with a length");
    let mut buf = vec![0u8; length];
    reader.read_exact(&mut buf).map_err(DapError::Io)?;
    let text = String::from_utf8_lossy(&buf);
    crate::json::parse(&text)
        .map(Some)
        .map_err(DapError::MalformedBody)
}

/// Default scenario size: n=13 so that round 12 exists for the DoD breakpoint.
const DEFAULT_SCENARIO_SIZE: u64 = 13;

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
    /// Narration channel (DEBUG events).
    observer: Arc<dyn Observer>,
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
        Self::with_scenario(DEFAULT_SCENARIO_SIZE)
    }

    /// New server over the standard scenario of the given size.
    pub fn with_scenario(n: u64) -> Self {
        Self::with_observer(n, Arc::new(NoOpObserver))
    }

    /// New server with an explicit narration observer.
    pub fn with_observer(n: u64, observer: Arc<dyn Observer>) -> Self {
        DapServer {
            seq: 0,
            debugger: Debugger::new(standard_scenario(n)),
            breakpoints: BTreeMap::new(),
            running: Arc::new(AtomicBool::new(true)),
            observer,
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
        narrate(
            self.observer.as_ref(),
            format!("dap dispatch command={command} seq={request_seq}"),
        );
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
                narrate(
                    self.observer.as_ref(),
                    "dap continue: run until breakpoint or end".to_string(),
                );
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
                    if let Some(TraceEvent::Round { numero, .. }) = self.debugger.current_event()
                        && breakpoints.contains(&(*numero as i64))
                    {
                        stopped_reason = "breakpoint";
                        stopped_round = Some(*numero as i64);
                        narrate(
                            self.observer.as_ref(),
                            format!("dap stopped at breakpoint round={numero}"),
                        );
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

/// Run the DAP server on stdio. Errors are returned, never swallowed;
/// the entry point prints them and exits non-zero.
pub fn run_stdio() -> Result<(), DapError> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut server = DapServer::new();
    loop {
        let msg = match read_frame(&mut reader) {
            Ok(Some(msg)) => msg,
            Ok(None) => break,
            Err(error) => {
                // Tell the client before tearing down the session.
                let frame = format!(
                    "Content-Length: {}\r\n\r\n{}",
                    r#"{"seq":0,"type":"response","success":false,"message":"malformed frame"}"#
                        .len(),
                    r#"{"seq":0,"type":"response","success":false,"message":"malformed frame"}"#
                );
                writer.write_all(frame.as_bytes()).map_err(DapError::Io)?;
                writer.flush().map_err(DapError::Io)?;
                return Err(error);
            }
        };
        if msg.get("type").and_then(|t| t.as_str()) == Some("request") {
            for frame in server.handle(&msg) {
                writer.write_all(frame.as_bytes()).map_err(DapError::Io)?;
                writer.flush().map_err(DapError::Io)?;
            }
            if !server.is_running() {
                break;
            }
        }
    }
    Ok(())
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
        let msg = read_frame(&mut reader).unwrap().unwrap();
        assert_eq!(msg.get("seq").and_then(|s| s.as_num()), Some(1));
        assert_eq!(
            msg.get("command").and_then(|c| c.as_str()),
            Some("initialize")
        );
    }
}
