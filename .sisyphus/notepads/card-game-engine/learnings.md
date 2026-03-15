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

## [2026-03-14] Effect System AST Types

### Task: Define complete effect system AST types in `card-core/src/effect/`

**Outcome**: ✅ SUCCESS — commit `b956dab`

### Types Defined (16 total)
Effect, Trigger, EventTrigger, ActivationLimit, Condition, CompareOp, ValueExpr,
Action, Choice, ChoiceCount, TargetType, TargetFilter, CostRequirement,
Modifier, AppliedModifier, ModifierDuration

### Key Observations
1. **CardFilter lacks PartialEq** — types containing `CardFilter` (Effect, EventTrigger, Trigger, TargetFilter, CostRequirement, Choice) cannot derive `PartialEq`. Tests use JSON roundtrip comparison instead.
2. **PlayerRef uses `Self_` not `Me`** — the existing `PlayerRef` enum uses `Self_`/`Opponent`.
3. **CardRef has no `Choice(u8)` variant** — existing `CardRef` has `This`/`ByInstanceId`/`BySlot`. The effect AST doesn't currently support referencing choice results by ID in action targets. All 7 spec examples work without this (they use `CardRef::This` or specific IDs). May need to be added later for complex targeting patterns.
4. **Continuous effects (aura)** — modeled as `OnSummon` + `Condition::CardIsOnField` + `ModifierDuration::WhileSourceOnField`. The "apply to other cards" semantics are a runtime concern, not encoded in the AST.

### Tests
- 7 spec example tests (mapping to plan/ReadMe.md:177-184)
- 3 additional tests (condition AST composition, applied modifier roundtrip, choice with card filter)
- All 41 tests pass (11 new effect + 30 existing)

## [2026-03-14] Protocol Message Types

### Task: Define network protocol message types in `card-protocol/src/message.rs`

**Outcome**: ✅ SUCCESS — commit `e0e2b84`

### Types Defined (8 total)
- Auxiliary: CostPayment, AttackTarget, Phase, GameOverReason
- Core enums: Command (10 variants), AvailableAction (7 variants), GameEvent (21 variants), NetworkMessage (16 variants)

### Key Observations
1. **bincode 1.x API**: `bincode::serialize(&val)` / `bincode::deserialize(&bytes)` — simple and works with serde derives
2. **card-core re-exports**: All needed types (CardId, EffectKey, InstanceId, PlayerId, TargetRef, Zone, ZoneLocation) available via `card_core::types::*`
3. **Pure type definitions**: Zero runtime logic, zero TCP/IO code — clean separation for Task 14 (framing)
4. **Tests**: 4 roundtrip tests (bincode + serde_json) all pass

### Verification
- `cargo build -p card-protocol`: ✅ zero errors
- `cargo test -p card-protocol`: ✅ 4/4 passed
- LSP diagnostics: ✅ clean on both files

## [2026-03-14] API 文档落地（docs/api.md）

### Task: 基于已实现公开类型编写中文 API 设计文档

**Outcome**: ✅ SUCCESS

### 关键结论
1. `CardDefinition` 当前实现使用 `cost: u32`、`attack: Option<u32>`、`effects: HashMap<EffectKey, serde_json::Value>`，与规划稿存在类型差异。
2. `CardRef` 当前实现是 `This`/`ByInstanceId`/`BySlot`，并非早期方案中的 `Choice(u8)` 形式。
3. `GameEvent` 已包含请求型事件（`RequestAction`、`RequestTargetSelection`、`RequestCardSelection`），客户端可直接用事件流驱动交互。
4. `ClientApi` 尚未实现，但文档已给出稳定 trait 设计草案，可作为 TUI/Bevy/Web/Unity 统一边界。
5. Lua 脚本建议保持声明式，解析后先进入 JSON，再映射为效果 AST，便于兼容升级。

### 验证记录
- `wc -l docs/api.md` → `1020`（满足 >300）
- `grep -c "ClientApi" docs/api.md` → `12`
- `grep -c "CardDefinition" docs/api.md` → `13`
- `grep -c "GameEvent" docs/api.md` → `15`
- `grep -c "Lua" docs/api.md` → `16`

## Error Type Layering (2026-03-14)
- Library crates use `thiserror` for typed errors: CoreError, ScriptError, ProtocolError, ClientError
- Application crates use `anyhow::Result` for top-level error propagation
- ProtocolError derives Serialize/Deserialize for network transmission
- ScriptError wraps `mlua::Error` via `#[from]`
- ClientError wraps `ProtocolError` via `#[from]`
- card-script imports CardId from `card_core::types::CardId`
- card-client imports ProtocolError from `card_protocol::ProtocolError`
- All app crates (server/tui/matchmaker) use `#[tokio::main] async fn main() -> Result<()>`

## [2026-03-15] Script Index Scanning & Loader Framework

### Task: Implement ScriptIndex::scan() and ScriptLoader in card-script/src/loader.rs

**Outcome**: ✅ SUCCESS — commit `afb2f01`

### Key Observations
1. **CardId has no `validate()` method** — uses `FromStr` impl via `.parse::<CardId>()` for format validation (`S\d{3}-[CSIL]-\d{3}`)
2. **CardDefinition::new() takes `cost: u32`** — not `u8` as some earlier notes suggested
3. **No tempfile needed** — `std::env::temp_dir()` + PID-unique subdirs sufficient for test isolation
4. **ScriptLoader::load_script() is a stub** — verifies file readability but returns placeholder CardDefinition. Task 12 (parser.rs) will replace with real Lua→CardDefinition parsing

### API Surface
- `ScriptIndex::scan(dir) -> Result<Self, ScriptError>` — recursive .lua scan, CardId filename validation
- `ScriptIndex::get_path/contains/len/is_empty/iter` — index queries
- `ScriptLoader::new(index)` + `load_for_game(deck1, deck2) -> Result<CardRegistryImpl, ScriptError>` — deduplicates across decks, resolves via index

### Tests (8 total, all pass)
- scan_empty_dir, scan_nonexistent_dir, scan_finds_lua_files, scan_ignores_non_card_id_files, scan_recursive
- loader_load_for_game, loader_missing_card_error, index_iter

## [2026-03-15] TCP Codec (Length-Prefixed Bincode Framing)

### Task: Implement TcpConnection in card-protocol/src/codec.rs

**Outcome**: ✅ SUCCESS — commit `6f05a72`

### Implementation
- `TcpConnection::from_stream(TcpStream)` — splits into OwnedReadHalf/OwnedWriteHalf
- `send(&mut self, msg: &NetworkMessage) -> Result<(), ProtocolError>` — serialize + length-prefix + write
- `recv(&mut self) -> Result<NetworkMessage, ProtocolError>` — read length-prefix + payload + deserialize
- Frame format: `| 4 bytes big-endian u32 length | N bytes bincode payload |`
- Max message size: 1 MiB (both send and recv enforce)

### Key Observations
1. **Private fields accessible in child test module** — Rust visibility allows descendant modules to access private fields (used in oversized message test to write raw bytes to `conn.writer`)
2. **bincode 1.x roundtrip** — `serialize` / `deserialize` Just Works with serde derives on NetworkMessage
3. **No new dependencies needed** — tokio (full) + bincode already in workspace Cargo.toml

### Tests (3 new, 7 total for crate)
- test_loopback_ping_pong: Ping→Pong roundtrip over real TCP
- test_complex_message_roundtrip: EventNotification with GameOver payload, JSON string comparison
- test_oversized_message_rejected: Manual 2MiB length prefix → InvalidMessage error

## [2026-03-15] Condition AST Evaluator (card-core)

### Task: Implement `effect/evaluator.rs` for condition/value runtime evaluation

**Outcome**: ✅ SUCCESS

### Key Observations
1. `Condition`/`ValueExpr` now have runtime evaluator functions in `card-core`: `evaluate_condition` and `resolve_value`
2. Existing `types::CardRef` is `This`/`ByInstanceId`/`BySlot(Zone, usize)` (not `Choice`/`SpecificInstance`); evaluator must align with this actual shape
3. `GameState` currently does not hold `CardRegistryImpl`, so `CostZonePropertyCount` and `HighestCostOnField` cannot read card definitions yet; evaluator returns `0` fallback for both branches
4. `CardIsOnField` is resolved via `GameState::get_card` and zone check (`Front`/`Back` only)

### Tests & Verification
- Added evaluator-focused tests in `crates/card-core/src/effect/evaluator.rs` (8 tests)
- `cargo test -p card-core -- evaluator`: passed (8/8)
- `cargo build -p card-core`: passed
- `cargo test -p card-core`: passed (63/63)
- LSP diagnostics on changed files: clean
