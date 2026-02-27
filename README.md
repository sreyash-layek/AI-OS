# AI-OS

AI-OS is an open-source, local-first AI operating layer for **Windows, macOS, and Linux**.

It is not a new kernel. It is a cross-platform AI-native desktop layer with:
- Global launcher (text + voice)
- Local semantic file search ("File Brain")
- Safe actions (open apps/files, summarize, organize)
- Permission model ("AI sudo")
- Audit logs + rollback

## Project Goals

1. **Free-first development** (avoid paid infra until needed)
2. **Privacy-first defaults** (local-first, opt-in indexing)
3. **Trust-first UX** (action previews, confirmations, undo)
4. **Cross-platform parity** (Windows + macOS + Linux)

## Current Status

🚧 Sprint 1 scaffold in progress.

## Planned Architecture

- `apps/desktop` → Tauri desktop app (launcher/chat/voice UI)
- `crates/core-daemon` → Rust daemon (IPC, tools, policy, audit)
- `crates/tool-registry` → typed tool schemas + risk tiers
- `crates/indexer` → file watchers + keyword/semantic indexing
- `crates/speech` → TTS/ASR abstraction + platform adapters
- `docs/` → architecture, install, development, security

## Quick Start

- Install via Docker dev container (recommended): see `docs/DEVELOPMENT.md`
- Branching and PR model: see `docs/GIT_WORKFLOW.md`
- Install/setup details: `docs/INSTALL.md`

## License

MIT — see `LICENSE`.
