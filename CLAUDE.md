# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Dev Commands

```bash
# Install JS dependencies (only @tauri-apps/cli)
npm install

# Run in dev mode (starts Rust backend + serves frontend from src/)
npm run dev

# Production build (outputs NSIS + MSI installers)
npm run build

# Rust-only commands (from src-tauri/)
cargo build
cargo check
cargo clippy
cargo test
```

The frontend is **not bundled** — `src/` is served as static files directly (configured via `frontendDist: ../src` in tauri.conf.json).

## Architecture

**Tauri v2 desktop app** — a small transparent, always-on-top, undecorated window (200×240px) showing a pure CSS animated cat that acts as a task manager companion with emotional reactions.

### Frontend (src/)
- **Vanilla HTML/CSS/JS** — no framework, no bundler, no TypeScript
- [index.html](src/index.html) — all UI markup: cat character, speech bubble, modals (tasks, add-task, settings), right-click context menu
- [styles.css](src/styles.css) — CSS-only cat character (ears/head/eyes/body/tail from pure CSS shapes), 7 animation states (idle, remind, warning, sad, recover, happy, sleeping), modal system, toggle switches
- [main.js](src/main.js) — Frontend logic: Tauri IPC integration, pet state polling (2s interval), speech bubble system, task CRUD via modals, settings persistence, right-click context menu, window drag, hourly reminder check, idle/sleep detection

### Rust Backend (src-tauri/src/)
- [lib.rs](src-tauri/src/lib.rs) — Tauri builder entry point; manages `DbState` (SQLite) and `PetStateMachine`; registers IPC commands
- [commands.rs](src-tauri/src/commands.rs) — 11 `#[tauri::command]` functions covering task CRUD, pet state, settings, and audio/TTS. Side effects (state transitions, TTS) happen inside commands.
- [db.rs](src-tauri/src/db.rs) — `rusqlite` + `Mutex<Connection>` wrapper. Tables: `tasks`, `pet_state` (single row), `settings` (key-value). DB file: `pet_data.db` (relative to CWD).
- [state_machine.rs](src-tauri/src/state_machine.rs) — Formal state machine (7 states, 11 events). Tracks `sadness_level` (0–100) that increments on expired tasks, decrements on petting.
- [tts.rs](src-tauri/src/tts.rs) / [audio.rs](src-tauri/src/audio.rs) — Windows-only via PowerShell (`System.Speech` for TTS, `MediaPlayer` for WAV). Non-Windows: silent no-ops.

### Frontend ↔ Backend Data Flow
Frontend calls `window.__TAURI__.core.invoke('command_name', { args })` to reach Rust commands. The state machine transitions are triggered **server-side** inside command handlers (e.g., `add_task` fires `TaskAdded` event). Frontend should poll `get_pet_state()` to reflect mood changes in the CSS animation class.

### Tauri Capabilities
Permissions in [capabilities/default.json](src-tauri/capabilities/default.json) allow window manipulation (drag, position, size, visibility, always-on-top, decorations) and shell access. All commands are allowed under `core:default`.

## Known Issues

- **No app icon** — `src-tauri/icons/` is empty; tauri.conf.json references `icons/icon.ico`
- **Build environment** — requires MSVC Build Tools or GCC; Git Bash may shadow MSVC `link.exe` with GNU coreutils version
