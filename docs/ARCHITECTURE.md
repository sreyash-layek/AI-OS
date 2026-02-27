# ARCHITECTURE (v0.1)

## High-Level Components

1. **Desktop UI (Tauri)**
   - Global hotkey launcher
   - Chat + action previews
   - Voice controls (listen/speak states)

2. **Core Daemon (Rust)**
   - Intent routing
   - Tool execution engine
   - Policy checks
   - Audit journaling

3. **Indexing Layer**
   - Folder opt-in
   - File watcher
   - Keyword + semantic search

4. **Speech Layer**
   - TTS abstraction
   - Platform adapters

## Safety Model
- Risk-tier tools (Tier 0/1/2)
- Capability-based permissions
- Preview + confirm for risky actions
- Undo for reversible operations
