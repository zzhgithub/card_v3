# Learnings — card-game-engine

## [2026-03-13] Session Start
- Project is greenfield (NO COMMITS yet — worktree not possible)
- Working directory: /Users/zhouzihao/RustroverProjects/card_v3
- Cargo edition = "2024"
- unsafe ONLY allowed in card-script crate (Lua API loading)
- All code/API identifiers in English; game spec and docs in Chinese

## [2026-03-14] Workspace Scaffolding Complete

### Task: Convert greenfield single-crate to 7-crate Cargo workspace

**Outcome**: ✅ SUCCESS
- Root `Cargo.toml` → workspace config with 7 members
- All 7 crates created with correct structure:
  - `card-core`: game engine core (lib)
  - `card-script`: Lua scripting (lib, depends on card-core)
  - `card-protocol`: network protocol (lib, depends on card-core)
  - `card-client`: client API trait (lib, depends on card-core + card-protocol)
  - `card-server`: game server (lib + bin, depends on all except card-matchmaker)
  - `card-tui`: TUI frontend (bin, depends on card-core/protocol/client/server)
  - `card-matchmaker`: matchmaking server (bin, depends on card-protocol)

### Dependency Graph Verified
```
card-core  ←─────────────────────────────
   ↑                                      │
card-script  card-protocol  card-client   │
              ↑    ↑    ↑                 │
           card-server  card-matchmaker   │
              ↑                           │
           card-tui                       │
```

### Build Results
- `cargo build --workspace`: ✅ exit 0 (35.54s)
- Individual crate builds: ✅ all 7 pass
- `cargo tree`: ✅ dependency graph correct
- `cargo metadata`: ✅ 7 workspace members detected

### Workspace Dependencies (shared via [workspace.dependencies])
- serde 1.0 + serde_json 1.0
- thiserror 2.0 (for libs), anyhow 1.0 (for bins)
- tokio 1.50 (full features)
- mlua 0.10 with lua54 + serialize + vendored (avoids system Lua dependency)
- ratatui 0.29 + crossterm 0.28 (TUI)
- tracing 0.1 + tracing-subscriber 0.3
- tokio-tungstenite 0.26 (WebSocket)
- bincode 1.3 (serialization)
- chrono 0.4, rand 0.9, rand_chacha 0.3, async-trait 0.1

### Key Decisions
1. **mlua vendored feature**: Ensures Lua is compiled from source, no system dependency
2. **bincode 1.x**: Simpler API than 2.x, sufficient for protocol serialization
3. **Workspace resolver = "2"**: Modern resolver, required for Edition 2024
4. **Library vs Binary split**: 
   - card-server: both lib (for testing) + bin (for deployment)
   - card-tui, card-matchmaker: bin only (no lib needed)
5. **Internal path dependencies**: All inter-crate deps use `path = "../crate-name"`

### Files Created
- Root: `Cargo.toml` (workspace config)
- 7 crates × (Cargo.toml + src/lib.rs or src/main.rs)
- `scripts/.gitkeep`, `docs/.gitkeep`
- Deleted: old `src/main.rs` from root

### Verification Commands (all pass)
```bash
cargo build --workspace          # ✅ exit 0
cargo build -p card-core         # ✅
cargo build -p card-script       # ✅
cargo build -p card-protocol     # ✅
cargo build -p card-client       # ✅
cargo build -p card-server       # ✅
cargo build -p card-tui          # ✅
cargo build -p card-matchmaker   # ✅
cargo tree --depth 1             # ✅ correct graph
cargo metadata --format-version 1 | jq '.workspace_members | length'  # ✅ 7
```

### Next Steps (for future tasks)
- Implement core game engine in card-core (phases, zones, effects)
- Implement Lua scripting layer in card-script
- Implement network protocol in card-protocol
- Implement game server logic in card-server
- Implement TUI client in card-tui
- Implement matchmaking server in card-matchmaker
