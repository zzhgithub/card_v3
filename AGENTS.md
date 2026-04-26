# PROJECT KNOWLEDGE BASE

**Generated:** 2026-04-22
**Branch:** master

## OVERVIEW

这是一个双人网络卡牌对战游戏项目。核心游戏逻辑完全用 Rust 实现，卡牌定义通过 JSON 描述（仅描述效果，逻辑执行在 Rust 侧）。采用 P2P TCP 架构，没有中央游戏服务器，主机在开局时运行本地服务器。项目活跃前端为 Godot 图形客户端（card-tui 与 card-bevy 已移除）。

项目已经不是 Greenfield 状态——核心引擎、网络协议、服务器（WebSocket Room）、Godot 客户端、卡组编辑器、匹配服务器、录像/残局系统均已实现，并配有大量测试。

---

## STRUCTURE

```
card_v3/
├── Cargo.toml              # Workspace 根配置，Edition 2024，resolver = "2"
├── crates/                 # Rust workspace crates（7 个成员）
│   ├── card-core           # 纯游戏逻辑核心（状态、效果 AST、引擎、规则）
│   ├── card-script         # JSON 卡牌定义加载与解析
│   ├── card-protocol       # 网络消息协议与 TCP 编解码
│   ├── card-client         # 客户端 API trait 与可见状态抽象
│   ├── card-server         # 游戏服务器（WebSocket Room Server）
│   ├── card-matchmaker     # 独立 WebSocket 匹配服务器（端口 9090）
│   └── card-effect-cli     # 卡牌效果文本 CLI 工具（二进制名 card-effect-tool）
├── godot-client/           # Godot 4.6 图形客户端（Mobile renderer）
│   ├── project.godot       # Godot 项目配置，主场景为 scenes/main.tscn
│   ├── PROTOCOL.md         # Godot 客户端 WebSocket 通信协议文档
│   ├── README.md           # Godot 客户端快速开始指南
│   ├── addons/card-framework/  # 卡牌框架插件（Card、Hand、Pile、DropZone 等）
│   ├── managers/           # 全局自动加载管理器（NetworkManager、SceneManager）
│   ├── scripts/            # 全局自动加载脚本（Network、GameState）
│   ├── scenes/             # 场景与脚本：主菜单、卡组编辑、房间、对战面板等
│   ├── dule/               # 对战相关测试场景与卡片逻辑脚本
│   ├── desks/              # 卡组 JSON 文件（与根目录 desks/ 内容独立）
│   ├── scripts_json/       # JSON 卡牌定义副本（S000/ 包，18 张）
│   ├── images/             # 卡牌图片、UI 素材、费用图标等
│   └── tools/              # card-effect-tool 二进制存放目录
├── scripts/                # 废弃：Lua 卡牌定义（S000/ 包，10 张）
├── scripts_json/           # JSON 卡牌定义（S000/ 包，18 张，当前权威数据源）
├── desks/                  # 卡组 JSON 文件（40–60 张）
├── plan/                   # 游戏设计规格书与 UI 原型图
│   └── ReadMe.md           # 权威游戏规则文档（183 行）
├── docs/                   # 技术文档
│   ├── api.md              # Rust API 设计文档（详尽类型与协议定义）
│   ├── architecture.md     # 系统架构设计（分层、Command/Event 模式、连锁、回放）
│   ├── game_start_flow.md  # 游戏开局流程与房间状态同步详细说明
│   └── plans/              # 开发计划
├── logs/                   # 运行时日志（gitignored）
├── build_effect_tool.sh    # 构建 card-effect-cli 并复制到 godot-client/tools/ 的脚本
├── restart_server.sh       # 重新构建并后台启动 card-server（release 模式）的脚本
├── test_game_start.py      # 游戏启动流程的 Python 测试脚本
├── test_two_player_flow.py # 双人对战流程的 Python 测试脚本
└── .worktrees/             # Git worktree
```

---

## TECHNOLOGY STACK

| 层级 | 技术 | 说明 |
|------|------|------|
| 语言 | Rust Edition 2024 | 全 workspace 统一使用 Edition 2024 |
| 异步运行时 | tokio（full features） | 网络层、服务器、客户端均基于 tokio |
| 序列化 | serde + serde_json + bincode | 状态、消息、录像、快照；TCP 层使用 bincode |
| 前端 | Godot 4.6（Mobile renderer） | 唯一活跃图形客户端 |
| WebSocket | tokio-tungstenite 0.26 | 房间服务器与匹配服务器 |
| 日志 | tracing + tracing-subscriber | 结构化日志，env-filter 支持 |
| 错误处理 | thiserror（库）+ anyhow（应用） | 分层错误处理 |
| 随机数 | rand + rand_chacha | 可种子化的确定性随机（录像/AI） |
| 外部客户端 | Godot 4.6（Mobile renderer） | 独立项目，位于 godot-client/ |
| 脚本加载 | mlua（lua54, serialize, vendored） | card-script 依赖，但 Lua 路径已废弃 |

### 关键外部依赖版本

- `futures` / `futures-util` 0.3：async 工具
- `uuid` 1（v4）：card-server 房间管理
- `chrono` 0.4：回放/日志时间戳
- `async-trait` 0.1：`ClientApi` trait
- `bincode` 1：`card-protocol` TCP 编码

---

## CRATE RESPONSIBILITIES & DEPENDENCIES

### `card-core` — 纯游戏逻辑核心
- **零 IO 依赖**：不依赖任何网络、终端或文件库。
- 模块划分：
  - `types/`：领域类型（`CardId`, `Zone`, `Phase`, `PlayerId`, `CardDefinition` 等）
  - `state/`：完整对局状态 `GameState`、`CoreGameEvent`、`CardInstance`、`PlayerState`
  - `engine/`：游戏引擎（`GameEngine`、`PhaseRunner` 回合状态机、`ActionExecutor`、`ChainManager` FILO 连锁、`TriggerChecker`、`ModifierManager`）
  - `effect/`：效果系统 AST（`Effect`、`Trigger`、`Condition`、`Action`、`Modifier`、`Choice`、`CostRequirement`）及中文文本生成器 `text.rs`
  - `rules/`：`GameRules` 可配置规则、卡组校验 `validate_deck`、`CardRegistry` trait
  - `deck.rs`：卡组构建与洗牌逻辑
  - `error.rs`：`CoreError` 精确错误枚举
- 所有状态变更以 `CoreGameEvent` 事件驱动，支持序列化与 `PartialEq`，用于存盘与录像验证。
- 引擎组件为无状态函数，配合 `ScriptedClient`/`PassAllClient` 可实现完全确定性的自动化测试。

### `card-script` — 卡牌定义加载
- 当前唯一路径为 **JSON**（`json_loader.rs`、`parser.rs`）。
- `loader.rs`：通用加载接口；`sandbox.rs`：Lua 沙盒（历史兼容，已废弃）。
- 输出 `CardRegistryImpl`（`HashMap<CardId, CardDefinition>`），供引擎使用。
- Godot 客户端与引擎均只读取 JSON。

### `card-protocol` — 网络协议层
- 定义游戏核心消息类型：`Command`、`GameEvent`、`AvailableAction`、`CostPayment`。
- ~~`TcpConnection`（`codec.rs`）~~：已移除。原 Raw TCP 传输层（bincode + 长度前缀帧）已废弃，当前使用 WebSocket + JSON。
- 包含单元测试（loopback、复杂消息往返、超大消息拒绝）。

### `card-client` — 客户端 API 抽象
- 定义 `ClientApi` async trait：所有前端（TUI、Godot）的统一接口。
- 提供 `VisibleGameState` / `OpponentView`：对手手牌仅暴露数量，不暴露内容。
- 包含 `TestClient`：确定性测试客户端。
- `api.rs`：状态投影与测试客户端实现；`error.rs`：`ClientError`。

### `card-server` — 游戏服务器
- **双轨实现**：
  1. **WebSocket Room Server**（当前主要入口，`main.rs` 使用）：`WebSocketServer` → `RoomManager`/`Room` → `game_starter::start_game()`。使用 JSON-over-WebSocket 协议，支持房间 join/leave、deck 提交、ready/unready、自动开局。默认端口 8080。
- 其他模块：
  - `ai_client.rs`：确定性 AI 客户端 `AiClient`
  - `replay.rs`：`ReplayRecorder` / `ReplayData`（录像 JSON 文件）
  - `snapshot.rs`：`GameSnapshot` 残局存盘
  - `room.rs`：房间管理、`VisibleGameState` 构建、信息隐藏
  - `game_starter.rs`：游戏启动流程编排（卡组加载、校验、引擎启动）
- 使用 `ActionRequestState` + `Condvar` 在 async WebSocket 层与同步 `GameEngine` 线程之间桥接。

### `card-matchmaker` — 独立匹配服务器
- WebSocket 端口 9090（默认）。FIFO 队列配对，版本检查，返回对手信息及 P2P host 地址。
- 不参与任何局内逻辑。

### `card-effect-cli` — 效果文本工具
- 二进制名 `card-effect-tool`。
- 读取 JSON 效果定义（`HashMap<EffectKey, serde_json::Value>`），调用 `card_core::effect::text::card_effects_text` 输出中文效果描述文本。
- `build_effect_tool.sh` 将其构建并复制到 `godot-client/tools/`，供 Godot 客户端调用。

---

## GODOT CLIENT ARCHITECTURE

### 项目配置
- **Godot 版本**：4.6
- **渲染器**：Mobile
- **主场景**：`scenes/main.tscn`
- **视口**：1280x720，stretch mode 为 `canvas_items`

### Autoload（全局单例）
- `Network` (`scripts/network.gd`)：Legacy WebSocket 客户端（端口 7878，旧协议）
- `GameState` (`scripts/game_state.gd`)：游戏状态管理（回合、阶段、HP、手牌、场地等）
- `NetworkManager` (`managers/network_manager.gd`)：当前主网络管理器（WebSocket Room 协议，端口 8080）
- `SceneManager` (`managers/scene_manager.gd`)：场景切换与淡入淡出过渡

### 场景组织
| 路径 | 用途 |
|------|------|
| `scenes/main.tscn` | 主入口场景 |
| `scenes/game_lobby.tscn` / `scenes/room_waiting.tscn` | 大厅与房间等待 |
| `scenes/game_boads/game_lobby.tscn` | 新版游戏大厅 |
| `scenes/game_boads/room_waiting.tscn` | 新版房间等待 |
| `scenes/game_boads/game_board.tscn` | 对战主面板 |
| `scenes/game_boads/battle_field_slot.tscn` | 战场格子 |
| `scenes/game_boads/cost_zone.tscn` | 费用区显示 |
| `scenes/game_boads/info_bar.tscn` | 信息栏 |
| `scenes/card_catalog.tscn` | 卡牌图鉴 |
| `scenes/deck_editor.tscn` / `scenes/deck_detail.tscn` | 卡组编辑器与详情 |
| `scenes/card_display*.tscn` | 卡牌显示组件（图鉴版、卡组版、预览版） |
| `scenes/card_preview_popup.tscn` | 卡牌预览弹窗 |
| `dule/dule_card.tscn` / `dule/card_box.tscn` | 对战卡片与卡盒测试场景 |

### 网络协议
- Godot 客户端目前主要使用 **WebSocket Room 协议**（端口 8080，见 `PROTOCOL.md`）。
- 消息类型包括：`join_room`、`submit_deck`、`action`、`recovery` 等。
- 服务器推送包括：`joined`、`opponent_joined`、`game_started`、`state_update`、`action_request`、`game_over`、`error` 等。
- Legacy `scripts/network.gd`（端口 7878）保留但已被 `NetworkManager` 取代。

### 卡牌框架插件
- `addons/card-framework/` 提供可复用的卡牌基础组件：
  - `Card`（`card.gd` + `card.tscn`）：基础卡牌节点
  - `CardContainer`：卡牌容器逻辑
  - `Hand`（`hand.gd` + `hand.tscn`）：手牌区域
  - `Pile`（`pile.gd` + `pile.tscn`）：牌堆
  - `DropZone`（`drop_zone.gd` + `drop_zone.tscn`）：放置区域
  - `DraggableObject`：拖拽逻辑
  - `CardFactory` / `JSONCardFactory`：卡牌工厂

---

## BUILD & TEST COMMANDS

```bash
# 构建全部
cargo build

# 构建特定 crate（推荐）
cargo build -p card-core
cargo build -p card-server
cargo build -p card-effect-cli

# 运行特定 crate 单元测试（推荐）
cargo test -p card-core --lib          # types、effect 等单元测试
cargo test -p card-protocol            # TCP 编解码测试
cargo test -p card-script --lib        # JSON 加载与解析测试
cargo test -p card-server --lib        # 服务器模块单元测试

# 运行集成测试
cargo test -p card-core --test engine_tests           # 引擎集成测试
cargo test -p card-server --test room_websocket_test  # WebSocket 房间集成测试
cargo test -p card-server --test local_game_test      # 本地游戏测试
# 注：network_game_test（Raw TCP）已随 TCP 路径移除而删除
cargo test -p card-server --test replay_test          # 录像系统测试

# 运行全部测试
cargo test --workspace

# Clippy
cargo clippy --workspace

# 构建并复制效果工具到 Godot 项目
./build_effect_tool.sh

# 重新构建并后台启动 card-server（release 模式）
./restart_server.sh
# 日志位置：logs/server.log
```

---

## CODE ORGANIZATION PRINCIPLES

- **分层架构**：`card-core`（纯逻辑） → `card-protocol`（消息类型） → `card-client`/`card-server`（网络 + 会话） → `godot-client`（前端）。上层 crate 可以依赖下层，**禁止反向依赖**。
- **事件驱动**：`card-core` 中所有状态变更产生 `CoreGameEvent`；网络层将其转为 `GameEvent`；客户端据此更新 UI。录像与残局均基于此事件流。
- **trait 抽象**：`PhaseClient`（引擎请求玩家决策）与 `ClientApi`（前端接收事件/做出选择）是核心抽象，允许多种前端接入同一引擎。
- **信息隐藏**：`VisibleGameState` 确保对手手牌仅显示 `hand_count`，绝不暴露内容。服务器在构建状态视图时严格分离。
- **确定性**：`GameState` 包含 `rng_seed`，配合 `ChaCha8Rng` 实现可复现的对局，支持录像回放和 AI 测试。
- **声明式卡牌定义**：JSON 仅描述效果结构，不执行逻辑。Rust 引擎负责解析、校验与执行。

---

## DEVELOPMENT CONVENTIONS

- **Edition 2024**：使用最新 Rust 特性。全 workspace 统一 `version = "0.1.0"`。
- **unsafe**：当前项目已无 Lua 集成层活跃使用；除历史兼容原因外，**禁止**在新增代码中使用 `unsafe`。
- **语言**：游戏规格、UI 文本、注释文档使用中文；代码标识符、API、类型名使用英文。
- **AI 交互语言**：所有与项目相关的对话、分析、解释必须使用 **中文**。
- **日志**：使用 `tracing` 进行结构化日志记录，是显式要求。应用层启用 `tracing-subscriber`（带 `env-filter`）。
- **错误类型**：库 crate（`card-core`、`card-protocol`、`card-client`、`card-script`）使用 `thiserror` 定义精确错误枚举；应用 crate（`card-server`、`card-matchmaker`、`card-effect-cli`）使用 `anyhow` 进行错误传播。
- **测试风格**：
  - 大量单元测试内联在 `#[cfg(test)]` 模块中（如 `card-core/src/types/tests.rs`、`card-core/src/effect/tests.rs`）。
  - 集成测试使用 `ScriptedClient`/`PassAllClient` 驱动引擎进行确定性测试。
  - `card-server` 包含基于真实 WebSocket 的异步集成测试（`tests/` 目录）。
  - Python 测试脚本（`test_game_start.py`、`test_two_player_flow.py`）用于端到端流程验证。

---

## TESTING STRATEGIES

### 单元测试
- `card-core/src/types/tests.rs`：类型构造、序列化、CardId 格式校验。
- `card-core/src/effect/tests.rs`：效果 AST 解析、条件求值、中文文本生成。
- `card-protocol`：消息类型序列化/反序列化测试。
- `card-script`：JSON 加载、字段映射、错误处理。

### 集成测试
- `card-core/tests/engine_tests.rs`：引擎阶段推进、战斗结算、费用支付、连锁解算。
- `card-core/tests/deck_tests.rs`：卡组构建、洗牌、合法性校验。
- `card-script/tests/integration_test.rs`：JSON 端到端加载与注册表构建。
- `card-script/tests/card_effect_test.rs`：效果定义解析与动作验证。
- `card-server/tests/local_game_test.rs`：本地 AI 对战完整流程。
- ~~`card-server/tests/network_game_test.rs`~~：已移除（Raw TCP 路径废弃）。
- `card-server/tests/room_websocket_test.rs`：WebSocket 房间 join/leave/ready/开局。
- `card-server/tests/replay_test.rs`：录像录制与回放一致性校验。

### 端到端测试
- `test_game_start.py`：验证游戏启动、房间创建、状态同步。
- `test_two_player_flow.py`：验证双人对战完整流程。

---

## SECURITY & DEPLOYMENT

- **无 CI/CD**：当前没有 GitHub Actions、Dockerfile、Makefile 或 justfile。构建和部署依赖手动 `cargo` 命令和项目根目录的两个 shell 脚本。
- **服务器部署**：`restart_server.sh` 以 `nohup` 后台方式启动 `card-server`（release 模式），日志输出到 `logs/server.log`。
- **消息大小限制**：WebSocket 消息通过 tokio-tungstenite 处理，应用层可配置消息大小上限。
- **版本检查**：匹配服务器和 TCP 握手均包含版本校验，不匹配时拒绝连接。
- **脚本安全**：JSON 为纯数据格式，无执行语义，天然沙盒化。解析层仅做结构校验，不执行任何用户提供的逻辑代码。
- **超时机制**：系统支持单操作响应超时与单局总时间超时，均可在对局前通过 `GameRules` 配置，超时默认判负。

---

## ANTI-PATTERNS

- **DO NOT** 构建中央游戏服务器 — P2P 架构是硬性要求（当前 `card-server` 是主机本地服务器）。
- **DO NOT** 硬编码游戏规则 — 规则通过 `GameRules` 每局可配置。
- **DO NOT** 在对手视图中暴露手牌内容 — 必须仅暴露 `hand_count`。
- **DO NOT** 在 Godot 客户端中自行推演游戏状态 — 必须以服务器事件为唯一权威来源。
- **DO NOT** 修改 `scripts/`（Lua）目录下的卡牌定义 — 该目录已废弃，当前唯一数据源为 `scripts_json/`。
- **DO NOT** 在调试（debug）或修复 bug 时修改 `godot-client/addons/card-framework/` 下的代码 — 卡牌框架插件是基础组件，bug 应在业务层（`scenes/`、`managers/`、`scripts/`）修复，而非改动框架本身。

---

## KEY DOMAIN MODEL

### 卡牌类型
- **Character**（人物卡）— 有攻击力，战斗在前场（Front Zone）。
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
| Rust API 设计 | `docs/api.md` | 详尽类型、trait、消息定义（1015 行） |
| 系统架构 | `docs/architecture.md` | 各 crate 职责、Command/Event 模式（556 行） |
| 游戏开局流程 | `docs/game_start_flow.md` | 房间状态同步与信息隐藏（207 行） |
| 卡牌定义（JSON） | `scripts_json/S000/*.json` | 当前唯一权威数据源 |
| 卡组文件 | `desks/*.json` | 玩家卡组 |
| Godot 客户端网络 | `godot-client/managers/network_manager.gd` | WebSocket 房间协议（端口 8080） |
| Godot 客户端旧网络 | `godot-client/scripts/network.gd` | Legacy WebSocket（端口 7878） |
| Godot 客户端状态 | `godot-client/scripts/game_state.gd` | 游戏状态单例 |
| Godot 场景切换 | `godot-client/managers/scene_manager.gd` | 场景管理器 |
| 效果文本工具 | `godot-client/tools/card-effect-tool` | 运行 `build_effect_tool.sh` 生成 |

---

## NOTES FOR AGENTS

- 在修改任何 crate 之前，先阅读 `plan/ReadMe.md` 中相关游戏规则段落，确保逻辑符合规格。
- 如果修改卡牌定义，只需更新 `scripts_json/` 下的 JSON 文件。`scripts/`（Lua）目录已废弃。注意 `godot-client/scripts_json/` 是副本，视情况同步。
- `src/` 目录已为空，项目入口点分布在各 crate 的 `src/main.rs` 或 `src/lib.rs` 中。
- 游戏状态 `GameState` 是权威的唯一真实来源（single source of truth），任何前端都不应自行推演状态，只应根据事件更新表现层。
- 修改 `card-core` 的效果系统或类型定义后，需要检查 `card-script` 的解析器和 `card-effect-cli` 的文本生成器是否仍兼容。
- 修改网络协议时，需要同步更新 `card-protocol`、`card-server`、Godot 客户端的 `PROTOCOL.md` 以及 `network_manager.gd`。
- ~~`card-bevy`~~ 与 ~~`card-tui`~~ 已移除，活跃前端只有 `godot-client`。
