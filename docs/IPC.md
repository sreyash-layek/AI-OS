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

### POST /v1/speak
Request:

```json
{
  "text": "Hello from AI-OS",
  "voice": "system-default"
}
```

Response:

```json
{
  "ok": true,
  "request_id": "<uuid>",
  "mode": "mock"
}
```

### POST /v1/speak/stop
Response:

```json
{ "ok": true }
```

Speech lifecycle events are emitted over `/v1/events`:
- `speech_started`
- `speech_stopped`

### GET /v1/config/voice
Returns voice settings:

```json
{
  "provider": "mock",
  "auto_speak": false,
  "default_voice": "system-default"
}
```

### POST /v1/config/voice
Partial update payload:

```json
{
  "auto_speak": true
}
```

### GET /v1/config/voice/health
Returns provider health resolution:

```json
{
  "configured_provider": "system",
  "effective_provider": "mock",
  "available": false,
  "detail": "system requested, but spd-say not found; using mock"
}
```

## Notes
- `mock` provider simulates duration based on text length.
- `system` provider routing:
  - Linux: `spd-say`
  - macOS: `say`
  - Windows: PowerShell + .NET SpeechSynthesizer
- If requested system provider is unavailable, daemon falls back to `mock` and reports this in `/v1/config/voice/health`.

## Next
- Add tool-call envelope format v2 with policy hints
- Add permission prompt event schema
- Add native TTS provider wiring (non-mock)
