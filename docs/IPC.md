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

## Next
- Add event stream for action previews
- Add tool-call envelope format
- Add permission prompt event schema
