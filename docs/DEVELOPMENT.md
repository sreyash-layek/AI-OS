# DEVELOPMENT SETUP

## Preferred: Docker Dev Container

AI-OS uses a dev container for consistent setup across Windows/macOS/Linux.

### Requirements
- Docker Desktop (or Docker Engine)
- VS Code (optional, for Dev Containers UX)

### Start dev container
```bash
docker compose up -d --build
docker compose exec dev bash
```

Inside container:
```bash
cargo check -p core-daemon
```

## Local (without Docker)
### Prerequisites
- Git
- Rust (stable)
- Node.js (LTS)
- pnpm or npm
- Tauri prerequisites for your OS

## Current Sprint Commands
```bash
# from repo root
cargo run -p core-daemon
```

## Engineering Guidelines (latest)
- Keep features behind documented contracts
- Use typed tool schemas for all AI actions
- Prefer local-first implementations
- Add/Update docs in same PR
- Open PRs against `develop` only
