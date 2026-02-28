# INDEXING API (Sprint 3–4)

## Scopes

### List scopes
`GET /v1/index/scopes`

Response:
```json
{ "scopes": [{ "id":"...", "path":"C:/...", "enabled":true, "created_at":"..." }] }
```

### Create scope
`POST /v1/index/scopes`

Body:
```json
{ "path":"C:/Users/sreya/Documents", "enabled": true }
```

### Update scope (pause/resume watcher)
`PATCH /v1/index/scopes/{id}`

Body:
```json
{ "enabled": false }
```

### Delete scope
`DELETE /v1/index/scopes/{id}`

---

## Index Events (watcher/manual ingestion)

### Ingest event
`POST /v1/index/events`

Body:
```json
{
  "scope_id": "...",
  "path": "C:/Users/.../notes.md",
  "event_type": "create",
  "size_bytes": 123,
  "mtime": "2026-02-28T00:00:00Z",
  "renamed_from": null
}
```

`event_type` values: `create | update | delete | rename`

---

## Search

### Hybrid search (keyword-first, metadata fallback)
`GET /v1/search?q=notes`

Returns:
- keyword hits over extracted content/title/path for supported files (`.txt`, `.md`, `.pdf`)
- snippet previews (`snippet_html`) for content matches
- relevance score (`score`) for keyword hits
- metadata path matches when no keyword hits are available

Example result item:
```json
{
  "scope_id": "...",
  "path": "C:/Users/.../notes.md",
  "match_reason": "keyword_content_match",
  "title": "notes.md",
  "snippet_html": "...<b>notes</b>...",
  "score": 12.34
}
```

---

## WebSocket events

- `index_scope_added`
- `index_scope_updated`
- `index_scope_removed`
- `index_event_applied` (single event)
- `index_batch_applied` (debounced watcher batch with counts)

---

## Local validation checklist

1. Create scope from UI/API.
2. Confirm watcher starts (`index_batch_applied` appears when files change).
3. Pause scope and verify watcher updates stop.
4. Resume scope and verify updates continue.
5. Rename a file and verify rename count increments in batch stats.
6. Search by file path fragment and verify result appears.
