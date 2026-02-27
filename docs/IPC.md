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

Response (Sprint 1 mock):

```json
{
  "reply": "Sprint 1 scaffold active...",
  "mode": "mock"
}
```

## WebSocket events (Sprint 1)

### GET /v1/events (WebSocket)
Sends lightweight lifecycle events:
- `event:connected`
- `event:heartbeat` (every 15s)

## Next
- Add tool-call envelope format
- Add permission prompt event schema
- Move to structured JSON event payloads
