# TESTING

## Why tests now?
Sprint 2 added enough behavior (routing, speech lifecycle, API contracts) that regression risk is real. Unit tests help us move faster without breaking existing behavior.

## Current test scope
- Rust unit tests in `crates/core-daemon/src/main.rs`
  - Tool preview classification logic

## Run tests locally

### Direct host
```bash
cargo test -p core-daemon -- --nocapture
```

### In dev container
```bash
docker compose exec dev bash -lc "cargo test -p core-daemon -- --nocapture"
```

## GitHub Actions
Two separate workflows are configured:

1. **Unit Tests** (`.github/workflows/tests.yml`)
   - `cargo check -p core-daemon`
   - `cargo test -p core-daemon --lib -- --nocapture`

2. **Coverage** (`.github/workflows/coverage.yml`)
   - `cargo llvm-cov -p core-daemon --lib --fail-under-lines 100 --summary-only`

> Coverage gate is currently enforced at **100% for the `core-daemon` library target** (`src/lib.rs`).
> The binary runtime (`src/main.rs`) is not included in this gate yet.

## Next testing expansion
- Extract `chat` routing into dedicated module with richer table-driven tests
- Add API handler tests for `/v1/speak` and `/v1/config/voice`
- Add frontend unit tests (Vitest) for event parsing + state transitions
