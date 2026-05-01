# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A 2-player networked card battle game. Core game logic is implemented in Rust. Card definitions are pure-data JSON (no executable logic). The active frontend is a Godot 4.6 graphical client. The network architecture is P2P WebSocket: the host runs a local `card-server` (WebSocket Room Server) and the guest connects to it. A separate `card-matchmaker` (port 9090) handles matchmaking only. All workspace crates live under `crates/`.

## Common Commands

### Build
```bash
# Build entire workspace
cargo build

# Build specific crate
cargo build -p card-core
cargo build -p card-server
cargo build -p card-effect-cli

# Release build
cargo build --release -p card-server
```

### Test
```bash
# Run all workspace tests
cargo test --workspace

# Run a single test by name
cargo test -p card-core --lib test_name

# Run tests for a specific crate
cargo test -p card-core --lib
cargo test -p card-protocol
cargo test -p card-script --lib
cargo test -p card-server --lib

# Run integration tests
cargo test -p card-core --test engine_tests
cargo test -p card-core --test deck_tests
cargo test -p card-server --test room_websocket_test
cargo test -p card-server --test local_game_test
cargo test -p card-server --test replay_test
```

### Lint
```bash
cargo clippy --workspace
```

### Run Applications
```bash
# Start the game server (WebSocket Room Server, default port 8080)
cargo run --release -p card-server

# Start the matchmaker (default port 9090)
cargo run --release -p card-matchmaker

# Build and copy effect tool to Godot project
./build_effect_tool.sh

# Rebuild and background-start card-server (release mode)
./restart_server.sh
# Logs written to: logs/server.log
```

### End-to-End Testing
```bash
# Requires Python websockets library and a running card-server on port 8080
python3 test_game_start.py
python3 test_two_player_flow.py
```

### Godot Client
- Open Godot Engine 4.6+, import `godot-client/` directory, main scene is `scenes/main.tscn`.
- Default server URL: `ws://127.0.0.1:8080/ws` (WebSocket Room protocol).
- To regenerate the effect text tool binary for Godot: run `./build_effect_tool.sh`.

## High-Level Architecture

### Workspace Crates (7 members)
Dependency direction is strictly downward: upper crates may depend on lower ones, never reverse.

- **`card-core`** — Pure game logic. Zero IO. Contains `GameState`, `GameEngine`, `PhaseRunner`, `ChainManager` (FILO chain resolution), effect AST, and `CoreGameEvent`. All state changes are event-driven and serializable.
- **`card-script`** — Loads JSON card definitions (`scripts_json/`) into `CardRegistryImpl`. Lua path (`scripts/`) is deprecated.
- **`card-protocol`** — Network message types (`Command`, `GameEvent`, `AvailableAction`). Raw TCP codec is removed; WebSocket + JSON is used.
- **`card-client`** — `ClientApi` async trait and `VisibleGameState` / `OpponentView` (hides opponent hand contents). Provides `TestClient` for deterministic testing.
- **`card-server`** — WebSocket Room Server (port 8080). Contains `RoomManager`, `Room`, `ReplayRecorder`, `GameSnapshot`, and `game_starter::start_game()`. Uses `ActionRequestState` + `Condvar` to bridge async WebSocket layer with sync `GameEngine` thread.
- **`card-matchmaker`** — Independent WebSocket matchmaking server (port 9090). FIFO queue, version check. No game logic.
- **`card-effect-cli`** — Binary `card-effect-tool`. Reads JSON effect definitions and outputs Chinese effect text. Built by `build_effect_tool.sh` into `godot-client/tools/`.

### Key Architectural Patterns

- **Command/Event**: Clients send `Command`. The engine validates, advances state, and emits `GameEvent`. The server broadcasts events and updates visible state. Clients must never derive state independently.
- **Determinism**: `GameState` carries an `rng_seed`; uses `ChaCha8Rng`. Enables replay and AI testing.
- **Information Hiding**: `VisibleGameState` must expose opponent hand count only (`hand_count`), never hand contents.
- **P2P Host Model**: No centralized game server. The host runs `card-server` locally; the guest connects via WebSocket.

### Card Data Authority
The sole source of truth for card definitions is `scripts_json/S000/*.json`. The copy under `godot-client/scripts_json/` should be kept in sync when definitions change. The `scripts/` directory (Lua) is deprecated—do not modify it.

### Communication Protocol
Godot client and server communicate via JSON over WebSocket. Key client messages: `join_room`, `submit_deck`, `action`, `recovery`. Key server pushes: `joined`, `opponent_joined`, `game_started`, `state_update`, `action_request`, `recovery_request`, `game_over`, `error`. Important: `game_started` is a pure notification without state data—clients must wait for the subsequent `state_update` to get initial game state. See `godot-client/PROTOCOL.md` for full message schemas.

### Godot Client Scenes (key files)
- `scenes/main.tscn` — entry point
- `scenes/game_lobby.tscn` — lobby / room creation
- `scenes/game_boads/` — game board scenes (battlefield, hand, zones)
- `scenes/deck_editor.tscn` — deck building
- `scenes/room_waiting.tscn` — waiting room
- `scenes/connecting.tscn` — connection screen
- `managers/network_manager.gd` — WebSocket room protocol handler (autoload)
- `managers/scene_manager.gd` — scene transitions (autoload)
- `scripts/game_state.gd` — game state cache (autoload)
- `scripts/network.gd` — legacy WebSocket client (superseded, do not use for new features)

## Language & Conventions

- All dialogue, analysis, and explanations related to this project must be in **Chinese**.
- Code identifiers, APIs, and types remain in English.
- Rust Edition 2024 across the entire workspace.
- Libraries (`card-core`, `card-protocol`, `card-client`, `card-script`) use `thiserror` for precise error enums. Applications (`card-server`, `card-matchmaker`, `card-effect-cli`) use `anyhow` for error propagation.
- Use `tracing` for structured logging.
- Avoid `unsafe` in new code.

## Important Constraints

- Do not build a centralized game server; P2P host architecture is a hard requirement.
- Do not expose opponent hand contents in any view—only `hand_count`.
- Do not modify `godot-client/addons/card-framework/` to fix gameplay bugs; fix in business layers (`scenes/`, `managers/`, `scripts/`).
- Do not modify `scripts/` (Lua) for card definitions; use `scripts_json/` instead.
- When modifying `card-core` effect types or `card-protocol` messages, check downstream consumers (`card-script`, `card-effect-cli`, `card-server`, Godot client).

## Reference Documentation

| Topic | Location |
|-------|----------|
| Game rules (authoritative) | `plan/ReadMe.md` |
| System architecture | `docs/architecture.md` |
| Rust API design | `docs/api.md` |
| Game start flow | `docs/game_start_flow.md` |
| Godot WebSocket protocol | `godot-client/PROTOCOL.md` |
| Godot client quick start | `godot-client/README.md` |
