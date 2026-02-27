# IPC Contract (Sprint 1)

Base transport: localhost HTTP (WebSocket event stream in next sprint).

## Endpoints

### GET /health
Returns daemon health.

```json
{
  "service": "AI-OS Core Daemon",
  "status": "ok"
}
```

### POST /v1/chat
Request:

```json
{
  "message": "open downloads"
}
```

Response (Sprint 1 mock + tool preview envelope):

```json
{
  "reply": "Sprint 1 scaffold active...",
  "mode": "mock",
  "tool_preview": {
    "name": "open_app_or_file",
    "risk_tier": 1,
    "requires_confirmation": false,
    "note": "Low-risk open action..."
  }
}
```

## WebSocket events (Sprint 1)

### GET /v1/events (WebSocket)
Sends lightweight lifecycle events as JSON:

```json
{
  "event": "heartbeat",
  "at": "2026-02-27T06:23:40.120Z",
  "source": "core-daemon"
}
```

## Next
- Add tool-call envelope format
- Add permission prompt event schema
- Move to structured JSON event payloads
