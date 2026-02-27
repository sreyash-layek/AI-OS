# Sprints Plan (Execution)

This document tracks the detailed execution plan for **Wedge A: AI Launcher + File Brain**.
Cadence: 1 sprint = 1 week (16 total sprints).

## Sprint 1 — Foundation + Skeleton
- [x] Finalize MVP scope + Tier 0/1/2 risk tiers
- [x] Monorepo setup + CI for Windows/macOS
- [x] Tauri launcher shell + hotkey open/close
- [x] Rust daemon scaffold + IPC baseline
- [x] Tool registry schema with permissions metadata
**Deliverable:** hotkey UI talks to daemon; tools catalog exists.

## Sprint 2 — Voice Reply Loop (TTS)
- [x] Daemon speech endpoints (`/v1/speak`, `/v1/speak/stop`)
- [x] UI speak/stop controls
- [x] Voice config API (`provider`, `auto_speak`, `default_voice`)
- [x] Native TTS adapters + interruption (barge-in)
- [x] Speech lifecycle events + guardrails
**Deliverable:** assistant speaks reliably on Windows/macOS.

## Sprint 3 — Folder Opt-In + Watchers + Metadata Base
- [x] Folder/scope manager UI (opt-in indexing + pause/resume + remove)
- [x] File watcher scaffold (polling watcher loop + index event ingestion contract)
- [x] SQLite schema base (`index_scopes`, `file_metadata`)
- [x] Basic metadata search (scope paths + indexed file paths)
**Deliverable:** opted-in folders are tracked and searchable by metadata.

## Sprint 4 — Text Extraction + Keyword Index MVP
- [ ] Text extraction for `.txt`, `.md`, `.pdf`
- [ ] Tantivy keyword index (title/path/content)
- [ ] Snippet generation + highlighting
- [ ] Search results UI with previews
**Deliverable:** fast keyword search across file contents.

## Sprint 5 — Semantic Indexing + Hybrid Search
- [ ] Local embedding pipeline
- [ ] Vector DB integration (LanceDB or FAISS)
- [ ] Hybrid ranking (keyword + semantic)
- [ ] UI semantic search toggle
**Deliverable:** “find by meaning” works locally.

## Sprint 6 — App/File Open Actions (Tier 0/1)
- [ ] Tools: `open_app`, `open_file`, `reveal_in_folder`
- [ ] Native adapters (macOS LaunchServices, Windows ShellExecute/WinRT)
- [ ] Audit entries for open actions
- [ ] Keyboard-first “open top result” flow
**Deliverable:** ask → find → open is smooth and safe.

## Sprint 7 — Summarize File (Local-First)
- [ ] Tool: `summarize_file(path)`
- [ ] Chunking for long docs + summary levels
- [ ] Scope-aware enforcement
- [ ] Optional TTS summary playback
**Deliverable:** local summaries integrated with launcher flow.

## Sprint 8 — Diff-Based Safe Edits
- [ ] Tool: `edit_text_file_patch` (txt/md)
- [ ] Diff/patch preview UI
- [ ] Confirmation gate before apply
- [ ] Journal before/after patch + undo
**Deliverable:** safe, reversible text edits.

## Sprint 9 — Batch Rename/Move + Undo
- [ ] Tools: `rename_files(ops[])`, `move_files(ops[])`
- [ ] Dry-run planner + conflict handling
- [ ] Transactional apply
- [ ] Reliable undo path
**Deliverable:** trustworthy cleanup workflows (e.g., Downloads).

## Sprint 10 — Permissions & Policy Engine (AI sudo v1)
- [ ] Capability model (read/write scopes)
- [ ] Grant modes (one-time, timed, always)
- [ ] Tool gating by risk tier
- [ ] Permission dashboard + audit viewer
**Deliverable:** users can grant access with confidence.

## Sprint 11 — Local Router + Tool Selection
- [ ] Intent routing (local)
- [ ] Tool-call schema validation + retries/fallbacks
- [ ] Eval harness (prompt → expected tool calls)
- [ ] Regression test suite
**Deliverable:** natural prompts trigger correct tools consistently.

## Sprint 12 — Reliability + Performance Pass
- [ ] Indexing performance improvements (incremental + throttling)
- [ ] Search latency tuning
- [ ] Crash/recovery handling
- [ ] Progress + cancel UX for long operations
- [ ] Index size management/pruning
**Deliverable:** stable and responsive on real machines.

## Sprint 13 — Optional Cloud Boost (Opt-In)
- [ ] Modes: Local Only / Hybrid / Cloud Preferred
- [ ] Redaction + minimal-context send policies
- [ ] Cloud summarization for long docs
- [ ] UI indicator when cloud is in use
**Deliverable:** optional quality boost without trust loss.

## Sprint 14 — Packaging, Installers, Updates, Signing
- [ ] macOS signing + notarization pipeline
- [ ] Windows signing + installer (MSIX/installer)
- [ ] Auto-update integration
- [ ] Internal alpha distribution
**Deliverable:** installable, updateable alpha builds.

## Sprint 15 — Beta Hardening + Security Review
- [ ] Threat modeling review (tool abuse, data leaks)
- [ ] Permission bypass tests
- [ ] Logging privacy review
- [ ] Abuse safeguards (rate limits + confirm defaults)
**Deliverable:** security baseline ready for external beta.

## Sprint 16 — Public Beta + Feedback Loop
- [ ] Opt-in privacy-preserving telemetry
- [ ] Feedback capture + bug triage workflow
- [ ] Post-beta roadmap (connectors, screen-aware, recipes)
**Deliverable:** public beta launched with iteration cadence.
