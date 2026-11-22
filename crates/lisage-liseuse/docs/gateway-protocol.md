# D6-2 loopback gateway — API contract (ADR-0016)

The gateway (`lisage-gateway <trace.jsonl> [title]`) binds **127.0.0.1 only**
(never 0.0.0.0) on an ephemeral port with one random 32-hex-char token per
run. **No secret ever appears in a URL**: HTTP uses `Authorization: Bearer`,
the WebSocket authenticates on its first text frame.

## Endpoints

| Method | Path | Auth | Success | Error |
| ------ | ---- | ---- | ------- | ----- |
| GET | `/` | `Authorization: Bearer <token>` | `200` `text/html` live page (token embedded server-side) | `403` empty body |
| GET | `/ws` | first WS frame `{"cmd":"auth","token":"<token>"}` | `101` upgrade, then `{"event":"auth_ok"}` | `{"event":"auth_failed"}` then close |

## WebSocket protocol (JSON text frames)

Client → server commands:

| Command | Payload | Response events |
| ------- | ------- | --------------- |
| `auth` | `{"cmd":"auth","token":"..."}` (first frame only) | `auth_ok` / `auth_failed` |
| `step` / `next` | `{"cmd":"step"}` | `event` + `position`, or `break` + `position`, or `done` |
| `back` | `{"cmd":"back"}` | `position` |
| `goto` | `{"cmd":"goto","position":N}` (clamped to `[0, length]`) | `position` |
| `break` | `{"cmd":"break","round":N}` | `break_set` |
| `pause` | `{"cmd":"pause"}` | `paused` |
| `continue` / `resume` | `{"cmd":"continue"}` | runs to the break round or the end: `break` + `position`, or `done` |
| `status` | `{"cmd":"status"}` | `status` with `position`, `length`, `paused`, `break_round` |

Server → client events: `position`, `event` (with `kind`), `break`,
`break_set`, `paused`, `done`, `status`, `auth_ok` / `auth_failed` — all as
`{"event":"...", ...}` JSON objects with named fields.

## Errors

Typed `GatewayError::{Io, Trace}` at the library boundary (entry point prints
to stderr, exit 1); per-connection failures go through
`GatewayEvents::connection_error`.
