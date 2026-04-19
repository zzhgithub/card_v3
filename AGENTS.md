# PROJECT KNOWLEDGE BASE

**Generated:** 2026-04-19
**Branch:** master

## OVERVIEW

这是一个双人网络卡牌对战游戏项目。核心游戏逻辑完全用 Rust 实现，卡牌定义通过 JSON 描述（仅描述效果，逻辑执行在 Rust 侧）。采用 P2P TCP 架构，没有中央游戏服务器，主机在开局时运行本地服务器。项目包含多个前端：终端 TUI（ratatui）与 Godot 图形客户端。

项目已经不是 Greenfield 状态——核心引擎、网络协议、服务器（WebSocket + 传统 TCP）、多种客户端、卡组编辑器、匹配服务器、录像/残局系统均已实现，并配有大量测试。

---

## STRUCTURE

```
card_v3/
├── Cargo.toml              # Workspace 根配置，Edition 2024
├── crates/                 # Rust workspace crates（9 个）
│   ├── card-core           # 纯游戏逻辑核心（状态、效果 AST、引擎、规则）
│   ├── card-script         # JSON 卡牌定义加载
│   ├── card-protocol       # 网络消息协议与 TCP 编解码
│   ├── card-client         # 客户端 API trait 与可见状态抽象
│   ├── card-server         # 游戏服务器（WebSocket Room + 传统 TCP）
│   ├── card-tui            # 终端用户界面（ratatui）
│   ├── card-bevy           # Bevy 图形客户端（已废弃，不再使用）
│   ├── card-matchmaker     # 独立 WebSocket 匹配服务器
│   └── card-effect-cli     # 卡牌效果文本 CLI 工具
├── godot-client/           # Godot 4.6 图形客户端（完整卡组编辑、对战场景）
├── scripts/                # 废弃：Lua 卡牌定义（S000/ 包，10 张）
├── scripts_json/           # JSON 卡牌定义（S000/ 包，18 张，目前更完整）
├── desks/                  # 卡组 JSON 文件（40–60 张）
├── plan/                   # 游戏设计规格书与 UI 原型图
│   └── ReadMe.md           # 权威游戏规则文档（184 行）
├── docs/                   # 技术文档
│   ├── api.md              # Rust API 设计文档（ exhaustive ）
│   ├── architecture.md     # 系统架构设计
│   ├── game_start_flow.md  # 游戏开局流程详细说明
│   └── plans/              # 开发计划（本地游戏面板重写、集成测试等）
├── logs/                   # 运行时日志（gitignored）
├── build_effect_tool.sh    # 构建 card-effect-cli 并复制到 Godot 的脚本
├── restart_server.sh       # 重新构建并后台启动 card-server 的脚本
└── .worktrees/             # Git worktree
```

---

## TECHNOLOGY STACK

| 层级 | 技术 | 说明 |
|------|------|------|
| 语言 | Rust 2024 | 全 workspace 统一使用 Edition 2024 |
| 异步运行时 | tokio | 网络层、服务器、客户端均基于 tokio |
| 序列化 | serde + serde_json + bincode | 状态、消息、录像、快照；TCP 层使用 bincode |

| 终端 UI | ratatui + crossterm | card-tui 的完整终端界面 |

| WebSocket | tokio-tungstenite | 房间服务器与匹配服务器 |
| 日志 | tracing + tracing-subscriber | 结构化日志，env-filter 支持 |
| 错误处理 | thiserror + anyhow | thiserror 用于库错误，anyhow 用于应用 |
| 随机数 | rand + rand_chacha | 可种子化的确定性随机（录像/AI） |
| 外部客户端 | Godot 4.6 (Mobile renderer) | 独立项目，位于 godot-client/ |

---

## CRATE RESPONSIBILITIES & DEPENDENCIES

### `card-core` — 纯游戏逻辑核心
- **不依赖**任何网络或渲染库。
- 包含：领域类型（`CardId`, `Zone`, `Phase` 等）、完整对局状态 `GameState`、效果系统 AST（`Effect`/`Trigger`/`Condition`/`Action`/`Modifier`）、`GameRules`、卡组校验、`PhaseRunner` 回合状态机、`ActionExecutor`、`ChainManager`（FILO 连锁）、`TriggerChecker`、`ModifierManager`。
- 所有状态变更以 `CoreGameEvent` 事件驱动，支持序列化与 PartialEq，用于存盘与录像验证。
- 引擎组件均为无状态函数，配合 `ScriptedClient` 可实现完全确定性的自动化测试。

### `card-script` — 卡牌定义加载
- 支持 **JSON**（当前唯一路径，`json_loader.rs`）格式。
- 输出 `CardRegistryImpl`（`HashMap<CardId, CardDefinition>`），供引擎使用。
- Godot 客户端与引擎均只读取 JSON。

### `card-protocol` — 网络协议层
- 定义所有对等节点间的消息类型：`NetworkMessage`、`Command`、`GameEvent`、`AvailableAction`。
- `TcpConnection`：基于 tokio 的长度前缀 + bincode 序列化编解码器，消息上限 1 MiB。
- 包含单元测试（loopback、复杂消息往返、超大消息拒绝）。

### `card-client` — 客户端 API 抽象
- 定义 `ClientApi` async trait：所有前端（TUI、Godot）必须实现。
- 提供 `VisibleGameState` / `OpponentView`：对手手牌仅暴露数量，不暴露内容。
- 包含 `TestClient`：确定性测试客户端。

### `card-server` — 游戏服务器
- **双轨实现**：
  1. **WebSocket Room Server**（当前主要入口，`main.rs` 使用）：`WebSocketServer` → `RoomManager`/`Room` → `game_starter::start_game()`。使用 JSON-over-WebSocket 协议，支持房间 join/leave、deck 提交、ready/unready、自动开局。
  2. **传统 Raw TCP Server**（`network.rs`）：直接接受 2 个 TCP 连接，握手、deck 校验后运行 `GameSession`。用于更直接的 P2P 场景。
- 其他模块：`GameSession`（完整游戏生命周期）、`AiClient`（确定性 AI）、`ReplayRecorder`/`ReplayData`（录像 JSON 文件）、`GameSnapshot`（残局存盘）、`RemoteClient`（TCP 上的 `ClientApi` 实现）。
- 使用 `ActionRequestState` + `Condvar` 在 async WebSocket 层与同步 `GameEngine` 线程之间桥接。

### `card-tui` — 终端用户界面
- ratatui 实现的完整终端体验：主菜单、卡组浏览器/编辑器、本地 AI 对战、在线匹配。
- `TuiClient` 实现 `PhaseClient` trait，通过 channel 与引擎交互。
- UI 模块在 `ui/` 下，所有界面文本为中文。

### `card-bevy` — Bevy 图形客户端（已废弃）
- 该 crate 已不再使用，也不再编译。当前活跃的前端为 `card-tui`（终端）与 `godot-client`（Godot 图形客户端）。

### `card-matchmaker` — 独立匹配服务器
- WebSocket 端口 9090（默认）。FIFO 队列配对，版本检查，返回对手信息及 P2P host 地址。

### `card-effect-cli` — 效果文本工具
- 二进制名 `card-effect-tool`。
- 读取 JSON 效果定义，输出中文效果描述文本。
- `build_effect_tool.sh` 将其构建并复制到 `godot-client/tools/`，供 Godot 客户端调用。

---

## BUILD & TEST COMMANDS

```bash
# 构建全部
cargo build

# 构建特定 crate（推荐）
cargo build -p card-core
cargo build -p card-server
cargo build -p card-tui

# 运行特定 crate 测试（推荐）
cargo test -p card-core --lib          # ~134 个单元测试
cargo test -p card-protocol            # ~7 个测试
cargo test -p card-script --lib        # ~30 个测试
cargo test -p card-server --lib        # ~29 个测试
cargo test -p card-core --test engine_tests   # 引擎集成测试（~40+）
cargo test -p card-server --test room_websocket_test  # WebSocket 房间集成测试

# 运行全部测试
cargo test --workspace

# Clippy
cargo clippy --workspace

# 构建并复制效果工具到 Godot 项目
./build_effect_tool.sh

# 重新构建并后台启动 card-server（release 模式）
./restart_server.sh
```

---

## CODE ORGANIZATION PRINCIPLES

- **分层架构**：`card-core`（纯逻辑） → `card-protocol`（网络消息） → `card-client`/`card-server`（网络 + 会话） → `card-tui`/`godot-client`（前端）。上层 crate 可以依赖下层，禁止反向依赖。
- **事件驱动**：`card-core` 中所有状态变更产生 `CoreGameEvent`；网络层将其转为 `GameEvent`；客户端据此更新 UI。录像与残局均基于此事件流。
- **trait 抽象**：`PhaseClient`（引擎请求玩家决策）与 `ClientApi`（前端接收事件/做出选择）是核心抽象，允许多种前端接入同一引擎。
- **信息隐藏**：`VisibleGameState` 确保对手手牌仅显示 `hand_count`，绝不暴露内容。服务器在构建状态视图时严格分离。
- **确定性**：`GameState` 包含 `rng_seed`，配合 `ChaCha8Rng` 实现可复现的对局，支持录像回放和 AI 测试。

---

## DEVELOPMENT CONVENTIONS

- **Edition 2024**：使用最新 Rust 特性。
- **unsafe**：当前项目已无 Lua 集成层；除历史兼容原因外，**禁止**在新增代码中使用 `unsafe`。
- **语言**：游戏规格和 UI 文本使用中文；代码标识符、API、类型名使用英文。
- **AI 交互语言**：所有与项目相关的对话、分析、解释必须使用 **中文**。
- **日志**：使用 `tracing` 进行结构化日志记录，是显式要求。
- **错误类型**：库 crate 使用 `thiserror` 定义精确错误枚举；应用 crate 使用 `anyhow` 进行错误传播。
- **测试风格**：大量单元测试 + `ScriptedClient`/`PassAllClient` 驱动引擎进行确定性集成测试。`card-server` 包含基于真实 WebSocket/TCP 的异步集成测试。

---

## ANTI-PATTERNS

- DO NOT 构建中央游戏服务器 — P2P 架构是硬性要求（当前 card-server 是主机本地服务器）。
- DO NOT 硬编码游戏规则 — 规则通过 `GameRules` 每局可配置。
- DO NOT 在对手视图中暴露手牌内容 — 必须仅暴露 `hand_count`。

---

## KEY DOMAIN MODEL

### 卡牌类型
- **Character**（人物卡）— 有攻击力，战斗在前场（ForEnd Zone）。
- **Strategy**（策略卡）— 子类型：Normal、Trick（诡计）、Instant（瞬时）。
- **Item**（物品卡）— 子类型：Normal、Persistent（存留）。
- **Legendary**（传奇卡）— 卡组中最多 5 张。

### 玩家区域
Deck（40–60） | Hand（max 20） | Front（5 格） | Back（5 格） | CostZone（max 6） | Grave | HP（初始 5，max 6） | RealPoint（max 6）

### 回合阶段
Turn Start → Draw → Recovery → Main1 → Battle → Main2 → Turn End

### 特殊机制
- **Cost 系统**：手牌送入 Cost Zone 作为费用；Cost Zone 中的卡仍可被使用。
- **RealPoint 溢出**：超过 6 时触发 `RealPointOverflow` 事件，卡牌可响应。
- **连锁**：FILO（类似游戏王），由 `ChainManager` 解析。
- **Recovery**：回收阶段回手 X 张卡，X = 对手前场最高费用卡的 cost。

---

## WHERE TO LOOK

| 任务 | 位置 | 备注 |
|------|------|------|
| 游戏规则与机制 | `plan/ReadMe.md` | 权威规格书：阶段、战斗、费用、效果 |
| 卡牌类型定义 | `plan/ReadMe.md` §卡片信息 | 4 种类型及子类型 |
| 效果/连锁系统 | `plan/ReadMe.md` §效果和连锁系统 | FILO 连锁、Trigger/Condition/Action 模型 |
| Rust API 设计 | `docs/api.md` | 详细的类型、trait、消息定义 |
| 系统架构 | `docs/architecture.md` | 各 crate 职责、Command/Event 模式 |
| 卡牌定义（JSON） | `scripts_json/S000/*.json` | 当前唯一数据源 |
| 卡组文件 | `desks/*.json` | 玩家卡组 |
| Godot 客户端网络 | `godot-client/managers/network_manager.gd` | WebSocket 房间协议（端口 8080） |
| Godot 客户端旧网络 | `godot-client/scripts/network.gd` | legacy WebSocket（端口 7878） |
| 效果文本工具缺失 | `godot-client/tools/` | 运行 `build_effect_tool.sh` 生成 |

---

## SECURITY & DEPLOYMENT NOTES

- **无 CI/CD**：当前没有 GitHub Actions、Dockerfile 或 Makefile，构建和部署依赖手动 `cargo` 命令和项目根目录的两个 shell 脚本。
- **服务器部署**：`restart_server.sh` 以 `nohup` 后台方式启动 `card-server`（release 模式），日志输出到 `logs/server.log`。

- **消息大小限制**：`card-protocol` 的 `TcpConnection` 限制单条消息最大 1 MiB，防止内存耗尽攻击。
- **版本检查**：匹配服务器和 TCP 握手均包含版本校验，不匹配时拒绝连接。

---

## NOTES FOR AGENTS

- 在修改任何 crate 之前，先阅读 `plan/ReadMe.md` 中相关游戏规则段落，确保逻辑符合规格。
- 如果修改卡牌定义，只需更新 `scripts_json/` 下的 JSON 文件。`scripts/`（Lua）目录已废弃。
- `src/` 目录已为空，项目入口点分布在各 crate 的 `src/main.rs` 中。
- 游戏状态 `GameState` 是权威的唯一真实来源（single source of truth），任何前端都不应自行推演状态，只应根据事件更新表现层。
