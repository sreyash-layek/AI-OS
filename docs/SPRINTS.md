# Sprints Plan (Execution)

## Sprint 1 (in progress)
- [x] Rust daemon skeleton
- [x] Basic IPC endpoints
- [x] Desktop app scaffold folder
- [x] IPC documentation
- [x] Vite desktop shell UI (futuristic launcher prototype)
- [x] Daemon health badge in UI
- [x] WebSocket event stub (daemon -> UI)
- [x] Tool preview envelope stub (`/v1/chat`)
- [x] UI action preview panel
- [x] Structured JSON websocket events
- [x] UI live event timeline panel
- [x] Tauri wrapper bootstrap

## Sprint 2 (in progress)
- [x] Daemon speech endpoints (`/v1/speak`, `/v1/speak/stop`)
- [x] UI speak/stop controls wired to API
- [x] Speech lifecycle events in websocket timeline
- [x] Voice config API (`provider`, `auto_speak`, `default_voice`)
- [x] Voice provider health endpoint
- [x] UI toggle for auto-speak setting
- [~] Native TTS adapters (Linux fallback via `spd-say`, Win/macOS pending)
- [ ] Voice toggles and interruption refinements

## Sprint 3
- Folder opt-in
- File watcher + metadata store
