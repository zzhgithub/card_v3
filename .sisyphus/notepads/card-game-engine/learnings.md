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

## [2026-03-15] Lua Table Parser for card-script

### Task: Implement `crates/card-script/src/parser.rs` and wire real parsing in loader

**Outcome**: ✅ SUCCESS

### Key Observations
1. Actual `CardDefinition` shape is `cost: u32`, `attack: Option<u32>`, `effects: HashMap<EffectKey, serde_json::Value>`; parser must target JSON effects, not `Effect` AST directly.
2. `ScriptLoader::load_for_game` tests using `-- comment` Lua files broke once real parsing was enabled; fixtures must return full Lua tables.
3. `extract_referenced_cards` can reliably scan `effects[*].actions` JSON for `type = "SummonFromZone"` + `card_id` and dedupe by ID.
4. `mlua::Value` to JSON conversion needs strict handling for unsupported runtime values (function/thread/userdata) to keep script errors explicit.

### Verification
- `cargo build -p card-script`: passed
- `cargo test -p card-script`: passed (13/13)
- LSP diagnostics on changed Rust files: clean

## [2026-03-15] Lua Sandbox (card-script)

### Task: Implement sandboxed Lua VM in `crates/card-script/src/sandbox.rs`

**Outcome**: ✅ SUCCESS — commit `15d4204`

### Key Observations
1. mlua 0.10 `set_memory_limit` returns `Result<usize, Error>` (not bare `usize`); must handle with `?` or `map_err`
2. `Lua::new_with(libs, LuaOptions::default())` returns `Result<Lua, Error>` — proper error handling needed
3. StdLib whitelist: `TABLE | STRING | MATH` — io/os/debug/package/coroutine all excluded
4. Even with StdLib restriction, some globals like `loadfile`/`dofile`/`require`/`load`/`collectgarbage` may leak; explicit nil-out via `globals.raw_set` needed
5. loader.rs `load_script` replaced `mlua::Lua::new()` → `crate::sandbox::create_sandboxed_lua()?`; all existing loader tests still pass with sandbox

### Verification
- `cargo build -p card-script`: ✅ zero errors, zero warnings
- `cargo test -p card-script`: ✅ 20/20 passed (7 sandbox + 8 loader + 5 parser)
- LSP diagnostics: ✅ clean on all 3 changed files

## [2026-03-16] Trigger Checker 扫描系统（card-core）

### Task: 实现 `engine/trigger.rs` + `GameState` 激活限制状态字段

**Outcome**: ✅ SUCCESS

### Key Observations
1. `card-core` 不能依赖 `card-protocol`（已存在单向依赖 `card-protocol -> card-core`），因此在 `card-core::state::events` 新增 `CoreGameEvent` 作为核心事件模型。
2. `GameState` 新增 `activated_this_turn: HashSet<(InstanceId, EffectKey)>` 后，`Serialize/Deserialize + PartialEq` 回归测试保持通过。
3. 目前 `GameState` 不持有可直接读取效果 AST 的卡牌定义注册表（`CardDefinition.effects` 是 JSON），因此 `check_triggers` 的扫描骨架已就位，实例效果解析留待后续引擎整合阶段接入。
4. `TriggerChecker::is_activation_allowed` 已完整覆盖 `OncePerTurn` 与 `OncePerTurnSameName` 限制逻辑。

### Verification
- `cargo build -p card-core`: passed
- `cargo test -p card-core`: passed (68/68)
- LSP diagnostics on changed files: clean

## Task: Modifier System (modifier.rs)
**Date:** 2026-03-16
**Commit:** `feat(core): implement modifier system with duration management`

### Key Implementation Details
1. `ModifierManager` is stateless — all methods take `&mut CardInstance` or `&mut GameState`.
2. `apply_modifier` immediately adjusts `current_attack` for `AttackBoost` modifiers (clamped ≥0).
3. `remove_modifier` reverts `AttackBoost` effects on removal (also clamped ≥0).
4. `cleanup_expired` handles all 4 duration types: `Permanent` (never), `UntilEndOfTurn` (always), `TurnCount(0)` (expired), `WhileSourceOnField` (checks `is_on_field`).
5. Borrow checker pattern: `cleanup_expired` uses `is_none() + continue` then separate immutable/mutable borrows to avoid NLL conflicts when reading card for expired indices then mutating to remove.
6. `ImmunityCheck` enum covers 4 immunity types: `ByCardType`, `ByProperty`, `Destruction`, `Targeting`.

### Verification
- `cargo build -p card-core`: zero warnings, zero errors
- `cargo test -p card-core`: 79/79 passed (11 new modifier tests)
- LSP diagnostics on changed files: clean

## [2026-03-21] GameEngine 主循环整合（card-core）

### Task: 实现 `crates/card-core/src/engine/game_engine.rs`

### Key Observations
1. `GameEngine` 直接依赖 `PhaseClient` trait，而不是 `card-client` 的 API，避免 `card-core` 产生循环依赖。
2. `PhaseRunner` 负责基础回合推进；`GameEngine` 额外串起 `ModifierManager::decrement_turn_counts`、触发器扫描与链结算，形成完整主循环外壳。
3. 当前卡牌效果仍以 `CardDefinition.effects: HashMap<EffectKey, serde_json::Value>` 存储，因此链条入口需要在运行时按需反序列化为 `effect::Effect`。
4. `TriggerChecker::check_triggers` 目前内部已扫描双方场上卡，因此在 `GameEngine` 中只用当前回合玩家视角调用一次，避免重复触发。
5. 波次 3 组件已经有接线位，但由于 `TriggerChecker::effects_for_instance` 仍返回空，实际自动触发/连锁效果会在后续任务补全数据来源后生效。

## [2026-03-21] S000 测试卡包（9 张 Lua 脚本）

### Task: 创建 `scripts/S000/` 目录并编写 9 张测试卡 Lua 脚本

**Outcome**: ✅ SUCCESS

### 卡片清单
| 文件 | 类型 | 名称 | 效果 |
|------|------|------|------|
| S000-C-001 | Character | 理性学者 | 无（基础人物卡） |
| S000-C-002 | Character | 知识探索者 | OnSummon → Draw 1 |
| S000-C-003 | Character | 守护者 | OnSummon → HealHp 1 |
| S000-S-001 | Strategy/Normal | 逻辑打击 | OwnMainPhase → Damage 1 (Opponent) |
| S000-S-002 | Strategy/Trick | 思维混乱 | OwnMainPhase → Discard 1 (Opponent) |
| S000-S-003 | Strategy/Instant | 灵光一现 | BothMainPhase → Draw 2 |
| S000-I-001 | Item/Normal | 能量水晶 | OwnMainPhase → GainRealPoint 1 |
| S000-I-002 | Item/Persistent | 生命之泉 | TurnStart → HealHp 1 |
| S000-L-001 | Legendary | 传奇智者 | OnSummon → Damage 2 (Opponent) |

### Key Observations
1. Strategy 卡 **必须** 定义 `strategy_kind`，否则 `validate_type_specific_fields` 报错
2. Item 卡 **必须** 定义 `item_kind`，同上
3. Strategy/Item 卡 **不能** 定义 `attack`，否则 parser 拒绝
4. `tags` 字段可以是空表 `{}`，也可以是字符串数组如 `{"传奇"}`
5. effects 中 action 的数值键名取决于类型：Draw/Discard 用 `count`，其他用 `amount`

### Verification
- `ls scripts/S000/`: 9 个 .lua 文件
- `cargo test -p card-script`: 20/20 passed

## [2026-03-22] GameSession 生命周期管理（card-server）

### Task: 创建 `crates/card-server/src/session.rs`

**Outcome**: ✅ SUCCESS

### Key Observations
1. `ScriptError` 不满足 `Send + Sync`（因为内含 `mlua::Error` 持有 `Arc<dyn StdError>`），不能直接用 `?` 转换为 `anyhow::Error`，需要 `.map_err(|e| anyhow::anyhow!("...: {}", e))` 手动转换。
2. `validate_deck` 返回 `Result<(), String>`（不是 `CoreError`），也需要 `.map_err` 包装为 `anyhow`。
3. `ScriptLoader::load_for_game` 对空卡组安全：`to_load` HashSet 为空时不迭代，直接返回空 `CardRegistryImpl`。
4. `ScriptIndex::scan` 对不存在的目录安全：直接返回空 index。
5. `SessionPhase` 状态机：WaitingForPlayers → DeckSubmission → LoadingScripts → Playing → Finished。验证在 LoadingScripts 阶段完成（需要先加载注册表才能验证卡组）。
6. `GameSession::start` 消费 self，因为 `Box<dyn PhaseClient>` 需要移动到 `GameEngine`。

### Verification
- `cargo build -p card-server`: zero errors
- `cargo test -p card-server`: 17/17 passed (6 new session tests)
- LSP diagnostics: clean

## [2026-03-22] TUI 骨架主循环（card-tui）

### Task: 实现 main.rs + app.rs 的 TUI 主循环与 PhaseClient 桥接

### Key Observations
1. `ratatui::init()` / `ratatui::restore()` 组合在 0.29 可直接用，配合 panic hook 可保证崩溃后终端状态恢复。
2. `PhaseClient` 是同步 trait；TUI 线程与引擎线程之间用 `std::sync::mpsc` + `recv_timeout` 可以实现“可阻塞、可超时”的桥接。
3. 在尚未实现 in-game 输入（Task 31）阶段，`choose_action` 自动回退 `Pass` 能保证本地演示流程可持续推进。
4. TUI 应用层日志建议写入文件（`logs/card-tui.log`），避免污染交互终端。
