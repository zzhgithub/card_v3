# 卡牌对战游戏引擎 — 全功能第一版

## TL;DR

> **目标**: 构建完整的联网卡牌对战游戏引擎。Rust核心 + Lua卡牌脚本 + P2P TCP对战 + TUI客户端 + 回放/存档 + 匹配服务器。
>
> **交付物**:
> - 7个 Cargo workspace crate（core/script/protocol/server/client/tui/matchmaker）
> - 中文设计文档（architecture.md + api.md）
> - Lua 测试卡包（S000，8+张覆盖所有类型）
> - 可通过 TUI 进行本地/联网对战的完整游戏
>
> **预估工作量**: XL（35+ 个任务）
> **并行执行**: YES — 6 waves + FINAL
> **关键路径**: Workspace搭建 → 核心类型 → GameState → 引擎循环 → GameSession → TUI客户端 → 集成测试

---

## Context

### 原始需求
用户要求基于 `plan/ReadMe.md`（184行中文规格书）设计并构建一个商业级可扩展的联网卡片对战系统。包含完整的游戏引擎、Lua脚本系统、P2P网络对战、TUI客户端、回放系统、残局保存和匹配服务器。

### 访谈摘要
**关键决策**:
- Cargo Workspace 多 crate 架构
- 事件驱动 + 纯结构体（Command/Event 模式）
- 纯声明式 Lua（仅返回数据结构，Rust 解析执行）
- Rust 枚举 AST 实现条件表达式
- 命令同步网络模式（主机执行，广播状态变更）
- 对局前按需加载 Lua 脚本（启动建索引，对局前加载双方卡组）
- 实现后补测试

**规则补充确认（Metis审查后）**:
- 战斗结算：攻击力高者胜，低者进墓地，平局双方进墓地
- 初始手牌：5张
- 先手第一回合不抽卡
- 传奇卡可放前场或后场，可有攻击力
- 连锁中被动效果立即插入当前连锁栈顶

**技术选型**:
- `mlua` 0.11 (features: lua54, serialize) — Lua集成
- `tokio` — 异步运行时
- `serde` + `bincode` — 网络序列化
- `serde` + `serde_json` — 存档/配置
- `ratatui` + `crossterm` — TUI
- `tracing` + `tracing-subscriber` — 日志
- `thiserror` (库crate) + `anyhow` (应用crate) — 错误处理
- `rand` + `rand_chacha` — 确定性随机
- `tokio-tungstenite` — 匹配服务器 WebSocket
- `chrono` — 时间戳

### Metis 审查
**已处理的关键发现**:
- mlua sandbox() 仅限 Luau → 改用 StdLib 白名单 + 资源限制
- 战斗结算规则缺失 → 用户已确认
- 初始手牌/先手规则缺失 → 用户已确认
- 传奇卡行为未定义 → 用户已确认
- 连锁中触发处理 → 用户已确认

**范围锁定（Metis建议）**:
- AI = 随机合法动作（不做战略逻辑）
- i18n = 仅中文效果文本
- 存档 = 仅快照方式
- 客户端 = 仅 TUI（ClientApi trait 保留扩展性）
- 版本更新 = 仅版本号比较

---

## Work Objectives

### 核心目标
构建一个可运行的联网卡牌对战游戏，从零实现全部系统，通过 TUI 客户端可以进行本地和联网对战。

### 具体交付物
- `crates/card-core/` — 游戏引擎核心（纯逻辑）
- `crates/card-script/` — Lua 脚本加载/解析
- `crates/card-protocol/` — 网络协议与消息定义
- `crates/card-server/` — 游戏服务端（会话/网络/回放/存档）
- `crates/card-client/` — 客户端 API trait 定义
- `crates/card-tui/` — TUI 客户端实现
- `crates/card-matchmaker/` — 独立匹配服务器
- `scripts/S000/` — 测试卡包（8+张 Lua 脚本）
- `docs/architecture.md` — 中文架构设计文档
- `docs/api.md` — 中文 API 设计文档

### 完成标准
- [ ] `cargo build --workspace` 无错误
- [ ] `cargo test --workspace` 全部通过
- [ ] `cargo clippy --workspace` 无警告
- [ ] 两个 TestClient 可以自动对弈完成一局
- [ ] 两个进程可以通过 TCP 完成联网对战
- [ ] 回放录制 → 回放 → 最终 GameState 一致
- [ ] TUI 客户端可以显示完整游戏界面并操作

### Must Have
- 完整的回合流程（7个阶段）
- 效果/连锁系统（FILO解算 + 事件触发 + 免疫）
- 费用系统（手卡支付 + RealPoint + 费用区卡可用）
- 战斗系统（攻击宣言 + 伤害计算 + 直接攻击）
- Lua 纯声明式卡牌定义 + 沙盒化加载
- P2P TCP 联网对战
- 命令日志回放
- 快照存档/恢复
- TUI 客户端
- 匹配服务器（基础匹配 + 版本检查）
- 中文设计文档

### Must NOT Have（防护栏）
- ❌ AI 战略逻辑（仅随机合法动作）
- ❌ 多语言效果文本（仅中文）
- ❌ Bevy/Web/Unity 客户端（仅 TUI）
- ❌ 自动版本更新系统
- ❌ Command-log 方式的存档（仅快照）
- ❌ Lua 运行时条件评估（纯 Rust AST）
- ❌ unsafe 出现在 card-script 以外的 crate
- ❌ Lua VM 在 tokio async task 中使用
- ❌ 游戏逻辑在 Lua 脚本中执行
- ❌ 中心化游戏服务器架构

---

## Verification Strategy

> **零人工干预** — 所有验证由 agent 执行。

### 测试决策
- **基础设施**：无（从零搭建）
- **自动化测试**：实现后补测试
- **框架**：cargo test 内置
- **重点覆盖**：效果系统、连锁解算、战斗计算、Lua加载、网络协议

### QA 策略
- **引擎逻辑**：cargo test 单元测试 + 集成测试（TestClient 自动对弈）
- **网络**：双进程 TCP 集成测试
- **TUI**：tmux 启动 + 截屏验证
- **Lua**：正常卡 + 畸形卡 + 无限循环卡测试
- **回放**：录制 → 回放 → assert_eq!(final_state)

---

## Execution Strategy

### 并行执行波次

```
Wave 1 (立即启动 — 基础搭建, 8个任务):
├── Task 1:  Workspace 搭建 + 所有 crate 脚手架 [quick]
├── Task 2:  中文架构设计文档 [writing]
├── Task 3:  中文 API 设计文档 [writing]
├── Task 4:  核心类型定义 card-core/src/types/ [quick]
├── Task 5:  效果 AST 类型 card-core/src/effect/ [unspecified-high]
├── Task 6:  协议消息类型 card-protocol/src/message.rs [quick]
├── Task 7:  游戏规则配置 card-core/src/rules/ [quick]
└── Task 8:  各 crate 错误类型定义 [quick]

Wave 2 (Wave 1 后 — 核心系统, 7个任务):
├── Task 9:  GameState + PlayerZones (depends: 4, 7) [unspecified-high]
├── Task 10: Condition AST 求值器 (depends: 5) [deep]
├── Task 11: ScriptIndex + CardRegistry (depends: 4, 5) [unspecified-high]
├── Task 12: Lua parser: table→CardDefinition (depends: 4, 5, 11) [deep]
├── Task 13: Lua 沙盒配置 (depends: 11) [quick]
├── Task 14: TCP 编解码 TcpConnection (depends: 6) [unspecified-high]
└── Task 15: ClientApi trait + VisibleGameState (depends: 4, 6, 9) [unspecified-high]

Wave 3 (Wave 2 后 — 引擎核心, 6个任务):
├── Task 16: 回合状态机 phase runner (depends: 9, 15) [deep]
├── Task 17: TriggerChecker 效果触发扫描 (depends: 5, 9, 10) [deep]
├── Task 18: ChainManager FILO连锁解算 (depends: 5, 9, 15, 17) [ultrabrain]
├── Task 19: ActionExecutor 动作执行+免疫 (depends: 5, 9, 17) [deep]
├── Task 20: Modifier 系统 (depends: 4, 9) [unspecified-high]
└── Task 21: GameEngine 主循环集成 (depends: 16-20) [ultrabrain]

Wave 4 (Wave 3 后 — 应用层, 7个任务):
├── Task 22: GameSession 会话管理 (depends: 21, 15) [unspecified-high]
├── Task 23: RemoteClient TCP代理 (depends: 14, 15) [unspecified-high]
├── Task 24: TCP 服务端 listen+accept (depends: 14, 23) [unspecified-high]
├── Task 25: 回放系统 ReplayData+ReplayClient (depends: 15, 21) [unspecified-high]
├── Task 26: 快照存档/恢复 (depends: 9, 21) [unspecified-high]
├── Task 27: AiClient 随机合法动作 (depends: 15, 21) [quick]
└── Task 28: S000 测试卡包 Lua 脚本 (depends: 5, 12) [unspecified-high]

Wave 5 (Wave 4 后 — TUI + 匹配, 5个任务):
├── Task 29: TUI 应用骨架 + 主循环 (depends: 15, 22) [visual-engineering]
├── Task 30: TUI 游戏板面渲染 (depends: 29) [visual-engineering]
├── Task 31: TUI 输入处理+操作选择 (depends: 29, 30) [visual-engineering]
├── Task 32: 匹配服务器 (depends: 6) [unspecified-high]
└── Task 33: TUI 匹配客户端集成 (depends: 29, 32) [unspecified-high]

Wave 6 (Wave 5 后 — 测试+集成, 5个任务):
├── Task 34: 核心引擎单元测试 (depends: 21) [deep]
├── Task 35: Lua 加载单元测试 (depends: 12, 13, 28) [unspecified-high]
├── Task 36: 集成测试:本地对弈 (depends: 21, 27, 28) [deep]
├── Task 37: 集成测试:联网对战 (depends: 22, 23, 24) [deep]
└── Task 38: 回放正确性测试 (depends: 25, 36) [deep]

Wave FINAL (所有任务后 — 独立审查, 4个并行):
├── Task F1: 计划合规审计 (oracle)
├── Task F2: 代码质量审查 (unspecified-high)
├── Task F3: 完整 QA (unspecified-high)
└── Task F4: 范围忠实度检查 (deep)

关键路径: T1 → T4 → T9 → T16 → T21 → T22 → T29 → T36 → F1-F4
并行加速: ~65% 快于串行
最大并发: 8 (Wave 1)
```

### 依赖矩阵

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | — | 2-8 | 1 |
| 2 | 1 | — | 1 |
| 3 | 1 | — | 1 |
| 4 | 1 | 5,6,7,8,9,11,15,20 | 1 |
| 5 | 1 | 10,11,12,17,19,28 | 1 |
| 6 | 1 | 14,15,32 | 1 |
| 7 | 1 | 9 | 1 |
| 8 | 1 | — | 1 |
| 9 | 4,7 | 15,16,17,19,20,26 | 2 |
| 10 | 5 | 17 | 2 |
| 11 | 4,5 | 12,13 | 2 |
| 12 | 4,5,11 | 28,35 | 2 |
| 13 | 11 | 35 | 2 |
| 14 | 6 | 23,24 | 2 |
| 15 | 4,6,9 | 16,18,22,23,25,27,29 | 2 |
| 16 | 9,15 | 21 | 3 |
| 17 | 5,9,10 | 18,19,21 | 3 |
| 18 | 5,9,15,17 | 21 | 3 |
| 19 | 5,9,17 | 21 | 3 |
| 20 | 4,9 | 21 | 3 |
| 21 | 16-20 | 22,25,26,27,34,36 | 3 |
| 22 | 21,15 | 29,37 | 4 |
| 23 | 14,15 | 24,37 | 4 |
| 24 | 14,23 | 37 | 4 |
| 25 | 15,21 | 38 | 4 |
| 26 | 9,21 | — | 4 |
| 27 | 15,21 | 36 | 4 |
| 28 | 5,12 | 35,36 | 4 |
| 29 | 15,22 | 30,31,33 | 5 |
| 30 | 29 | 31 | 5 |
| 31 | 29,30 | — | 5 |
| 32 | 6 | 33 | 5 |
| 33 | 29,32 | — | 5 |
| 34 | 21 | — | 6 |
| 35 | 12,13,28 | — | 6 |
| 36 | 21,27,28 | 38 | 6 |
| 37 | 22,23,24 | — | 6 |
| 38 | 25,36 | — | 6 |

### Agent 分派摘要

| Wave | 任务数 | 分类分布 |
|------|--------|---------|
| 1 | 8 | 5×quick, 2×writing, 1×unspecified-high |
| 2 | 7 | 2×deep, 4×unspecified-high, 1×quick |
| 3 | 6 | 3×deep, 2×ultrabrain, 1×unspecified-high |
| 4 | 7 | 5×unspecified-high, 1×quick, 1×unspecified-high |
| 5 | 5 | 3×visual-engineering, 2×unspecified-high |
| 6 | 5 | 3×deep, 2×unspecified-high |
| FINAL | 4 | 1×oracle, 2×unspecified-high, 1×deep |

---

## TODOs

- [x] 1. Workspace 搭建 + 所有 crate 脚手架

  **What to do**:
  - 将根 `Cargo.toml` 改为 workspace 配置，members 包含 7 个 crate
  - 在 `crates/` 下创建 7 个 crate 目录: card-core, card-script, card-protocol, card-server, card-client, card-tui, card-matchmaker
  - 每个 crate 的 `Cargo.toml` 配置正确的 name、edition=2024、内部依赖关系
  - 依赖关系: card-core ← card-script/card-protocol/card-server/card-client; card-protocol ← card-server; card-client ← card-tui/card-server; card-protocol ← card-matchmaker
  - 每个 crate 的 lib.rs/main.rs 包含模块骨架声明
  - 创建 `scripts/` 目录和 `docs/` 目录
  - 根 Cargo.toml workspace dependencies 统一管理: serde, tokio, mlua, bincode, tracing, thiserror, anyhow, ratatui, crossterm, rand, chrono 等
  - mlua features: `["lua54", "serialize"]`
  - 移除原 src/main.rs（已被 crates/ 替代）

  **Must NOT do**:
  - 不在任何 crate 中添加实质业务逻辑
  - 不使用 `luau` feature

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2-8)
  - **Blocks**: Tasks 2-8 (所有后续任务依赖 workspace 存在)
  - **Blocked By**: None

  **References**:
  - `Cargo.toml` — 当前根配置，需要改为 workspace
  - `plan/ReadMe.md:106-121` — 架构设计原则
  - `.sisyphus/drafts/game-engine-design.md` — 完整 crate 结构设计

  **Acceptance Criteria**:
  - [ ] `cargo build --workspace` 成功编译
  - [ ] 7 个 crate 各自的 `cargo build -p <name>` 成功
  - [ ] 内部依赖关系正确（card-server 可以 use card_core）

  **QA Scenarios**:
  ```
  Scenario: Workspace 全编译
    Tool: Bash
    Steps:
      1. cargo build --workspace
      2. 检查退出码 = 0
      3. 逐个检查: cargo build -p card-core, cargo build -p card-script, ...
    Expected Result: 所有 crate 编译成功
    Evidence: .sisyphus/evidence/task-1-workspace-build.txt

  Scenario: 目录结构完整
    Tool: Bash
    Steps:
      1. ls crates/ — 确认 7 个子目录
      2. ls scripts/ — 确认目录存在
      3. ls docs/ — 确认目录存在
      4. cat crates/card-core/Cargo.toml — 确认 edition = "2024"
    Expected Result: 所有目录和文件就位
    Evidence: .sisyphus/evidence/task-1-structure.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat: scaffold workspace with 7 crates`
  - Files: `Cargo.toml`, `crates/*/Cargo.toml`, `crates/*/src/*`

- [x] 2. 中文架构设计文档

  **What to do**:
  - 创建 `docs/architecture.md`，中文撰写
  - 内容基于本次设计讨论的全部内容，涵盖:
    - 项目概述和目标
    - 整体架构图（Crate 划分 + 数据流）
    - 核心设计原则（纯逻辑引擎、Command/Event模式、Lua声明式、客户端抽象）
    - 各 crate 职责说明
    - 技术选型及理由
    - 游戏规则补充（Metis审查后确认的战斗结算、先手规则等）
    - 效果系统架构（触发/条件AST/动作/修饰器/特殊效果）
    - 连锁解算流程（FILO + 瞬时卡处理 + 栈顶插入被动效果）
    - 网络架构（P2P + RemoteClient代理 + 命令同步）
    - 回放/存档架构
    - 可配置规则系统
  - 使用 Mermaid 或 ASCII 图表辅助说明

  **Must NOT do**:
  - 不包含英文（全部中文）
  - 不包含具体代码实现（只有架构级伪代码）

  **Recommended Agent Profile**:
  - **Category**: `writing`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1,3-8)
  - **Blocks**: None
  - **Blocked By**: Task 1 (docs/ 目录存在)

  **References**:
  - `plan/ReadMe.md` — 完整游戏规格书（中文）
  - `.sisyphus/drafts/game-engine-design.md` — 所有设计决策和Metis审查结论
  - `.sisyphus/plans/card-game-engine.md` — 本计划的 Context 部分

  **Acceptance Criteria**:
  - [ ] `docs/architecture.md` 存在且内容完整
  - [ ] 涵盖上述所有章节
  - [ ] 全部中文

  **QA Scenarios**:
  ```
  Scenario: 文档完整性
    Tool: Bash
    Steps:
      1. cat docs/architecture.md | wc -l — 确认行数 > 200
      2. grep "效果系统" docs/architecture.md — 确认包含关键章节
      3. grep "连锁" docs/architecture.md
      4. grep "P2P" docs/architecture.md
    Expected Result: 所有关键章节存在
    Evidence: .sisyphus/evidence/task-2-doc-check.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `docs: add Chinese architecture design document`
  - Files: `docs/architecture.md`

- [x] 3. 中文 API 设计文档

  **What to do**:
  - 创建 `docs/api.md`，中文撰写
  - 内容涵盖所有公开 API 的设计:
    - 核心类型定义（CardId, InstanceId, EffectKey, PlayerId 等所有 newtype）
    - 卡片类型枚举（CardType, StrategyKind, ItemKind, Property, Category）
    - CardDefinition 结构体完整字段说明
    - Effect 系统完整类型（Trigger, EventTrigger, Condition AST, Action, Modifier, CostRequirement）
    - GameState 结构体及所有子结构
    - Command 枚举（所有玩家操作）
    - GameEvent 枚举（所有游戏事件）
    - NetworkMessage 枚举（所有网络消息）
    - ClientApi trait 完整接口
    - GameRules 可配置字段
    - Lua 卡牌脚本格式规范（table 结构示例）
  - 每个类型/接口包含: 用途说明 + 字段说明 + 使用示例

  **Must NOT do**:
  - 不包含实现细节（只有接口定义）

  **Recommended Agent Profile**:
  - **Category**: `writing`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `plan/ReadMe.md` — 规格书中的效果定义(132-173行)
  - `.sisyphus/drafts/game-engine-design.md` — 完整类型设计

  **Acceptance Criteria**:
  - [ ] `docs/api.md` 存在且内容完整
  - [ ] 所有公开类型有中文说明
  - [ ] 包含 Lua 脚本格式示例

  **QA Scenarios**:
  ```
  Scenario: API 文档完整性
    Tool: Bash
    Steps:
      1. cat docs/api.md | wc -l — 确认行数 > 300
      2. grep "ClientApi" docs/api.md
      3. grep "CardDefinition" docs/api.md
      4. grep "GameEvent" docs/api.md
      5. grep "Lua" docs/api.md
    Expected Result: 所有关键 API 类型有文档
    Evidence: .sisyphus/evidence/task-3-api-check.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `docs: add Chinese API design document`
  - Files: `docs/api.md`

- [x] 4. 核心类型定义 card-core/src/types/

  **What to do**:
  - 创建 `crates/card-core/src/types/mod.rs` 及子模块
  - 实现所有基础类型（均 derive Serialize, Deserialize, Clone, Debug）:
    - `CardId(pub String)` — 卡片定义ID，实现 Display 解析 S[pack]-[type]-[num] 格式
    - `InstanceId(pub u32)` — 卡片实例ID
    - `EffectKey(pub String)` — 特殊效果全局键
    - `PlayerId { Player1, Player2 }` — 实现 opponent() 方法
    - `CardType`, `StrategyKind`, `ItemKind` — 卡片类型枚举
    - `Property { Rational, Divine, Spiritual }` — 卡片属性
    - `Category { Math, Science, Literature, Philosophy, Mystery }` — 卡片范畴
    - `Zone` 枚举（Deck, Hand, Front(usize), Back(usize), CostZone, Grave）
    - `ZoneLocation { player: PlayerId, zone: Zone }` — 完整区域位置
    - `PlayerRef { Self_, Opponent }` — 相对玩家引用
    - `CardRef { This, ByInstanceId(InstanceId), BySlot(Zone, usize) }` — 卡片引用
    - `TargetRef` 枚举 — 效果目标引用
    - `CardDefinition` 结构体 — 完整卡牌定义
    - `CardFilter` 结构体 — 卡片过滤器
  - 所有 newtype 实现 `Hash, Eq, PartialEq`
  - `GameState` 必须实现 `PartialEq + Clone`（回放验证用）

  **Must NOT do**:
  - 不包含游戏逻辑
  - 不依赖 mlua 或 tokio

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 5,6,7,8,9,11,15,20
  - **Blocked By**: Task 1

  **References**:
  - `plan/ReadMe.md:12-30` — 卡片属性定义
  - `plan/ReadMe.md:40-48` — 区域定义
  - `plan/ReadMe.md:97-103` — 卡片ID格式
  - `.sisyphus/drafts/game-engine-design.md` — 完整类型设计（含5项修正）

  **Acceptance Criteria**:
  - [ ] `cargo build -p card-core` 成功
  - [ ] 所有 newtype 支持 serde 序列化/反序列化
  - [ ] CardId::parse("S001-C-001") 能正确解析

  **QA Scenarios**:
  ```
  Scenario: 类型编译和序列化
    Tool: Bash
    Steps:
      1. cargo build -p card-core
      2. 在 card-core 中添加临时测试: serde_json::to_string(&CardId("S001-C-001".into()))
      3. cargo test -p card-core
    Expected Result: 编译通过，序列化/反序列化正确
    Evidence: .sisyphus/evidence/task-4-types-build.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(core): define core domain types with serde support`
  - Files: `crates/card-core/src/types/*.rs`

- [x] 5. 效果 AST 类型定义 card-core/src/effect/

  **What to do**:
  - 创建 `crates/card-core/src/effect/mod.rs` 及子模块
  - 实现完整效果类型系统（均 derive Serialize, Deserialize, Clone, Debug）:
    - `Effect` 结构体 — trigger, optional, activation_limit, conditions, choices, actions, costs
    - `Trigger` 枚举 — 阶段触发(TurnStart/OwnMainPhase/...) + 动作触发(OnSummon/OnAttack/OnExpose/OnDestroy) + 事件触发(OnEvent(EventTrigger))
    - `EventTrigger` 枚举 — RealPointChanged/RealPointOverflow/HpChanged/AnyCardDestroyed/AnyCardSummoned/AnyEffectActivated/AnyCardExposed/CardDrawn，各含 PlayerRef/CardFilter
    - `ActivationLimit` 枚举 — OncePerTurn/OncePerTurnSameName
    - `Condition` 枚举 AST — And(Vec)/Or(Vec)/Not(Box)/Compare{left,op,right}/CardIsOnField{card}
    - `CompareOp` 枚举 — Gt/Lt/Ge/Le/Eq/Ne
    - `ValueExpr` 枚举 — Literal/HandCount/CostZoneCount/CostZonePropertyCount/FrontFieldCount/BackFieldCount/RealPoint/Hp/HighestCostOnField/AttackPower
    - `Action` 枚举 — Draw/Damage/Destroy/SummonFromZone/ReturnToHand/SendToGrave/ModifyAttack/GainRealPoint/HealHp/Discard/ApplyModifier/RemoveModifier/Special{key:EffectKey}
    - `Choice` 结构体 + `TargetType` + `TargetFilter`
    - `CostRequirement` 枚举 — SendFieldCardToGrave/DiscardHand/SendCostZoneToGrave
    - `Modifier` 枚举 — AttackBoost/ImmuneToCardType/ImmuneToProperty/ImmuneToDestruction/ImmuneToTargeting/CanAttackDirectly/ExtraAttackPerTurn/CannotAttack/Special{key}
    - `AppliedModifier` 结构体 — modifier + source InstanceId + duration
    - `ModifierDuration` 枚举 — WhileSourceOnField/UntilEndOfTurn/Permanent/TurnCount(u32)

  **Must NOT do**:
  - 不包含求值逻辑（纯类型定义）
  - 不依赖外部 crate（只用 serde）

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 10,11,12,17,19,28
  - **Blocked By**: Task 1 (crate 存在), Task 4 (基础类型如 InstanceId, PlayerRef)

  **References**:
  - `plan/ReadMe.md:132-184` — 效果系统完整定义和7个示例
  - `.sisyphus/drafts/game-engine-design.md` — 修正后的 Effect 类型设计（含 EventTrigger、Modifier、Special）

  **Acceptance Criteria**:
  - [ ] `cargo build -p card-core` 成功
  - [ ] 能用 serde_json 序列化/反序列化一个包含嵌套 Condition 的 Effect
  - [ ] 规格书 7 个效果示例都能用这些类型表达

  **QA Scenarios**:
  ```
  Scenario: 效果类型表达力验证
    Tool: Bash
    Steps:
      1. cargo build -p card-core
      2. 编写测试: 用代码构造规格书例1"登场时登场另一张卡"的 Effect 实例
      3. 编写测试: 用代码构造规格书例4a"不受策略卡效果影响"的 Modifier
      4. 编写测试: 用代码构造规格书例4b"对手RP增加时返回手卡"的 EventTrigger + Condition
      5. cargo test -p card-core
    Expected Result: 所有 7 个规格书示例均可表达
    Evidence: .sisyphus/evidence/task-5-effect-types.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(core): define effect system AST types`
  - Files: `crates/card-core/src/effect/*.rs`

- [x] 6. 协议消息类型 card-protocol/src/message.rs

  **What to do**:
  - 实现 `Command` 枚举 — 所有玩家操作（PlayCard, ActivateEffect, DeclareAttack, DirectAttackRealPoint, ChainActivate, ChainPass, SelectRecoveryCards, SelectTargets, SelectCards, Surrender）
  - 实现 `CostPayment` 结构体 — hand_cards + real_point
  - 实现 `AttackTarget` 枚举 — FrontSlot(usize) / DirectAttack
  - 实现 `AvailableAction` 枚举 — 向客户端展示的可选操作
  - 实现 `GameEvent` 枚举 — 所有游戏事件（CardMoved, CardSummoned, CardDestroyed, HpChanged, RealPointChanged, RealPointOverflow, AttackModified, PhaseChanged, TurnChanged, DrawCard, EffectActivated, ChainStarted, ChainLink, ChainResolving, ChainComplete, RequestAction, RequestTargetSelection, RequestCardSelection, GameOver）
  - 实现 `GameOverReason` 枚举 — HpZero/DeckOut/Surrender/Timeout
  - 实现 `NetworkMessage` 枚举 — Hello/HelloAck/DeckSubmit/DeckAccepted/DeckRejected/GameStart/EventNotification/RequestAction/RequestTargetSelection/RequestCardSelection/CommandResponse/TargetResponse/CardResponse/Ping/Pong/Disconnect
  - 所有类型 derive `Serialize, Deserialize, Clone, Debug`
  - card-protocol 依赖 card-core（使用其类型）

  **Must NOT do**:
  - 不包含编解码逻辑（纯类型定义）
  - 不包含网络IO

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 14, 15, 32
  - **Blocked By**: Task 1, Task 4 (使用 InstanceId, PlayerId 等)

  **References**:
  - `.sisyphus/drafts/game-engine-design.md` — Command/Event/NetworkMessage 完整设计

  **Acceptance Criteria**:
  - [ ] `cargo build -p card-protocol` 成功
  - [ ] bincode 和 serde_json 均可序列化 Command/GameEvent/NetworkMessage

  **QA Scenarios**:
  ```
  Scenario: 协议消息序列化
    Tool: Bash
    Steps:
      1. cargo build -p card-protocol
      2. 编写测试: bincode序列化 Command::PlayCard → 反序列化 → assert_eq
      3. 编写测试: serde_json序列化 GameEvent::GameOver → 确认 JSON 可读
      4. cargo test -p card-protocol
    Expected Result: 所有消息类型可正确序列化/反序列化
    Evidence: .sisyphus/evidence/task-6-protocol-serde.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(protocol): define command, event, and network message types`
  - Files: `crates/card-protocol/src/message.rs`, `crates/card-protocol/src/types.rs`

- [x] 7. 游戏规则配置 card-core/src/rules/

  **What to do**:
  - 实现 `GameRules` 结构体（derive Serialize, Deserialize, Clone, Debug, PartialEq）:
    - initial_hp: u8 (default 5)
    - max_hp: u8 (default 6)
    - max_hand: usize (default 20)
    - max_cost_zone: usize (default 6)
    - max_real_point: u8 (default 6)
    - deck_size_range: (usize, usize) (default (40, 60))
    - max_same_card: usize (default 3)
    - max_legendary: usize (default 5)
    - initial_hand_size: usize (default 5)
    - first_player_draws: bool (default false)
    - operation_timeout: Duration (default 30s)
    - total_game_timeout: Duration (default 3600s)
  - 实现 `GameRules::default()` 返回标准规则
  - 实现卡组校验函数 `validate_deck(deck: &[CardId], rules: &GameRules, registry: &CardRegistry) -> Result<()>`
    - 检查卡组大小在范围内
    - 检查同名卡不超过限制
    - 检查传奇卡不超过限制
    - 检查所有 CardId 在 registry 中存在

  **Must NOT do**:
  - 不硬编码规则值（全部可配置）

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 9
  - **Blocked By**: Task 1, Task 4

  **References**:
  - `plan/ReadMe.md:6-10` — 卡组限制规则
  - `plan/ReadMe.md:87-94` — 游戏内限制
  - `plan/ReadMe.md:120-121` — 超时和自定义规则

  **Acceptance Criteria**:
  - [ ] `GameRules::default()` 返回符合规格书的标准值
  - [ ] `validate_deck` 正确拒绝超限卡组

  **QA Scenarios**:
  ```
  Scenario: 规则校验
    Tool: Bash
    Steps:
      1. 编写测试: 40张卡组通过验证
      2. 编写测试: 39张卡组被拒绝（低于最小值）
      3. 编写测试: 同名卡4张被拒绝
      4. 编写测试: 传奇卡6张被拒绝
      5. cargo test -p card-core
    Expected Result: 合法卡组通过，非法卡组被正确拒绝
    Evidence: .sisyphus/evidence/task-7-rules-validate.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(core): add configurable game rules and deck validation`
  - Files: `crates/card-core/src/rules/*.rs`

- [x] 8. 各 crate 错误类型定义

  **What to do**:
  - `card-core`: 定义 `CoreError` (thiserror)
    - InvalidCardId, CardNotFound, InvalidZone, RuleViolation, InvalidCommand, InvalidTarget, EffectError, GameOver
  - `card-script`: 定义 `ScriptError` (thiserror)
    - LuaError(mlua::Error), ParseError, CardNotFound(CardId), SandboxViolation, ScriptTimeout, InvalidCardDefinition
  - `card-protocol`: 定义 `ProtocolError` (thiserror, 可序列化)
    - SerializationError, DeserializationError, InvalidMessage, VersionMismatch
  - `card-server`: 用 `anyhow` 做顶层错误传播
  - `card-tui`: 用 `anyhow` 做顶层错误传播
  - `card-matchmaker`: 用 `anyhow` 做顶层错误传播
  - 库 crate（core/script/protocol/client）仅用 `thiserror`
  - 应用 crate（server/tui/matchmaker）用 `anyhow`

  **Must NOT do**:
  - 不在库 crate 中使用 `anyhow`
  - `ProtocolError` 不依赖 `anyhow`（需要可跨网络序列化）

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: None (被所有后续任务隐式使用)
  - **Blocked By**: Task 1, Task 4

  **References**:
  - Metis 审查报告中的错误分层建议

  **Acceptance Criteria**:
  - [ ] 所有 crate 编译通过
  - [ ] `ProtocolError` 可以 bincode 序列化

  **QA Scenarios**:
  ```
  Scenario: 错误类型编译
    Tool: Bash
    Steps:
      1. cargo build --workspace
      2. 确认 card-core 不依赖 anyhow
      3. 确认 card-server 使用 anyhow::Result
    Expected Result: 编译通过，依赖分层正确
    Evidence: .sisyphus/evidence/task-8-errors.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat: define error types for all crates with proper layering`
  - Files: `crates/*/src/error.rs`

- [x] 9. GameState + PlayerZones + PlayerState

  **What to do**:
  - 创建 `crates/card-core/src/state/mod.rs` 及子模块
  - `CardInstance` 结构体 — instance_id(InstanceId), definition_id(CardId), current_attack(Option<u16>), modifiers(Vec<AppliedModifier>), attacked_this_turn(bool), turn_summoned(u32)
  - `PlayerZones` 结构体 — deck(Vec<CardInstance>), hand(Vec<CardInstance>), front([Option<CardInstance>; 5]), back([Option<CardInstance>; 5]), cost_zone(Vec<CardInstance>), grave(Vec<CardInstance>)
  - `PlayerZones` 方法: all_cards() 迭代器, card_by_instance_id(), remove_card(), add_card_to_zone(), zone_count(), is_zone_full()
  - `PlayerState` 结构体 — id(PlayerId), zones(PlayerZones), hp(u8), real_point(u8)
  - `GameState` 结构体 — players([PlayerState; 2]), turn_player(PlayerId), phase(Phase), turn_number(u32), chain(ChainStack), rules(GameRules), card_registry(Arc<CardRegistry>), rng_seed(u64), instance_counter(u32)
  - `GameState` 方法: current_player(), opponent_player(), next_instance_id(), get_card(), move_card()
  - `Phase` 枚举 + `BattleStep` 子枚举
  - `ChainStack` 结构体 + `ChainLink` 结构体
  - `CardRegistry` 结构体 — cards(HashMap<CardId, CardDefinition>)，get()方法
  - 所有均 derive Clone, Debug, Serialize, Deserialize; GameState 额外 derive PartialEq

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 2)
  - **Blocks**: Tasks 15, 16, 17, 19, 20, 26
  - **Blocked By**: Tasks 4, 7

  **References**:
  - `plan/ReadMe.md:40-48` — 区域定义和限制
  - `plan/ReadMe.md:87-94` — 数值限制
  - `.sisyphus/drafts/game-engine-design.md` — GameState 完整设计

  **Acceptance Criteria**:
  - [ ] `cargo build -p card-core` 成功
  - [ ] GameState 可以 serde_json 序列化/反序列化（存档用）
  - [ ] `assert_eq!(state1, state2)` 可用于比较两个 GameState（回放验证）

  **QA Scenarios**:
  ```
  Scenario: GameState 创建和序列化
    Tool: Bash
    Steps:
      1. 编写测试: 创建一个初始 GameState（空卡组）
      2. serde_json::to_string(&state) 成功
      3. 反序列化后 assert_eq 与原始状态
      4. cargo test -p card-core
    Expected Result: 状态可正确创建和序列化
    Evidence: .sisyphus/evidence/task-9-state.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(core): implement game state, player zones, and card instances`
  - Files: `crates/card-core/src/state/*.rs`

- [x] 10. Condition AST 求值器

  **What to do**:
  - 创建 `crates/card-core/src/effect/evaluator.rs`
  - 实现 `evaluate_condition(cond: &Condition, state: &GameState, perspective: PlayerId, source: &CardInstance) -> bool`
    - And: 所有子条件为真
    - Or: 任一子条件为真
    - Not: 取反
    - Compare: 求值左右 ValueExpr 并比较
    - CardIsOnField: 检查卡片是否在场上
  - 实现 `resolve_value(expr: &ValueExpr, state: &GameState, perspective: PlayerId, source: &CardInstance) -> i32`
    - Literal: 直接返回
    - HandCount: 对应玩家手牌数
    - CostZoneCount: 费用区卡数
    - CostZonePropertyCount: 费用区指定属性卡数（如"理性值"）
    - FrontFieldCount/BackFieldCount: 场上卡数
    - RealPoint/Hp: 对应数值
    - HighestCostOnField: 遍历前场+后场找最高费用
    - AttackPower: 指定卡的当前攻击力
  - `PlayerRef::Self_/Opponent` 根据 perspective 解析为具体 PlayerId
  - 充分的单元测试覆盖所有 Condition 分支

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 2)
  - **Blocks**: Task 17
  - **Blocked By**: Task 5

  **References**:
  - `plan/ReadMe.md:159-162` — 条件示例
  - `.sisyphus/drafts/game-engine-design.md` — Condition AST 和 ValueExpr 设计

  **Acceptance Criteria**:
  - [ ] "对手手卡>自己手卡" 条件能正确求值
  - [ ] "费用区理性卡数>3" 条件能正确求值
  - [ ] 嵌套 And/Or/Not 组合正确求值

  **QA Scenarios**:
  ```
  Scenario: 条件求值正确性
    Tool: Bash
    Steps:
      1. 构造 GameState: P1手牌5张, P2手牌3张
      2. 评估 Condition::Compare(HandCount(Opponent), Gt, HandCount(Self_)) — P1视角应为 false
      3. 评估同条件 P2视角应为 true
      4. 构造费用区有理性卡4张的状态
      5. 评估 CostZonePropertyCount(Self_, Rational) > 3 — 应为 true
      6. cargo test -p card-core
    Expected Result: 所有条件求值结果正确
    Evidence: .sisyphus/evidence/task-10-evaluator.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(core): implement condition AST evaluator`
  - Files: `crates/card-core/src/effect/evaluator.rs`

- [x] 11. ScriptIndex + CardRegistry

  **What to do**:
  - 创建 `crates/card-script/src/loader.rs`
  - `ScriptIndex` 结构体 — entries(HashMap<CardId, PathBuf>)
  - `ScriptIndex::scan(scripts_dir: &Path) -> Result<Self>` — 扫描目录建立索引
    - 遍历 scripts/ 下所有 .lua 文件
    - 从文件名解析 CardId（如 S001-C-001.lua → CardId("S001-C-001")）
    - 不打开文件内容，仅建立路径映射
  - `CardRegistry` 已在 card-core 中定义（HashMap<CardId, CardDefinition>），此任务实现构建逻辑
  - `ScriptLoader` 结构体 — 持有 ScriptIndex
  - `ScriptLoader::load_for_game(deck1, deck2) -> Result<CardRegistry>` — 核心加载入口
    - 合并双方卡组的所有 CardId
    - 用 BFS/队列加载（处理递归引用）
    - 调用 Lua VM 执行每个脚本 → 获取 table（parser 在 Task 12 实现）
    - 加载完毕后 Lua VM 自动 drop 释放

  **Must NOT do**:
  - 不在此任务实现 Lua table → CardDefinition 的解析（那是 Task 12）
  - 此任务只负责文件扫描、索引、加载调度

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 2)
  - **Blocks**: Tasks 12, 13
  - **Blocked By**: Tasks 4, 5

  **References**:
  - `.sisyphus/drafts/game-engine-design.md` — ScriptIndex/ScriptLoader 设计
  - `plan/ReadMe.md:97-103` — CardId 命名规则

  **Acceptance Criteria**:
  - [ ] `ScriptIndex::scan("scripts/")` 能正确扫描目录
  - [ ] 文件名 → CardId 解析正确

  **QA Scenarios**:
  ```
  Scenario: 索引扫描
    Tool: Bash
    Steps:
      1. 创建 scripts/S000/S000-C-001.lua（空文件）
      2. ScriptIndex::scan("scripts/") 返回包含 CardId("S000-C-001") 的索引
      3. cargo test -p card-script
    Expected Result: 索引正确建立
    Evidence: .sisyphus/evidence/task-11-index.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(script): implement script index scanning and loader framework`
  - Files: `crates/card-script/src/loader.rs`

- [x] 12. Lua parser: table → CardDefinition

  **What to do**:
  - 创建 `crates/card-script/src/parser.rs`
  - 使用 mlua 的 `LuaSerdeExt`（feature = serialize）或手动 table 遍历
  - 实现 `parse_card_definition(table: &Table) -> Result<CardDefinition>`
    - 解析基础字段: id, name, card_type, property, category, tags, cost
    - 解析可选字段: attack（仅人物卡）, strategy_kind, item_kind
    - 解析 effects HashMap: 遍历 effects table 的每个 key-value
    - 每个 effect 解析为 Effect 结构体：trigger, optional, activation_limit, conditions, choices, actions, costs
    - 条件解析: 递归解析嵌套 table 为 Condition AST（And/Or/Not/Compare）
    - 动作解析: table → Action 枚举变体
    - Special 效果: 遇到 `type = "special"` 时解析为 `Action::Special { key }`
  - 实现 `extract_referenced_cards(def: &CardDefinition) -> Vec<CardId>` — 提取效果中引用的其他卡ID（递归加载用）
  - 定义 Lua 卡牌脚本的标准格式并在注释中记录

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 2, after T11)
  - **Blocks**: Tasks 28, 35
  - **Blocked By**: Tasks 4, 5, 11

  **References**:
  - `plan/ReadMe.md:12-30` — 卡片属性
  - `plan/ReadMe.md:132-184` — 效果定义和示例
  - `.sisyphus/drafts/game-engine-design.md` — Lua 脚本格式和解析设计
  - mlua docs: `LuaSerdeExt` trait, `Table` API

  **Acceptance Criteria**:
  - [ ] 能正确解析一个包含完整 Effect 的 Lua table
  - [ ] 条件 AST 的嵌套 And/Or 能正确递归解析
  - [ ] 引用卡ID提取正确

  **QA Scenarios**:
  ```
  Scenario: 完整卡牌解析
    Tool: Bash
    Steps:
      1. 创建测试 Lua 脚本包含: id, name, card_type="character", property="rational", cost=3, attack=500, effects 含 on_summon 触发
      2. 用 mlua 加载执行获取 table
      3. parse_card_definition 返回正确的 CardDefinition
      4. 验证 effects["e1"].trigger == Trigger::OnSummon
      5. cargo test -p card-script
    Expected Result: Lua table 正确解析为 Rust 类型
    Evidence: .sisyphus/evidence/task-12-parser.txt

  Scenario: 畸形脚本处理
    Tool: Bash
    Steps:
      1. 创建缺少 "id" 字段的 Lua 脚本
      2. parse_card_definition 返回 Err(ScriptError::InvalidCardDefinition)
    Expected Result: 错误被正确捕获，不 panic
    Evidence: .sisyphus/evidence/task-12-parser-error.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(script): implement Lua table to CardDefinition parser`
  - Files: `crates/card-script/src/parser.rs`

- [x] 13. Lua 沙盒配置

  **What to do**:
  - 创建 `crates/card-script/src/sandbox.rs`
  - 实现 `create_sandboxed_lua() -> Result<Lua>`:
    - 使用 `Lua::new_with(StdLib::TABLE | StdLib::STRING | StdLib::MATH, LuaOptions::default())`
    - **不加载**: IO, OS, DEBUG, PACKAGE（禁止文件/进程/调试器访问）
    - 设置内存限制: `lua.set_memory_limit(16 * 1024 * 1024)` (16MB)
    - 设置指令计数 hook: `lua.set_hook(HookTriggers::every_nth_instruction(10_000))` — 防无限循环
    - 额外清理: 将 globals 中的 loadfile/dofile/require 设为 nil
  - 此模块是 crate 中**唯一允许 unsafe 的地方**（mlua 内部 FFI）

  **Must NOT do**:
  - 不使用 `lua.sandbox()` (那是 Luau 专属)
  - 不使用 `luau` feature

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 2)
  - **Blocks**: Task 35
  - **Blocked By**: Task 11

  **References**:
  - Metis 审查报告 — mlua sandbox 技术修正
  - mlua docs: `Lua::new_with`, `StdLib`, `set_memory_limit`, `set_hook`

  **Acceptance Criteria**:
  - [ ] 沙盒 Lua 不能调用 `os.execute`, `io.open`, `require`
  - [ ] 无限循环脚本被 hook 中断

  **QA Scenarios**:
  ```
  Scenario: 沙盒安全性
    Tool: Bash
    Steps:
      1. 创建 Lua 脚本尝试 os.execute("ls") — 应报错 nil value
      2. 创建 Lua 脚本 while true do end — 应超时中断
      3. 创建 Lua 脚本分配巨大字符串 — 应内存限制报错
      4. cargo test -p card-script
    Expected Result: 所有危险操作被沙盒阻止
    Evidence: .sisyphus/evidence/task-13-sandbox.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(script): implement Lua sandbox with StdLib whitelist and resource limits`
  - Files: `crates/card-script/src/sandbox.rs`

- [x] 14. TCP 编解码 TcpConnection

  **What to do**:
  - 创建 `crates/card-protocol/src/codec.rs`
  - `TcpConnection` 结构体 — 封装 tokio TcpStream 的读写半
  - 帧格式: `| 4 bytes: length (big-endian u32) | N bytes: bincode payload |`
  - `TcpConnection::send(&mut self, msg: &NetworkMessage) -> Result<()>` — bincode 序列化 + 长度前缀发送
  - `TcpConnection::recv(&mut self) -> Result<NetworkMessage>` — 读长度 + 读 payload + bincode 反序列化
  - `TcpConnection::from_stream(stream: TcpStream) -> Self` — 从 tokio TcpStream 构建
  - 最大消息大小限制（如 1MB）防止恶意大消息
  - 支持 Ping/Pong 心跳

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 2)
  - **Blocks**: Tasks 23, 24
  - **Blocked By**: Task 6

  **References**:
  - `.sisyphus/drafts/game-engine-design.md` — TcpConnection 设计
  - tokio docs: TcpStream, AsyncReadExt, AsyncWriteExt

  **Acceptance Criteria**:
  - [ ] 能在 localhost 上发送和接收 NetworkMessage
  - [ ] 超大消息被拒绝

  **QA Scenarios**:
  ```
  Scenario: 本地回环通信
    Tool: Bash
    Steps:
      1. 启动 tokio TcpListener on 127.0.0.1:0（随机端口）
      2. 客户端连接, 发送 NetworkMessage::Hello
      3. 服务端接收, 验证消息正确
      4. 服务端回复 NetworkMessage::HelloAck
      5. 客户端接收并验证
      6. cargo test -p card-protocol
    Expected Result: 双向通信正确
    Evidence: .sisyphus/evidence/task-14-tcp-codec.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(protocol): implement length-prefixed TCP codec with bincode`
  - Files: `crates/card-protocol/src/codec.rs`

- [x] 15. ClientApi trait + VisibleGameState

  **What to do**:
  - 创建 `crates/card-client/src/api.rs`
  - 定义 `#[async_trait] pub trait ClientApi: Send + Sync`:
    - `async fn on_event(&self, event: &GameEvent, state: &VisibleGameState)`
    - `async fn choose_action(&self, available: &[AvailableAction], timeout: Duration) -> Option<Command>`
    - `async fn choose_targets(&self, options: &[TargetOption], count: usize, timeout: Duration) -> Option<Vec<InstanceId>>`
    - `async fn choose_cards(&self, options: &[CardOption], count: usize, timeout: Duration) -> Option<Vec<InstanceId>>`
  - `VisibleGameState` 结构体 — 客户端可见的游戏状态:
    - my_state: PlayerState（完整自己的状态）
    - opponent_public: OpponentView（对手公开信息）
    - phase, turn_number, chain 信息
  - `OpponentView` 结构体 — hand_count, deck_count, front, back, cost_zone, grave, hp, real_point
  - `CardPublicInfo` 结构体 — 公开卡片信息（id, name, card_type, attack 等）
  - `AvailableAction` 枚举 — 向客户端展示可选操作（PlayCard, ActivateEffect, DeclareAttack, Pass 等，含每个操作的有效参数范围）
  - `TargetOption` / `CardOption` 结构体 — 选择提示
  - `GameState::to_visible(&self, for_player: PlayerId) -> VisibleGameState` — 状态过滤（隐藏对手手牌/卡组）
  - 实现 `TestClient`（用于测试）— 接受预编程的 Command 队列，按顺序返回

  **Must NOT do**:
  - trait 中不包含任何 UI/渲染概念
  - 不包含 TUI 或终端相关类型

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 2)
  - **Blocks**: Tasks 16, 18, 22, 23, 25, 27, 29
  - **Blocked By**: Tasks 4, 6, 9

  **References**:
  - `.sisyphus/drafts/game-engine-design.md` — ClientApi 和 VisibleGameState 设计
  - `plan/ReadMe.md:118` — 客户端抽象要求

  **Acceptance Criteria**:
  - [ ] `cargo build -p card-client` 成功
  - [ ] TestClient 能按预编程返回命令
  - [ ] VisibleGameState 不暴露对手手牌内容

  **QA Scenarios**:
  ```
  Scenario: 状态可见性过滤
    Tool: Bash
    Steps:
      1. 创建 GameState, P2 手牌含3张卡
      2. to_visible(Player1) — 验证 opponent_public.hand_count == 3
      3. 验证 opponent_public 中不包含手牌的具体 CardId
      4. cargo test -p card-client
    Expected Result: 对手手牌被正确隐藏
    Evidence: .sisyphus/evidence/task-15-client-api.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(client): define ClientApi trait, VisibleGameState, and TestClient`
  - Files: `crates/card-client/src/api.rs`

- [x] 16. 回合状态机 Phase Runner

  **What to do**:
  - 创建 `crates/card-core/src/engine/phase.rs`
  - 实现 `PhaseRunner` 结构体，管理回合内7个阶段的状态转换
  - **TurnStart 阶段**: 触发回合开始效果，增加回合计数
  - **Draw 阶段**: 从卡组顶部抽1张（先手第一回合跳过，通过 `rules.first_player_draws` 和 turn_number 判断）。卡组为空时触发 GameOver(DeckOut)
  - **Recovery 阶段**: 
    - 计算回收数 X = 对手前场攻击力最高的卡的费用（对手前场无卡则 X=0）
    - 调用 ClientApi::choose_cards 让玩家从费用区选择最多 X 张卡回手
  - **Main1 阶段**: 循环调用 ClientApi::choose_action 展示可选操作
    - 计算 AvailableAction 列表：可出的手牌（费用够）、可激活的效果、Pass
    - 玩家选 Pass 则结束阶段
    - 选 PlayCard → 执行出卡流程（费用支付 → 入场/效果 → 触发检查）
    - 选 ActivateEffect → 进入连锁流程
  - **Battle 阶段**:
    - 遍历前场每个可攻击的人物卡
    - 每个攻击：选择攻击目标（对手前场卡 or 直接攻击前提无前场卡）
    - 直接攻击扣 HP，HP ≤ 0 触发 GameOver(HpZero)
    - 攻击对手前场卡：比较攻击力，高者胜低者进墓地，平局双方墓地
    - 战斗胜利获得 RealPoint（1点），溢出(>max)时发出 RealPointOverflow 事件
  - **Main2 阶段**: 同 Main1
  - **TurnEnd 阶段**: 
    - 清理"直到回合结束"的修饰器
    - 重置所有卡的 attacked_this_turn
    - 切换回合玩家
  - 每个阶段转换发出 `GameEvent::PhaseChanged`
  - 每个阶段开始/结束时检查触发效果（调用 TriggerChecker）

  **Must NOT do**:
  - 不实现连锁解算逻辑（那是 Task 18）
  - 不实现效果执行逻辑（那是 Task 19）
  - 只调用它们的接口

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 3, 部分并行)
  - **Parallel Group**: Wave 3 (with Tasks 17, 19, 20)
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 9, 15

  **References**:
  - `plan/ReadMe.md:50-85` — 7 个阶段详细规则
  - `plan/ReadMe.md:62-74` — 战斗流程
  - `.sisyphus/drafts/game-engine-design.md` — 阶段设计和战斗结算规则
  - `crates/card-core/src/state/` (Task 9) — GameState, Phase 枚举
  - `crates/card-client/src/api.rs` (Task 15) — ClientApi trait

  **Acceptance Criteria**:
  - [ ] TurnStart → Draw → Recovery → Main1 → Battle → Main2 → TurnEnd 正确循环
  - [ ] 先手第一回合 Draw 阶段不抽卡
  - [ ] Recovery 正确计算回收数
  - [ ] Battle 正确结算攻击力比较
  - [ ] 每阶段发出 PhaseChanged 事件

  **QA Scenarios**:
  ```
  Scenario: 完整回合流程
    Tool: Bash
    Steps:
      1. 构造 GameState: P1 有卡组10张, 前场1个攻击力500的人物卡; P2 前场1个攻击力300的人物卡
      2. 用 TestClient 预编程: Recovery选0张, Main1选Pass, Battle攻击P2前场卡, Main2选Pass
      3. 运行一个完整回合
      4. 验证 P2 前场卡进墓地（300 < 500）
      5. 验证 P1 获得 1 RealPoint
      6. 验证回合玩家切换
      7. cargo test -p card-core
    Expected Result: 回合按阶段顺序执行，战斗结算正确
    Evidence: .sisyphus/evidence/task-16-phase-runner.txt

  Scenario: 先手第一回合不抽卡
    Tool: Bash
    Steps:
      1. 构造初始 GameState, turn_number=1, first_player_draws=false
      2. 执行 Draw 阶段
      3. 验证手牌数不变（未抽卡）
      4. turn_number=2 再执行 Draw — 验证手牌+1
    Expected Result: 先手第一回合跳过抽卡
    Evidence: .sisyphus/evidence/task-16-first-turn-no-draw.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `feat(core): implement turn phase runner with 7-phase state machine`
  - Files: `crates/card-core/src/engine/phase.rs`

- [x] 17. TriggerChecker 效果触发扫描

  **What to do**:
  - 创建 `crates/card-core/src/engine/trigger.rs`
  - `TriggerChecker` 结构体（无状态，纯函数风格）
  - `check_triggers(state: &GameState, event: &GameEvent, perspective: PlayerId) -> Vec<TriggeredEffect>`
    - 扫描 perspective 玩家所有场上卡的所有效果
    - 检查每个效果的 trigger 是否匹配当前 event
    - 检查 activation_limit（OncePerTurn 需要记录已激活次数，存储在 GameState 中）
    - 对匹配的效果，评估 conditions（调用 Task 10 的 evaluate_condition）
    - 条件满足的效果加入返回列表
  - `TriggeredEffect` 结构体 — source_instance(InstanceId), effect_name(String), effect(Effect ref), optional(bool)
  - **事件触发匹配**:
    - `Trigger::OnSummon` 匹配 `GameEvent::CardSummoned`（当 source 是被召唤的卡）
    - `Trigger::OnDestroy` 匹配 `GameEvent::CardDestroyed`
    - `Trigger::OnExpose` 匹配 `GameEvent::CardMoved` 到费用区
    - `Trigger::OnAttack` 匹配 `GameEvent::AttackDeclared`
    - `Trigger::OnEvent(EventTrigger::RealPointChanged{..})` 匹配 `GameEvent::RealPointChanged`
    - 等等，覆盖所有 Trigger 变体
  - **可选效果处理**:
    - optional=true: 询问玩家是否激活
    - optional=false: 强制激活

  **Must NOT do**:
  - 不执行效果（只扫描和匹配）
  - 不修改 GameState

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 3)
  - **Parallel Group**: Wave 3 (with Tasks 16, 19, 20)
  - **Blocks**: Tasks 18, 19, 21
  - **Blocked By**: Tasks 5, 9, 10

  **References**:
  - `plan/ReadMe.md:95-103` — 效果触发规则
  - `plan/ReadMe.md:132-184` — 7 个效果示例（分析每个的触发条件）
  - `crates/card-core/src/effect/` (Task 5) — Trigger, EventTrigger 枚举
  - `crates/card-core/src/effect/evaluator.rs` (Task 10) — evaluate_condition

  **Acceptance Criteria**:
  - [ ] OnSummon 触发正确匹配 CardSummoned 事件
  - [ ] EventTrigger::RealPointOverflow 正确匹配
  - [ ] OncePerTurn 限制正确执行
  - [ ] 条件不满足时效果不触发

  **QA Scenarios**:
  ```
  Scenario: 召唤触发效果匹配
    Tool: Bash
    Steps:
      1. 构造 GameState: P1 前场有卡A，效果"on_summon: draw 1"
      2. 发出 GameEvent::CardSummoned { instance: A }
      3. check_triggers 返回包含卡A的触发效果
      4. cargo test -p card-core
    Expected Result: 触发正确匹配
    Evidence: .sisyphus/evidence/task-17-trigger-check.txt

  Scenario: 条件不满足不触发
    Tool: Bash
    Steps:
      1. 构造 GameState: P1 前场卡效果带条件 "hand_count > 5", 当前手牌3张
      2. 发出匹配的事件
      3. check_triggers 返回空列表
    Expected Result: 条件阻止触发
    Evidence: .sisyphus/evidence/task-17-condition-block.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `feat(core): implement trigger checker for effect activation scanning`
  - Files: `crates/card-core/src/engine/trigger.rs`

- [x] 18. ChainManager FILO 连锁解算

  **What to do**:
  - 创建 `crates/card-core/src/engine/chain.rs`
  - `ChainManager` 结构体
  - **连锁构建阶段**:
    - `start_chain(initial_effect: TriggeredEffect)` — 初始化连锁栈，压入第一个效果
    - 循环给双方玩家连锁机会（通过 ClientApi::choose_action，提供 ChainActivate/ChainPass）
    - 瞬时策略卡：打出后立即结算（不入栈），然后将连锁权交给对手
    - 双方都 Pass 时连锁构建结束
  - **连锁解算阶段**（FILO）:
    - 从栈顶开始逐个执行效果（调用 ActionExecutor）
    - 每个效果执行后，检查是否有新的被动触发（调用 TriggerChecker）
    - **被动效果立即插入当前连锁栈顶**（用户确认的规则）
    - 继续从新栈顶解算，直到栈空
  - **处理无效效果**:
    - 如果目标在解算前已不存在（如已被破坏），效果失效（跳过，不报错）
    - 连锁中 source 卡被破坏，效果仍然执行（效果已入栈）
  - 发出连锁相关事件: ChainStarted, ChainLink, ChainResolving, ChainComplete

  **Must NOT do**:
  - 不直接修改 GameState（通过 ActionExecutor 的返回值/events 进行）
  - 不实现具体 Action 执行（那是 Task 19）

  **Recommended Agent Profile**:
  - **Category**: `ultrabrain`
  - **Skills**: []
  - Reason: FILO 连锁解算含递归触发、栈操作、边界情况，逻辑密度极高

  **Parallelization**:
  - **Can Run In Parallel**: NO (依赖 T17 完成)
  - **Parallel Group**: Wave 3 (starts after T17)
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 5, 9, 15, 17

  **References**:
  - `plan/ReadMe.md:95-103` — 连锁规则
  - `plan/ReadMe.md:108-116` — 瞬时策略卡处理
  - `.sisyphus/drafts/game-engine-design.md` — "连锁中触发：被动效果立即插入当前连锁栈顶"
  - `crates/card-core/src/engine/trigger.rs` (Task 17) — TriggerChecker
  - `crates/card-client/src/api.rs` (Task 15) — ClientApi::choose_action

  **Acceptance Criteria**:
  - [ ] 基本连锁：A触发→B连锁→B先解→A后解（FILO）
  - [ ] 被动触发正确插入栈顶
  - [ ] 瞬时策略卡立即结算不入栈
  - [ ] 目标消失时效果正确失效
  - [ ] 双方 Pass 正确结束连锁

  **QA Scenarios**:
  ```
  Scenario: FILO 连锁解算顺序
    Tool: Bash
    Steps:
      1. 构造连锁: 效果A(draw 1) 入栈, 效果B(damage 1) 入栈
      2. 解算连锁
      3. 验证 B 先执行（damage），A 后执行（draw）
      4. cargo test -p card-core
    Expected Result: 后入栈的先执行
    Evidence: .sisyphus/evidence/task-18-filo-order.txt

  Scenario: 连锁中被动触发插入
    Tool: Bash
    Steps:
      1. 构造: 栈中有效果A, 效果A执行触发卡X的被动效果
      2. 验证卡X的效果被插入栈顶并立即解算
    Expected Result: 被动触发正确处理
    Evidence: .sisyphus/evidence/task-18-passive-insert.txt

  Scenario: 目标消失时效果失效
    Tool: Bash
    Steps:
      1. 构造: 效果A目标为卡X, 效果B destroy 卡X
      2. B 在栈顶先执行 → 卡X 被破坏
      3. A 执行时目标不存在 → 效果跳过
    Expected Result: 无 panic，效果安全跳过
    Evidence: .sisyphus/evidence/task-18-target-gone.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `feat(core): implement FILO chain resolution with passive trigger insertion`
  - Files: `crates/card-core/src/engine/chain.rs`

- [x] 19. ActionExecutor 动作执行 + 免疫检查

  **What to do**:
  - 创建 `crates/card-core/src/engine/action.rs`
  - `ActionExecutor` 结构体
  - `execute_action(action: &Action, state: &mut GameState, source: InstanceId, perspective: PlayerId, targets: &[InstanceId]) -> Vec<GameEvent>`
    - 每个 Action 变体的执行实现:
    - `Draw { count, player }` → 从卡组移到手牌，卡组空则 DeckOut
    - `Damage { amount, target }` → 扣 HP
    - `Destroy { target }` → 从场上移到墓地
    - `SummonFromZone { card, from_zone, to_zone }` → 区域间移动 + 触发 CardSummoned
    - `ReturnToHand { target }` → 返回手牌
    - `SendToGrave { target }` → 送入墓地
    - `ModifyAttack { target, delta }` → 修改攻击力
    - `GainRealPoint { player, amount }` → 增加 RP，溢出发 RealPointOverflow
    - `HealHp { player, amount }` → 恢复 HP（不超过 max_hp）
    - `Discard { player, count }` → 弃牌（调用 choose_cards）
    - `ApplyModifier { target, modifier, duration }` → 给卡添加修饰器
    - `RemoveModifier { target, modifier_type }` → 移除修饰器
    - `Special { key }` → 查找并调用 SpecialEffectHandler（HashMap<EffectKey, Box<dyn SpecialEffectHandler>>）
  - **免疫检查**（在执行前检查）:
    - 遍历目标卡的 modifiers，检查是否有免疫本次效果的修饰器
    - `ImmuneToCardType(CardType)` → 如果效果来源卡的类型匹配，跳过
    - `ImmuneToProperty(Property)` → 如果来源卡属性匹配，跳过
    - `ImmuneToDestruction` → Destroy 动作无效
    - `ImmuneToTargeting` → 不能被选为目标
    - 免疫时效果静默跳过（不报错，发 log）
  - **费用支付流程**:
    - `pay_cost(state: &mut GameState, player: PlayerId, cost: &CostRequirement, client: &dyn ClientApi) -> Result<Vec<GameEvent>>`
    - 手卡→费用区：触发 OnExpose
    - RealPoint 消耗

  **Must NOT do**:
  - 不处理连锁逻辑（只执行单个 Action）
  - 不决定执行顺序

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 3, 与 T16/T20 并行)
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 5, 9, 17

  **References**:
  - `plan/ReadMe.md:55-60` — 费用支付系统
  - `plan/ReadMe.md:87-94` — 数值上下限
  - `plan/ReadMe.md:132-184` — 效果示例（验证所有动作类型）
  - `.sisyphus/drafts/game-engine-design.md` — 修饰器和免疫设计
  - `crates/card-core/src/effect/` (Task 5) — Action 枚举, Modifier 枚举

  **Acceptance Criteria**:
  - [ ] 所有 Action 变体正确执行并返回相应 GameEvent
  - [ ] 免疫检查正确阻止被免疫的效果
  - [ ] RealPoint 溢出正确发出 Overflow 事件
  - [ ] 费用支付触发 OnExpose
  - [ ] DeckOut 在卡组空时正确触发

  **QA Scenarios**:
  ```
  Scenario: 基本动作执行
    Tool: Bash
    Steps:
      1. 构造 GameState: P1 HP=5, 有卡组
      2. execute_action(Draw{count:1, player:Self_}) → 验证手牌+1
      3. execute_action(Damage{amount:2, target:P1}) → 验证 HP=3
      4. execute_action(GainRealPoint{player:Self_, amount:7}) → 验证 RP=6 + RealPointOverflow 事件
      5. cargo test -p card-core
    Expected Result: 所有动作正确执行
    Evidence: .sisyphus/evidence/task-19-actions.txt

  Scenario: 免疫效果阻止破坏
    Tool: Bash
    Steps:
      1. 构造: 卡A 有 ImmuneToDestruction 修饰器
      2. execute_action(Destroy{target:A})
      3. 验证卡A仍在场上
      4. 验证返回的 events 中无 CardDestroyed
    Expected Result: 破坏被免疫
    Evidence: .sisyphus/evidence/task-19-immune.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `feat(core): implement action executor with immunity checks`
  - Files: `crates/card-core/src/engine/action.rs`

- [x] 20. Modifier 系统（应用/移除/持续时间管理）

  **What to do**:
  - 创建 `crates/card-core/src/engine/modifier.rs`
  - `ModifierManager` 结构体（无状态，纯函数）
  - `apply_modifier(card: &mut CardInstance, modifier: Modifier, source: InstanceId, duration: ModifierDuration)`:
    - 创建 AppliedModifier，添加到 card.modifiers
    - 如果是 AttackBoost(delta)，立即修改 card.current_attack
  - `remove_modifier(card: &mut CardInstance, index: usize)`:
    - 移除指定修饰器
    - 如果是 AttackBoost，回退攻击力修改
  - `cleanup_expired(state: &mut GameState) -> Vec<GameEvent>`:
    - **UntilEndOfTurn**: 在 TurnEnd 时清理
    - **TurnCount(n)**: 每回合递减，归零时清理
    - **WhileSourceOnField**: 检查 source 是否仍在场上，不在则清理
    - **Permanent**: 永不清理
    - 返回清理产生的事件（如攻击力回退）
  - `has_immunity(card: &CardInstance, check: ImmunityCheck) -> bool`:
    - 检查卡上所有修饰器是否有匹配的免疫
  - `ImmunityCheck` 枚举 — ByCardType(CardType), ByProperty(Property), Destruction, Targeting
  - `get_effective_attack(card: &CardInstance) -> u16`:
    - 基础攻击力 + 所有 AttackBoost 修饰器的累计效果
    - 最低为 0（不为负）

  **Must NOT do**:
  - 不处理修饰器的触发逻辑（那是 TriggerChecker 的职责）

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 3)
  - **Parallel Group**: Wave 3 (with Tasks 16, 17, 19)
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 4, 9

  **References**:
  - `.sisyphus/drafts/game-engine-design.md` — Modifier/AppliedModifier/ModifierDuration 设计
  - `crates/card-core/src/effect/` (Task 5) — Modifier 枚举, AppliedModifier 结构体
  - `crates/card-core/src/state/` (Task 9) — CardInstance.modifiers 字段

  **Acceptance Criteria**:
  - [ ] AttackBoost 正确修改攻击力并在移除时回退
  - [ ] UntilEndOfTurn 修饰器在回合结束时清理
  - [ ] WhileSourceOnField 在 source 离场时清理
  - [ ] 免疫检查正确工作

  **QA Scenarios**:
  ```
  Scenario: 攻击力修饰器生命周期
    Tool: Bash
    Steps:
      1. 卡A 基础攻击 500
      2. apply_modifier(AttackBoost(200), UntilEndOfTurn) → 攻击力 700
      3. cleanup_expired (回合结束) → 攻击力回退到 500
      4. cargo test -p card-core
    Expected Result: 修饰器正确应用和回退
    Evidence: .sisyphus/evidence/task-20-modifier-lifecycle.txt

  Scenario: WhileSourceOnField 清理
    Tool: Bash
    Steps:
      1. 卡B 给卡A 一个 Permanent 效果，duration=WhileSourceOnField(B)
      2. 将卡B 移入墓地
      3. cleanup_expired → 卡A 的修饰器被移除
    Expected Result: source 离场时修饰器清理
    Evidence: .sisyphus/evidence/task-20-source-leave.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `feat(core): implement modifier system with duration management`
  - Files: `crates/card-core/src/engine/modifier.rs`

- [ ] 21. GameEngine 主循环集成

  **What to do**:
  - 创建 `crates/card-core/src/engine/mod.rs` 和 `crates/card-core/src/engine/game_engine.rs`
  - `GameEngine` 结构体 — 持有 GameState, PhaseRunner, TriggerChecker, ChainManager, ActionExecutor, ModifierManager
  - `GameEngine::new(state: GameState, clients: [Arc<dyn ClientApi>; 2]) -> Self`
  - `GameEngine::run(&mut self) -> GameResult`:
    - 游戏主循环: 反复执行回合直到 GameOver
    - 每回合: 调用 PhaseRunner 依次执行 7 个阶段
    - 每个阶段/动作执行后检查触发效果 → 进入连锁
    - 连锁解算中调用 ActionExecutor
    - 回合结束时调用 ModifierManager::cleanup_expired
  - `GameResult` 结构体 — winner(Option<PlayerId>), reason(GameOverReason), final_state(GameState), event_log(Vec<GameEvent>)
  - **出牌流程集成**:
    1. 玩家选择出牌
    2. 检查手牌合法性
    3. 费用支付（手卡→费用区，触发 OnExpose）
    4. 卡片入场（触发 OnSummon）
    5. TriggerChecker 扫描触发
    6. 如有触发 → ChainManager 构建连锁 → FILO 解算
  - **事件日志**: 所有 GameEvent 按顺序记录到 event_log（回放用）
  - **超时处理**: 每次 ClientApi 调用传入 timeout，超时返回 None → GameOver(Timeout)
  - **确定性 RNG**: 使用 rand_chacha::ChaCha8Rng + seed，保证相同 seed 产生相同游戏

  **Must NOT do**:
  - 不包含网络逻辑
  - 不包含 UI 渲染
  - 纯游戏逻辑引擎

  **Recommended Agent Profile**:
  - **Category**: `ultrabrain`
  - **Skills**: []
  - Reason: 集成所有核心系统，游戏循环逻辑最复杂

  **Parallelization**:
  - **Can Run In Parallel**: NO (依赖 Wave 3 所有其他任务)
  - **Parallel Group**: Wave 3 (最后完成)
  - **Blocks**: Tasks 22, 25, 26, 27, 34, 36
  - **Blocked By**: Tasks 16, 17, 18, 19, 20

  **References**:
  - `plan/ReadMe.md:50-116` — 完整游戏流程
  - `.sisyphus/drafts/game-engine-design.md` — GameEngine 设计
  - `crates/card-core/src/engine/phase.rs` (Task 16) — PhaseRunner
  - `crates/card-core/src/engine/trigger.rs` (Task 17) — TriggerChecker
  - `crates/card-core/src/engine/chain.rs` (Task 18) — ChainManager
  - `crates/card-core/src/engine/action.rs` (Task 19) — ActionExecutor
  - `crates/card-core/src/engine/modifier.rs` (Task 20) — ModifierManager
  - `crates/card-client/src/api.rs` (Task 15) — ClientApi, TestClient

  **Acceptance Criteria**:
  - [ ] 两个 TestClient 能自动完成一局游戏（一方 HP 归零或卡组耗尽）
  - [ ] event_log 包含完整游戏过程
  - [ ] 相同 seed + 相同操作 = 相同结果（确定性）
  - [ ] 超时正确触发 GameOver

  **QA Scenarios**:
  ```
  Scenario: TestClient 自动对弈
    Tool: Bash
    Steps:
      1. 创建 CardRegistry 包含基础测试卡（攻击力不同的人物卡）
      2. 两个 TestClient 预编程: 交替出牌、攻击、Pass
      3. GameEngine::run() 执行完整游戏
      4. 验证返回 GameResult，winner 不为 None
      5. 验证 event_log 长度 > 0
      6. cargo test -p card-core
    Expected Result: 游戏正常完成并返回结果
    Evidence: .sisyphus/evidence/task-21-auto-game.txt

  Scenario: 确定性验证
    Tool: Bash
    Steps:
      1. 用 seed=42 运行一局游戏
      2. 用 seed=42 + 相同操作重新运行
      3. assert_eq!(result1.final_state, result2.final_state)
      4. assert_eq!(result1.event_log, result2.event_log)
    Expected Result: 两局结果完全一致
    Evidence: .sisyphus/evidence/task-21-deterministic.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `feat(core): integrate game engine main loop with all subsystems`
  - Files: `crates/card-core/src/engine/mod.rs`, `crates/card-core/src/engine/game_engine.rs`

- [ ] 22. GameSession 会话管理

  **What to do**:
  - 创建 `crates/card-server/src/session.rs`
  - `GameSession` 结构体 — 管理一局游戏的完整生命周期
  - **会话阶段**:
    1. `WaitingForPlayers` — 等待两个玩家连接
    2. `DeckSubmission` — 等待双方提交卡组（validate_deck 校验）
    3. `LoadingScripts` — 调用 ScriptLoader::load_for_game 加载 Lua
    4. `Playing` — GameEngine::run() 执行游戏
    5. `Finished` — 游戏结束，记录结果
  - `GameSession::new(rules: GameRules, script_index: Arc<ScriptIndex>) -> Self`
  - `GameSession::add_player(client: Arc<dyn ClientApi>, deck: Vec<CardId>) -> Result<PlayerId>`
  - `GameSession::start(&mut self) -> Result<GameResult>`:
    - 校验双方卡组
    - 加载脚本构建 CardRegistry
    - 构建初始 GameState（洗牌用确定性 RNG）
    - 创建 GameEngine 并运行
    - 返回 GameResult
  - **回放集成**: 传入 ReplayRecorder（Option），记录 event_log 和 commands
  - **存档集成**: 提供 save_snapshot() 和 load_snapshot() 入口
  - 正确的错误处理: 脚本加载失败、卡组非法、玩家断连

  **Must NOT do**:
  - 不包含 TCP 网络逻辑（那是 Task 23/24）
  - 不包含 UI 逻辑

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 4)
  - **Parallel Group**: Wave 4 (with Tasks 23-28)
  - **Blocks**: Tasks 29, 37
  - **Blocked By**: Tasks 21, 15

  **References**:
  - `.sisyphus/drafts/game-engine-design.md` — GameSession 设计
  - `crates/card-core/src/engine/game_engine.rs` (Task 21) — GameEngine
  - `crates/card-script/src/loader.rs` (Task 11) — ScriptLoader
  - `crates/card-core/src/rules/` (Task 7) — validate_deck

  **Acceptance Criteria**:
  - [ ] GameSession 可从创建到结束完成完整流程
  - [ ] 非法卡组被正确拒绝
  - [ ] 脚本加载失败返回错误而非 panic

  **QA Scenarios**:
  ```
  Scenario: 完整会话流程
    Tool: Bash
    Steps:
      1. 创建 GameSession(default rules)
      2. 添加两个 TestClient + 合法卡组
      3. session.start() 运行游戏
      4. 验证返回 GameResult
      5. cargo test -p card-server
    Expected Result: 会话正常完成
    Evidence: .sisyphus/evidence/task-22-session.txt

  Scenario: 非法卡组拒绝
    Tool: Bash
    Steps:
      1. 提交只有 10 张卡的卡组
      2. session.add_player() 返回 Err
      3. 错误信息包含 "deck size"
    Expected Result: 卡组校验失败被捕获
    Evidence: .sisyphus/evidence/task-22-invalid-deck.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `feat(server): implement game session lifecycle management`
  - Files: `crates/card-server/src/session.rs`

- [ ] 23. RemoteClient TCP 代理

  **What to do**:
  - 创建 `crates/card-server/src/remote_client.rs`
  - `RemoteClient` 结构体 — 实现 `ClientApi` trait，通过 TCP 代理到远程玩家
  - 持有 `TcpConnection`（Task 14）
  - **ClientApi 实现**:
    - `on_event` → 发送 `NetworkMessage::EventNotification`
    - `choose_action` → 发送 `NetworkMessage::RequestAction { available, timeout }` → 等待 `NetworkMessage::CommandResponse`
    - `choose_targets` → 发送 `NetworkMessage::RequestTargetSelection` → 等待 `NetworkMessage::TargetResponse`
    - `choose_cards` → 发送 `NetworkMessage::RequestCardSelection` → 等待 `NetworkMessage::CardResponse`
  - **超时处理**: 使用 `tokio::time::timeout` 包裹 recv，超时返回 None
  - **断连处理**: recv 返回 Error 时，标记为已断连，后续调用直接返回 None（触发 GameOver(Timeout)）
  - **Ping/Pong**: 后台 task 定期发送 Ping，检测连接活性

  **Must NOT do**:
  - 不处理连接建立（那是 Task 24）
  - 不包含重连逻辑（第一版不做）

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 4)
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 24, 37
  - **Blocked By**: Tasks 14, 15

  **References**:
  - `.sisyphus/drafts/game-engine-design.md` — RemoteClient 设计
  - `crates/card-protocol/src/codec.rs` (Task 14) — TcpConnection
  - `crates/card-client/src/api.rs` (Task 15) — ClientApi trait
  - `crates/card-protocol/src/message.rs` (Task 6) — NetworkMessage

  **Acceptance Criteria**:
  - [ ] RemoteClient 正确实现 ClientApi trait
  - [ ] 超时返回 None
  - [ ] 断连后不阻塞

  **QA Scenarios**:
  ```
  Scenario: 远程动作请求/响应
    Tool: Bash
    Steps:
      1. 启动 TCP listener，accept 一个连接
      2. 创建 RemoteClient(connection)
      3. 在另一端发送 CommandResponse(PlayCard{...})
      4. RemoteClient::choose_action 返回正确的 Command
      5. cargo test -p card-server
    Expected Result: TCP 代理正确传递命令
    Evidence: .sisyphus/evidence/task-23-remote-client.txt

  Scenario: 超时处理
    Tool: Bash
    Steps:
      1. 创建 RemoteClient，对端不发送任何响应
      2. choose_action(timeout=100ms) → 应返回 None
    Expected Result: 超时后正确返回 None
    Evidence: .sisyphus/evidence/task-23-timeout.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `feat(server): implement RemoteClient TCP proxy for ClientApi`
  - Files: `crates/card-server/src/remote_client.rs`

- [ ] 24. TCP 服务端 listen + accept

  **What to do**:
  - 创建 `crates/card-server/src/network.rs`
  - `GameServer` 结构体:
    - `bind_addr: SocketAddr`
    - `script_index: Arc<ScriptIndex>`
    - `rules: GameRules`
  - `GameServer::start(&self) -> Result<()>`:
    - 绑定 TCP listener
    - 接受第一个连接 → 发送 Hello → 接收 HelloAck（版本号检查）
    - 接受第二个连接 → 同上
    - 接收双方 DeckSubmit
    - 创建两个 RemoteClient
    - 创建 GameSession，添加两个 RemoteClient
    - GameSession::start() 运行游戏
    - 游戏结束后关闭连接
  - `GameServer::start_local(client1: Arc<dyn ClientApi>, client2_remote: bool)`:
    - 混合模式: 本地玩家 + 远程玩家
  - **Hello 握手协议**:
    - 服务端: Hello { version, rules }
    - 客户端: HelloAck { version } — 版本不匹配则断开
  - 使用 tracing 记录所有网络事件

  **Must NOT do**:
  - 不实现匹配逻辑（那是 matchmaker）
  - 不实现重连

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 4, after T23)
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 37
  - **Blocked By**: Tasks 14, 23

  **References**:
  - `.sisyphus/drafts/game-engine-design.md` — 网络架构设计
  - `crates/card-protocol/src/codec.rs` (Task 14) — TcpConnection
  - `crates/card-server/src/remote_client.rs` (Task 23) — RemoteClient
  - `crates/card-server/src/session.rs` (Task 22) — GameSession

  **Acceptance Criteria**:
  - [ ] 两个 TCP 客户端能连接并完成握手
  - [ ] 版本不匹配时正确断开
  - [ ] 双方提交卡组后游戏正常开始

  **QA Scenarios**:
  ```
  Scenario: 双人TCP连接
    Tool: Bash
    Steps:
      1. 启动 GameServer on 127.0.0.1:0
      2. 两个 TCP 客户端连接
      3. 完成 Hello/HelloAck 握手
      4. 双方提交卡组
      5. 验证游戏开始（接收到 GameStart 消息）
      6. cargo test -p card-server
    Expected Result: 握手和卡组提交成功
    Evidence: .sisyphus/evidence/task-24-tcp-server.txt

  Scenario: 版本不匹配
    Tool: Bash
    Steps:
      1. 客户端发送 HelloAck { version: "0.0.0" }
      2. 服务端检测版本不匹配
      3. 发送 Disconnect + 关闭连接
    Expected Result: 连接被正确拒绝
    Evidence: .sisyphus/evidence/task-24-version-mismatch.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `feat(server): implement TCP game server with handshake protocol`
  - Files: `crates/card-server/src/network.rs`

- [ ] 25. 回放系统 ReplayData + ReplayClient

  **What to do**:
  - 创建 `crates/card-server/src/replay.rs`
  - `ReplayData` 结构体 (Serialize, Deserialize):
    - version: String
    - timestamp: chrono::DateTime<Utc>
    - rules: GameRules
    - deck1: Vec<CardId>, deck2: Vec<CardId>
    - rng_seed: u64
    - commands: Vec<(PlayerId, Command)> — 按顺序记录所有玩家命令
    - events: Vec<GameEvent> — 完整事件日志
  - `ReplayRecorder` 结构体:
    - `record_command(player: PlayerId, command: &Command)`
    - `record_event(event: &GameEvent)`
    - `finalize() -> ReplayData`
  - `ReplayData::save_to_file(path: &Path) -> Result<()>` — serde_json 序列化
  - `ReplayData::load_from_file(path: &Path) -> Result<Self>` — 反序列化
  - `ReplayClient` 结构体 — 实现 ClientApi trait:
    - 持有 ReplayData.commands 的迭代器
    - `choose_action` → 返回下一个命令（不等待用户输入）
    - `on_event` → 可选：与 ReplayData.events 比对验证
  - **回放验证**: `verify_replay(replay: &ReplayData, registry: &CardRegistry) -> Result<bool>`:
    - 用 ReplayClient 重新运行游戏
    - 比较最终 GameState 是否一致

  **Must NOT do**:
  - 不实现回放速度控制（第一版按最快速度回放）
  - 不实现 UI 回放界面

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 4)
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 38
  - **Blocked By**: Tasks 15, 21

  **References**:
  - `.sisyphus/drafts/game-engine-design.md` — 回放系统设计（命令日志方式）
  - `plan/ReadMe.md:117` — 回放功能要求
  - `crates/card-client/src/api.rs` (Task 15) — ClientApi trait
  - `crates/card-core/src/engine/game_engine.rs` (Task 21) — GameEngine, GameResult

  **Acceptance Criteria**:
  - [ ] ReplayData 可以 JSON 序列化/反序列化
  - [ ] ReplayClient 正确实现 ClientApi
  - [ ] verify_replay 返回 true（相同结果）

  **QA Scenarios**:
  ```
  Scenario: 录制→保存→加载→回放
    Tool: Bash
    Steps:
      1. 运行一局游戏，用 ReplayRecorder 记录
      2. 保存到临时文件
      3. 加载 ReplayData
      4. 用 ReplayClient 重放
      5. assert_eq!(original_final_state, replay_final_state)
      6. cargo test -p card-server
    Expected Result: 回放重现相同游戏
    Evidence: .sisyphus/evidence/task-25-replay.txt

  Scenario: 回放文件可读
    Tool: Bash
    Steps:
      1. 保存 ReplayData 到 .json 文件
      2. cat 文件确认是有效 JSON
      3. 确认包含 commands 和 events 数组
    Expected Result: JSON 可读
    Evidence: .sisyphus/evidence/task-25-replay-json.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `feat(server): implement replay recording, saving, and playback`
  - Files: `crates/card-server/src/replay.rs`

- [ ] 26. 快照存档/恢复

  **What to do**:
  - 创建 `crates/card-server/src/snapshot.rs`
  - `GameSnapshot` 结构体 (Serialize, Deserialize):
    - version: String
    - timestamp: chrono::DateTime<Utc>
    - state: GameState（完整游戏状态）
    - rules: GameRules
    - card_ids: Vec<CardId>（需要加载的所有卡ID，用于恢复时重建 CardRegistry）
  - `save_snapshot(state: &GameState, rules: &GameRules) -> GameSnapshot`
  - `GameSnapshot::save_to_file(path: &Path) -> Result<()>` — serde_json
  - `GameSnapshot::load_from_file(path: &Path) -> Result<Self>`
  - `restore_game(snapshot: GameSnapshot, script_index: &ScriptIndex, clients: [Arc<dyn ClientApi>; 2]) -> Result<GameEngine>`:
    - 从 snapshot 加载 GameState
    - 重新加载 card_ids 涉及的 Lua 脚本构建 CardRegistry
    - 创建 GameEngine 并恢复到指定状态继续执行
  - **重要**: GameState 必须包含足够信息完全恢复（包括 chain 状态、修饰器、已激活记录等）

  **Must NOT do**:
  - 不实现 command-log 方式的存档（只用快照）
  - 不实现自动保存（手动触发）

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 4)
  - **Parallel Group**: Wave 4
  - **Blocks**: None
  - **Blocked By**: Tasks 9, 21

  **References**:
  - `.sisyphus/drafts/game-engine-design.md` — 快照存档设计
  - `plan/ReadMe.md:119` — 残局保存要求
  - `crates/card-core/src/state/` (Task 9) — GameState（需要完整序列化）

  **Acceptance Criteria**:
  - [ ] 保存→加载→恢复后游戏可继续执行
  - [ ] 快照文件是有效 JSON
  - [ ] 恢复后的 GameState 与保存时一致

  **QA Scenarios**:
  ```
  Scenario: 存档→恢复→继续
    Tool: Bash
    Steps:
      1. 运行游戏到第 3 回合
      2. save_snapshot 保存当前状态
      3. 从快照 restore_game 恢复
      4. 继续运行至游戏结束
      5. 验证游戏正常完成
      6. cargo test -p card-server
    Expected Result: 从存档恢复后游戏正常继续
    Evidence: .sisyphus/evidence/task-26-snapshot.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `feat(server): implement snapshot save/restore for game persistence`
  - Files: `crates/card-server/src/snapshot.rs`

- [ ] 27. AiClient 随机合法动作

  **What to do**:
  - 创建 `crates/card-server/src/ai_client.rs`
  - `AiClient` 结构体 — 实现 ClientApi trait
  - 持有 `ChaCha8Rng`（确定性随机）
  - **choose_action**: 从 available actions 中随机选一个
    - 偏好权重（可选但推荐）: 优先出牌 > 攻击 > 激活效果 > Pass
    - 但第一版简单随机即可
  - **choose_targets**: 从可选目标中随机选
  - **choose_cards**: 从可选卡中随机选
  - **on_event**: 空实现（AI 不需要看事件）
  - 所有选择使用确定性 RNG（相同 seed = 相同行为）

  **Must NOT do**:
  - 不实现战略逻辑（仅随机）
  - 不分析游戏状态做决策

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 4)
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 36
  - **Blocked By**: Tasks 15, 21

  **References**:
  - `crates/card-client/src/api.rs` (Task 15) — ClientApi trait
  - Metis 审查: "AI = 随机合法动作"

  **Acceptance Criteria**:
  - [ ] AiClient 正确实现 ClientApi 所有方法
  - [ ] 相同 seed = 相同选择序列
  - [ ] 永不返回 None（AI 不超时）

  **QA Scenarios**:
  ```
  Scenario: AI 对弈
    Tool: Bash
    Steps:
      1. 两个 AiClient（不同 seed）互相对弈
      2. GameEngine::run() 正常完成
      3. 验证返回 GameResult
      4. cargo test -p card-server
    Expected Result: AI 可以自动完成一局游戏
    Evidence: .sisyphus/evidence/task-27-ai.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `feat(server): implement AI client with random legal moves`
  - Files: `crates/card-server/src/ai_client.rs`

- [ ] 28. S000 测试卡包 Lua 脚本

  **What to do**:
  - 创建 `scripts/S000/` 目录
  - 编写 8+ 张测试卡的 Lua 脚本，覆盖所有卡类型和主要效果机制:
  - **S000-C-001.lua** — 基础人物卡（无效果，攻击力 500，费用 3，理性）
  - **S000-C-002.lua** — 人物卡 + 召唤触发（on_summon: draw 1）
  - **S000-C-003.lua** — 人物卡 + 事件触发（对手 RP 增加时抽 1）+ 条件（手牌<5）
  - **S000-S-001.lua** — 普通策略卡（destroy 对手前场1张卡，费用 2）
  - **S000-S-002.lua** — 诡计策略卡（对手弃 1 张手牌，费用 1）
  - **S000-S-003.lua** — 瞬时策略卡（instant type，给己方 1 卡 +200 攻击直到回合结束）
  - **S000-I-001.lua** — 普通物品卡（给装备的卡 +300 攻击，WhileSourceOnField）
  - **S000-I-002.lua** — 存留物品卡（每回合开始恢复 1 HP）
  - **S000-L-001.lua** — 传奇卡（强力效果：召唤时破坏对手所有前场卡，费用 6）
  - 每个脚本返回符合 parser (Task 12) 期望格式的 Lua table
  - 包含中文效果描述文本
  - 覆盖的机制：触发效果(on_summon/on_event)、条件表达式、修饰器(attack_boost/until_end_of_turn/while_source_on_field)、多种动作(draw/destroy/discard/modify_attack/heal_hp)

  **Must NOT do**:
  - 不在 Lua 中编写游戏逻辑
  - 不调用任何 Lua 标准库函数（纯数据返回）
  - 不使用 require/loadfile

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 4)
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 35, 36
  - **Blocked By**: Tasks 5 (Effect 类型定义), 12 (parser 格式)

  **References**:
  - `plan/ReadMe.md:132-184` — 7 个效果示例
  - `crates/card-core/src/effect/` (Task 5) — Effect 类型（Lua table 必须映射到这些类型）
  - `crates/card-script/src/parser.rs` (Task 12) — parser 期望的 table 格式
  - `plan/ReadMe.md:12-30` — 卡片属性和类型枚举值

  **Acceptance Criteria**:
  - [ ] 8+ 个 .lua 文件存在于 scripts/S000/
  - [ ] 所有文件能被 ScriptLoader 加载 + parser 解析无错
  - [ ] 覆盖所有 4 种卡类型
  - [ ] 覆盖主要效果机制（触发/条件/动作/修饰器）

  **QA Scenarios**:
  ```
  Scenario: 全卡包加载
    Tool: Bash
    Steps:
      1. ScriptIndex::scan("scripts/") 扫描
      2. 对每个 S000 卡调用 ScriptLoader 加载 + parser 解析
      3. 验证返回 8+ 个 CardDefinition
      4. 验证类型分布: 至少有 C/S/I/L 各1张
      5. cargo test -p card-script
    Expected Result: 所有测试卡正确加载
    Evidence: .sisyphus/evidence/task-28-test-cards.txt

  Scenario: 效果机制覆盖
    Tool: Bash
    Steps:
      1. 验证 S000-C-002 有 on_summon 触发
      2. 验证 S000-C-003 有 EventTrigger + Condition
      3. 验证 S000-I-001 有 WhileSourceOnField 修饰器
      4. 验证 S000-L-001 有 destroy 动作
    Expected Result: 所有主要机制都有测试卡覆盖
    Evidence: .sisyphus/evidence/task-28-mechanism-coverage.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `feat(scripts): add S000 test card pack with 8+ cards covering all types`
  - Files: `scripts/S000/*.lua`

- [ ] 29. TUI 应用骨架 + 主循环

  **What to do**:
  - 创建 `crates/card-tui/src/main.rs` + `crates/card-tui/src/app.rs`
  - `App` 结构体 — 应用状态:
    - mode: AppMode（MainMenu / LocalGame / OnlineGame / Connecting / InGame / GameOver）
    - game_state: Option<VisibleGameState>（从 ClientApi::on_event 更新）
    - pending_action: Option<ActionPrompt>（当前等待用户操作）
    - message_log: Vec<String>（事件日志显示）
    - selected_index: usize（UI 选择光标）
  - `TuiClient` 结构体 — 实现 ClientApi trait:
    - on_event → 更新 App.game_state + 追加 message_log
    - choose_action → 设置 pending_action，等待用户通过 TUI 选择
    - choose_targets → 同上，切换到目标选择 UI
    - choose_cards → 同上，切换到卡片选择 UI
    - 使用 tokio::sync::mpsc channel 在 UI 线程和 ClientApi 异步调用之间通信
  - **主循环** (main.rs):
    - 初始化 ratatui terminal (crossterm backend)
    - 事件循环: poll crossterm events + 渲染 UI
    - 退出: 'q' 键或 Ctrl+C
    - panic hook: 恢复终端
  - **主菜单**: 本地对战 (vs AI) / 联网对战 / 退出
  - **本地对战流程**: 选择卡组 → 创建 GameSession + AiClient → 运行游戏
  - **联网对战流程**: 输入服务器地址 → TCP 连接 → 提交卡组 → 运行游戏

  **Must NOT do**:
  - 不实现复杂动画
  - 不实现卡组编辑器（使用默认卡组）

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 5)
  - **Parallel Group**: Wave 5 (with Tasks 30-33)
  - **Blocks**: Tasks 30, 31, 33
  - **Blocked By**: Tasks 15, 22

  **References**:
  - `crates/card-client/src/api.rs` (Task 15) — ClientApi, VisibleGameState
  - `crates/card-server/src/session.rs` (Task 22) — GameSession
  - `crates/card-server/src/ai_client.rs` (Task 27) — AiClient
  - ratatui docs: Terminal, Frame, Widget trait, crossterm backend

  **Acceptance Criteria**:
  - [ ] `cargo run -p card-tui` 启动并显示主菜单
  - [ ] 'q' 键正确退出并恢复终端
  - [ ] TuiClient 正确实现 ClientApi

  **QA Scenarios**:
  ```
  Scenario: TUI 启动和退出
    Tool: interactive_bash (tmux)
    Steps:
      1. tmux new-session -d -s tui-test
      2. tmux send-keys -t tui-test "cargo run -p card-tui" Enter
      3. 等待 3 秒
      4. tmux capture-pane -t tui-test -p — 验证包含 "本地对战" 或 "Local Game"
      5. tmux send-keys -t tui-test "q"
      6. 验证进程退出
    Expected Result: TUI 正常启动显示菜单，q 退出
    Evidence: .sisyphus/evidence/task-29-tui-launch.txt

  Scenario: 终端 panic 恢复
    Tool: Bash
    Steps:
      1. cargo build -p card-tui — 确认编译成功
      2. 检查 main.rs 中有 panic hook 恢复终端
    Expected Result: panic 时终端不损坏
    Evidence: .sisyphus/evidence/task-29-panic-hook.txt
  ```

  **Commit**: YES (groups with Wave 5)
  - Message: `feat(tui): implement TUI application skeleton with TuiClient`
  - Files: `crates/card-tui/src/main.rs`, `crates/card-tui/src/app.rs`

- [ ] 30. TUI 游戏板面渲染

  **What to do**:
  - 创建 `crates/card-tui/src/ui/mod.rs` + 子模块
  - **游戏板面布局** (自上而下):
    - 对手后场（5格）
    - 对手前场（5格）
    - 中间信息栏（回合数、阶段、RP、HP）
    - 己方前场（5格）
    - 己方后场（5格）
    - 手牌区（滚动显示）
    - 费用区（折叠显示）
    - 操作提示/消息日志
  - **卡片显示组件**:
    - 小型卡片: `[C001 500]`（ID缩写 + 攻击力）
    - 空格位: `[  ---  ]`
    - 对手手牌: `[ ? ]` × N（仅显示数量）
    - 面朝下: `[ ??? ]`
  - **信息面板**:
    - 双方 HP（数字 + 彩色条）
    - 双方 RP（数字）
    - 当前回合数 + 阶段名
    - 卡组剩余数
    - 墓地数量
  - **消息日志区**: 显示最近 N 条 GameEvent 的中文描述
  - 使用 ratatui 的 Layout, Block, Table, Paragraph, Gauge 等 Widget
  - 所有 UI 文本为中文

  **Must NOT do**:
  - 不处理用户输入（那是 Task 31）
  - 不实现动画效果

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on T29)
  - **Parallel Group**: Wave 5 (after T29)
  - **Blocks**: Task 31
  - **Blocked By**: Task 29

  **References**:
  - `plan/ReadMe.md:40-48` — 游戏区域（前场5格、后场5格、费用区等）
  - `crates/card-client/src/api.rs` (Task 15) — VisibleGameState, OpponentView
  - ratatui docs: Layout, Constraint, Widget, Stylize trait

  **Acceptance Criteria**:
  - [ ] 游戏板面正确显示双方场地
  - [ ] HP/RP 信息正确显示
  - [ ] 对手手牌只显示数量不显示内容
  - [ ] 中文 UI 文本

  **QA Scenarios**:
  ```
  Scenario: 游戏板面渲染
    Tool: interactive_bash (tmux)
    Steps:
      1. 启动 TUI 进入本地对战
      2. 等待游戏开始
      3. tmux capture-pane 截取屏幕
      4. 验证包含: 前场/后场格位显示, HP 数字, 回合信息
    Expected Result: 板面布局正确渲染
    Evidence: .sisyphus/evidence/task-30-board-render.txt
  ```

  **Commit**: YES (groups with Wave 5)
  - Message: `feat(tui): implement game board rendering with card display`
  - Files: `crates/card-tui/src/ui/*.rs`

- [ ] 31. TUI 输入处理 + 操作选择

  **What to do**:
  - 创建 `crates/card-tui/src/input.rs`
  - **操作选择 UI**:
    - 当 pending_action 为 Some 时，显示可选操作列表
    - 上下箭头移动光标
    - Enter 确认选择
    - Esc 取消/返回
  - **出牌流程 UI**:
    1. 主阶段: 显示可选操作（出牌/激活效果/Pass）
    2. 选择手牌: 高亮可出的手牌，选择一张
    3. 费用支付: 显示费用需求，让玩家选择哪些手牌作为费用
    4. 目标选择: 高亮可选目标（对手前场卡），选择目标
  - **攻击宣言 UI**:
    1. 选择己方攻击者（前场可攻击的卡）
    2. 选择攻击目标（对手前场卡 or 直接攻击）
  - **连锁响应 UI**:
    1. 显示当前连锁栈内容
    2. 提示: 连锁激活 / 放弃连锁
  - **回收阶段 UI**:
    1. 显示费用区可回收的卡
    2. 多选（最多 X 张）
  - **KeyEvent 映射**:
    - 方向键: 导航
    - Enter: 确认
    - Esc: 取消/Pass
    - Tab: 切换手牌/场地焦点
    - h: 帮助信息
  - 通过 mpsc channel 将用户选择发回 TuiClient

  **Must NOT do**:
  - 不处理网络连接 UI（那在 Task 33）
  - 不实现拖放操作

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on T29, T30)
  - **Parallel Group**: Wave 5
  - **Blocks**: None
  - **Blocked By**: Tasks 29, 30

  **References**:
  - `crates/card-client/src/api.rs` (Task 15) — AvailableAction, ClientApi 方法签名
  - `crates/card-tui/src/app.rs` (Task 29) — App 状态, TuiClient channel
  - crossterm docs: KeyEvent, KeyCode

  **Acceptance Criteria**:
  - [ ] 玩家可以通过键盘选择操作并确认
  - [ ] 出牌流程（选卡→付费→放置）完整可操作
  - [ ] 攻击宣言可选择目标
  - [ ] 连锁响应可选择激活或放弃

  **QA Scenarios**:
  ```
  Scenario: 手动出牌操作
    Tool: interactive_bash (tmux)
    Steps:
      1. 启动本地对战
      2. 等待进入主阶段
      3. 按下箭头选择 "出牌"
      4. 按 Enter 确认
      5. 选择一张手牌 → Enter
      6. 选择费用卡 → Enter
      7. 验证卡片出现在前场
    Expected Result: 出牌流程完整可操作
    Evidence: .sisyphus/evidence/task-31-play-card.txt

  Scenario: Pass 操作
    Tool: interactive_bash (tmux)
    Steps:
      1. 在主阶段按 Esc（Pass）
      2. 验证阶段前进到下一阶段
    Expected Result: Pass 正确结束当前阶段
    Evidence: .sisyphus/evidence/task-31-pass.txt
  ```

  **Commit**: YES (groups with Wave 5)
  - Message: `feat(tui): implement keyboard input handling and action selection UI`
  - Files: `crates/card-tui/src/input.rs`

- [ ] 32. 匹配服务器

  **What to do**:
  - 创建 `crates/card-matchmaker/src/main.rs` + `crates/card-matchmaker/src/server.rs`
  - **WebSocket 服务器** (tokio-tungstenite):
    - 监听指定端口
    - 接受玩家连接
    - 消息格式: JSON（serde_json）
  - **匹配消息**:
    - `MatchRequest { player_name, version, deck_ids }` — 玩家请求匹配
    - `MatchFound { opponent_name, host_addr, is_host }` — 匹配成功
    - `MatchCancel` — 取消匹配
    - `VersionMismatch { required }` — 版本不兼容
    - `Error { message }` — 错误
  - **匹配流程**:
    1. 玩家 A 连接 + 发送 MatchRequest
    2. 检查版本兼容性
    3. 加入等待队列
    4. 玩家 B 连接 + MatchRequest
    5. 匹配成功: A 被指定为 host → MatchFound(is_host=true, host_addr=A's_addr)
    6. B 收到 MatchFound(is_host=false, host_addr=A's_addr)
    7. A 启动 GameServer，B 连接 A 的 GameServer
  - **简单匹配逻辑**: FIFO 队列，先来先配，无 ELO/段位
  - **超时**: 等待匹配 60 秒无结果则通知超时
  - **tracing** 日志记录所有连接/匹配事件

  **Must NOT do**:
  - 不实现 ELO/段位系统
  - 不持久化匹配记录
  - 不实现房间系统

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 5)
  - **Parallel Group**: Wave 5 (与 T29-31 并行)
  - **Blocks**: Task 33
  - **Blocked By**: Task 6

  **References**:
  - `.sisyphus/drafts/game-engine-design.md` — 匹配服务器设计
  - `plan/ReadMe.md:105-107` — 匹配服务器要求
  - tokio-tungstenite docs: accept_async, Message

  **Acceptance Criteria**:
  - [ ] `cargo run -p card-matchmaker` 启动 WebSocket 服务器
  - [ ] 两个客户端连接后被正确匹配
  - [ ] 版本不匹配时正确拒绝

  **QA Scenarios**:
  ```
  Scenario: 双人匹配
    Tool: Bash
    Steps:
      1. 启动 matchmaker on 127.0.0.1:9090
      2. 客户端A WebSocket连接 + MatchRequest(version="0.1.0")
      3. 客户端B WebSocket连接 + MatchRequest(version="0.1.0")
      4. A 收到 MatchFound(is_host=true)
      5. B 收到 MatchFound(is_host=false, host_addr=A)
      6. cargo test -p card-matchmaker
    Expected Result: 匹配成功，双方收到正确信息
    Evidence: .sisyphus/evidence/task-32-matchmaker.txt

  Scenario: 版本不匹配
    Tool: Bash
    Steps:
      1. 客户端发送 MatchRequest(version="0.0.1")
      2. 服务器当前版本 "0.1.0"
      3. 收到 VersionMismatch 消息
    Expected Result: 版本检查正确
    Evidence: .sisyphus/evidence/task-32-version-check.txt
  ```

  **Commit**: YES (groups with Wave 5)
  - Message: `feat(matchmaker): implement WebSocket matchmaking server`
  - Files: `crates/card-matchmaker/src/main.rs`, `crates/card-matchmaker/src/server.rs`

- [ ] 33. TUI 匹配客户端集成

  **What to do**:
  - 创建 `crates/card-tui/src/matchmaker_client.rs`
  - **联网对战流程** (TUI 侧):
    1. 用户选择 "联网对战"
    2. 输入匹配服务器地址（或使用默认 localhost:9090）
    3. WebSocket 连接到 matchmaker
    4. 发送 MatchRequest
    5. 显示 "等待匹配..." 动画
    6. 收到 MatchFound:
       - is_host=true → 启动 GameServer + 等待对手 TCP 连接
       - is_host=false → TCP 连接到 host_addr
    7. 进入游戏
  - **UI 状态**:
    - Connecting（连接匹配服务器）
    - WaitingForMatch（等待中，可 Esc 取消）
    - MatchFound（显示对手信息，准备开始）
    - ConnectingToHost（TCP 连接游戏服务器）
  - **错误处理**: 连接失败/超时 → 显示错误消息 → 返回主菜单
  - tokio-tungstenite 客户端连接

  **Must NOT do**:
  - 不实现好友列表/邀请
  - 不实现服务器地址持久化

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on T29, T32)
  - **Parallel Group**: Wave 5 (last)
  - **Blocks**: None
  - **Blocked By**: Tasks 29, 32

  **References**:
  - `crates/card-matchmaker/src/server.rs` (Task 32) — 匹配消息格式
  - `crates/card-tui/src/app.rs` (Task 29) — App 状态机
  - `crates/card-server/src/network.rs` (Task 24) — GameServer

  **Acceptance Criteria**:
  - [ ] TUI 可连接匹配服务器
  - [ ] 匹配成功后 TCP 连接建立
  - [ ] 取消匹配正确退出

  **QA Scenarios**:
  ```
  Scenario: 联网匹配流程
    Tool: interactive_bash (tmux)
    Steps:
      1. 启动 matchmaker
      2. 启动 TUI, 选择 "联网对战"
      3. 验证显示 "等待匹配..."
      4. 启动第二个 TUI 实例, 同样选择联网对战
      5. 验证双方进入游戏
    Expected Result: 匹配 → 连接 → 游戏开始
    Evidence: .sisyphus/evidence/task-33-online-match.txt

  Scenario: 连接失败处理
    Tool: interactive_bash (tmux)
    Steps:
      1. 不启动 matchmaker
      2. TUI 选择联网对战
      3. 验证显示连接错误消息
      4. 验证返回主菜单
    Expected Result: 错误被优雅处理
    Evidence: .sisyphus/evidence/task-33-connect-fail.txt
  ```

  **Commit**: YES (groups with Wave 5)
  - Message: `feat(tui): integrate matchmaker client for online play`
  - Files: `crates/card-tui/src/matchmaker_client.rs`

- [ ] 34. 核心引擎单元测试

  **What to do**:
  - 创建 `crates/card-core/tests/` 目录下的集成测试文件
  - **phase_test.rs** — 阶段运行器测试:
    - 完整回合流转（7阶段顺序）
    - 先手第一回合不抽卡
    - 回收阶段：对手前场最高费用计算
    - 卡组耗尽触发 DeckOut
    - HP 归零触发 GameOver
  - **trigger_test.rs** — 触发检查器测试:
    - OnSummon 正确匹配 CardSummoned
    - EventTrigger 正确匹配（RealPointOverflow 等）
    - 条件不满足阻止触发
    - OncePerTurn 限制
  - **chain_test.rs** — 连锁解算测试:
    - FILO 顺序验证（3层连锁）
    - 被动触发栈顶插入
    - 目标消失效果安全跳过
    - 瞬时策略卡立即结算
    - 双方 Pass 结束连锁
  - **action_test.rs** — 动作执行器测试:
    - 每种 Action 变体的正确性
    - 免疫阻止效果
    - RealPoint 溢出事件
    - 费用支付 OnExpose 触发
  - **modifier_test.rs** — 修饰器测试:
    - 攻击力增减和回退
    - UntilEndOfTurn 清理
    - WhileSourceOnField 清理
    - 免疫检查
  - **combat_test.rs** — 战斗测试:
    - 高攻击力胜出
    - 平局双方墓地
    - 直接攻击扣 HP
    - 多次攻击（ExtraAttackPerTurn 修饰器）

  **Must NOT do**:
  - 不测试网络/TUI（那是其他测试任务）

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 6)
  - **Parallel Group**: Wave 6 (with Tasks 35-38)
  - **Blocks**: None
  - **Blocked By**: Task 21

  **References**:
  - `crates/card-core/src/engine/` (Tasks 16-21) — 所有引擎模块
  - `plan/ReadMe.md:50-116` — 游戏规则（测试用例来源）

  **Acceptance Criteria**:
  - [ ] `cargo test -p card-core` 全部通过
  - [ ] 覆盖所有关键路径：阶段流转、触发匹配、连锁解算、动作执行、战斗结算
  - [ ] 每个测试文件至少 5 个测试用例

  **QA Scenarios**:
  ```
  Scenario: 引擎单元测试全通过
    Tool: Bash
    Steps:
      1. cargo test -p card-core -- --nocapture
      2. 验证输出包含 "test result: ok"
      3. 验证 0 failures
      4. 统计测试数量 >= 30
    Expected Result: 所有测试通过
    Evidence: .sisyphus/evidence/task-34-core-tests.txt
  ```

  **Commit**: YES (groups with Wave 6)
  - Message: `test(core): add comprehensive unit tests for engine subsystems`
  - Files: `crates/card-core/tests/*.rs`

- [ ] 35. Lua 加载单元测试

  **What to do**:
  - 创建 `crates/card-script/tests/` 目录
  - **loader_test.rs**:
    - ScriptIndex::scan 正确扫描目录
    - 文件名 → CardId 解析正确
    - 不存在的文件返回错误
  - **parser_test.rs**:
    - 解析基础人物卡（所有字段）
    - 解析带效果的人物卡（on_summon + draw）
    - 解析策略卡（普通/诡计/瞬时）
    - 解析物品卡（普通/存留）
    - 解析传奇卡
    - 解析嵌套条件（And/Or/Not）
    - 解析 EventTrigger
    - 解析 Modifier 效果
    - 畸形 table 返回 ScriptError
    - 缺少必填字段返回明确错误
  - **sandbox_test.rs**:
    - os.execute 被阻止
    - io.open 被阻止
    - require 被阻止
    - 无限循环被中断
    - 巨大内存分配被阻止
    - 正常脚本能执行
  - **integration_test.rs**:
    - 加载 S000 整个卡包
    - 验证所有卡正确解析
    - 验证类型覆盖

  **Must NOT do**:
  - 不测试游戏引擎逻辑

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 6)
  - **Parallel Group**: Wave 6
  - **Blocks**: None
  - **Blocked By**: Tasks 12, 13, 28

  **References**:
  - `crates/card-script/src/` (Tasks 11-13) — loader, parser, sandbox
  - `scripts/S000/` (Task 28) — 测试卡包

  **Acceptance Criteria**:
  - [ ] `cargo test -p card-script` 全部通过
  - [ ] 沙盒安全测试全部通过
  - [ ] S000 卡包全部正确加载

  **QA Scenarios**:
  ```
  Scenario: Lua 测试全通过
    Tool: Bash
    Steps:
      1. cargo test -p card-script -- --nocapture
      2. 验证 0 failures
      3. 验证测试数量 >= 15
    Expected Result: 所有 Lua 相关测试通过
    Evidence: .sisyphus/evidence/task-35-script-tests.txt
  ```

  **Commit**: YES (groups with Wave 6)
  - Message: `test(script): add Lua loader, parser, and sandbox tests`
  - Files: `crates/card-script/tests/*.rs`

- [ ] 36. 集成测试：本地对弈（2个 TestClient/AiClient）

  **What to do**:
  - 创建 `crates/card-server/tests/local_game_test.rs`
  - **测试场景 1: AI vs AI 完整对弈**
    - 使用 S000 卡包构建两个合法卡组
    - 创建 GameSession + 两个 AiClient
    - 运行游戏直到结束
    - 验证 GameResult 有 winner
    - 验证 event_log 非空
    - 验证最终状态一致性
  - **测试场景 2: TestClient 预编程对弈**
    - TestClient 预编程: P1 出人物卡 → 攻击 → Pass; P2 全程 Pass
    - 验证 P1 的卡正确出场
    - 验证攻击正确执行
    - 验证 P2 HP 减少（如果直接攻击）
  - **测试场景 3: 效果触发集成**
    - 使用 S000-C-002（on_summon draw 1）
    - 出牌后验证手牌增加 1
  - **测试场景 4: 确定性验证**
    - 相同 seed + 相同操作运行两次
    - assert_eq 最终状态
  - **测试场景 5: 边缘情况**
    - 卡组只有 40 张，打到卡组耗尽
    - HP 打到 0

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 6)
  - **Parallel Group**: Wave 6
  - **Blocks**: Task 38
  - **Blocked By**: Tasks 21, 27, 28

  **References**:
  - `crates/card-server/src/session.rs` (Task 22) — GameSession
  - `crates/card-server/src/ai_client.rs` (Task 27) — AiClient
  - `crates/card-client/src/api.rs` (Task 15) — TestClient
  - `scripts/S000/` (Task 28) — 测试卡包

  **Acceptance Criteria**:
  - [ ] AI vs AI 游戏正常完成
  - [ ] 效果触发正确
  - [ ] 确定性验证通过
  - [ ] `cargo test -p card-server -- local_game` 全部通过

  **QA Scenarios**:
  ```
  Scenario: 本地集成测试
    Tool: Bash
    Steps:
      1. cargo test -p card-server -- local_game --nocapture
      2. 验证 AI vs AI 游戏完成（输出包含 "GameOver"）
      3. 验证确定性测试通过
      4. 验证 0 failures
    Expected Result: 所有本地对弈测试通过
    Evidence: .sisyphus/evidence/task-36-local-game.txt
  ```

  **Commit**: YES (groups with Wave 6)
  - Message: `test(server): add local game integration tests with AI and TestClient`
  - Files: `crates/card-server/tests/local_game_test.rs`

- [ ] 37. 集成测试：联网对战（2进程 TCP）

  **What to do**:
  - 创建 `crates/card-server/tests/network_game_test.rs`
  - **测试场景 1: TCP 双人对战**
    - 启动 GameServer on 127.0.0.1:0（随机端口）
    - 两个 tokio task 作为客户端连接
    - 客户端用 AiClient 逻辑自动操作
    - 完成 Hello 握手 + 卡组提交 + 游戏
    - 验证游戏正常结束
  - **测试场景 2: 版本不匹配**
    - 客户端发送错误版本
    - 验证连接被正确拒绝
  - **测试场景 3: 客户端断连**
    - 游戏中一方断开 TCP
    - 验证另一方获胜（Timeout/Disconnect）
  - **测试场景 4: 超时处理**
    - 一方长时间不响应
    - 验证超时触发 GameOver

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (Wave 6)
  - **Parallel Group**: Wave 6
  - **Blocks**: None
  - **Blocked By**: Tasks 22, 23, 24

  **References**:
  - `crates/card-server/src/network.rs` (Task 24) — GameServer
  - `crates/card-server/src/remote_client.rs` (Task 23) — RemoteClient
  - `crates/card-protocol/src/codec.rs` (Task 14) — TcpConnection

  **Acceptance Criteria**:
  - [ ] TCP 双人对战正常完成
  - [ ] 断连/超时正确处理
  - [ ] `cargo test -p card-server -- network_game` 全部通过

  **QA Scenarios**:
  ```
  Scenario: TCP 联网对战测试
    Tool: Bash
    Steps:
      1. cargo test -p card-server -- network_game --nocapture
      2. 验证输出包含成功连接日志
      3. 验证游戏正常完成
      4. 验证 0 failures
    Expected Result: 联网测试通过
    Evidence: .sisyphus/evidence/task-37-network-game.txt
  ```

  **Commit**: YES (groups with Wave 6)
  - Message: `test(server): add networked game integration tests over TCP`
  - Files: `crates/card-server/tests/network_game_test.rs`

- [ ] 38. 回放正确性测试

  **What to do**:
  - 创建 `crates/card-server/tests/replay_test.rs`
  - **测试场景 1: 录制→回放→验证**
    - 运行一局 AI vs AI 游戏，ReplayRecorder 记录
    - finalize() 获取 ReplayData
    - 保存到临时文件
    - 从文件加载
    - 用 ReplayClient 重放
    - assert_eq!(original.final_state, replayed.final_state)
  - **测试场景 2: 多局回放一致性**
    - 运行 3 局不同 seed 的游戏
    - 每局录制 + 回放 + 验证
    - 全部一致
  - **测试场景 3: 回放文件格式**
    - 验证 JSON 格式正确
    - 验证包含所有必要字段
    - 验证 commands 和 events 数量一致
  - **测试场景 4: 损坏回放处理**
    - 修改 JSON 中一条 command
    - 回放应能检测到不一致（verify_replay 返回 false 或 error）

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on T36 providing tested game infrastructure)
  - **Parallel Group**: Wave 6 (after T36)
  - **Blocks**: None
  - **Blocked By**: Tasks 25, 36

  **References**:
  - `crates/card-server/src/replay.rs` (Task 25) — ReplayData, ReplayClient, verify_replay
  - `crates/card-server/tests/local_game_test.rs` (Task 36) — 可复用游戏设置代码

  **Acceptance Criteria**:
  - [ ] 录制→回放→验证一致性
  - [ ] 多局均一致
  - [ ] 损坏回放被检测
  - [ ] `cargo test -p card-server -- replay` 全部通过

  **QA Scenarios**:
  ```
  Scenario: 回放正确性全测试
    Tool: Bash
    Steps:
      1. cargo test -p card-server -- replay --nocapture
      2. 验证所有回放一致性检查通过
      3. 验证损坏检测正确
      4. 验证 0 failures
    Expected Result: 所有回放测试通过
    Evidence: .sisyphus/evidence/task-38-replay-test.txt
  ```

  **Commit**: YES (groups with Wave 6)
  - Message: `test(server): add replay recording and verification tests`
  - Files: `crates/card-server/tests/replay_test.rs`

---

## Final Verification Wave

- [ ] F1. **计划合规审计** — `oracle`
  读取完整计划。对每个"Must Have"验证实现是否存在（读文件、curl端点、运行命令）。对每个"Must NOT Have"搜索代码库中是否出现禁止模式——发现则拒绝并报告 file:line。检查 .sisyphus/evidence/ 中证据文件是否存在。对比交付物与计划。
  输出: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **代码质量审查** — `unspecified-high`
  运行 `cargo build --workspace` + `cargo clippy --workspace` + `cargo test --workspace`。审查所有变更文件：`as any`/unsafe（card-script外）/空catch/println!在生产代码/注释掉的代码/未使用import。检查AI痕迹：过度注释、过度抽象、泛型命名。
  输出: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **完整 QA** — `unspecified-high`
  从干净状态开始。执行每个任务的QA场景——按精确步骤操作、捕获证据。测试跨任务集成（功能协同工作）。测试边缘情况：空状态、非法输入、快速操作。保存到 `.sisyphus/evidence/final-qa/`。
  输出: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [ ] F4. **范围忠实度检查** — `deep`
  对每个任务：读取"What to do"，读取实际 diff。验证 1:1——spec 中的所有内容都已构建（无遗漏），spec 之外的内容未构建（无蔓延）。检查"Must NOT do"合规。检测跨任务污染：Task N 修改了 Task M 的文件。标记未计入的变更。
  输出: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Wave |提交消息 | 包含文件 | 前置检查 |
|------|---------|---------|---------|
| 1 | `feat: scaffold workspace with 7 crates and core types` | 所有 Wave 1 文件 | `cargo build --workspace` |
| 2 | `feat: implement core state, effect system, lua loader, and protocol codec` | Wave 2 文件 | `cargo build --workspace` |
| 3 | `feat: implement game engine core loop with chain resolution` | Wave 3 文件 | `cargo test -p card-core` |
| 4 | `feat: add server session, networking, replay, save, AI, and test cards` | Wave 4 文件 | `cargo build --workspace` |
| 5 | `feat: implement TUI client and matchmaking server` | Wave 5 文件 | `cargo build --workspace` |
| 6 | `test: add unit tests, integration tests, and replay verification` | Wave 6 文件 | `cargo test --workspace` |

---

## Success Criteria

### 验证命令
```bash
cargo build --workspace          # Expected: 无错误
cargo test --workspace           # Expected: 全部 PASS
cargo clippy --workspace         # Expected: 无警告
cargo run -p card-tui            # Expected: TUI 界面正常显示
cargo run -p card-matchmaker     # Expected: WebSocket 服务器启动
```

### 最终检查清单
- [ ] 所有"Must Have"功能已实现
- [ ] 所有"Must NOT Have"模式不存在
- [ ] `cargo test --workspace` 全部通过
- [ ] 两个 TestClient 可自动完成一局对弈
- [ ] TCP 联网对战可正常进行
- [ ] 回放录制→回放→状态一致
- [ ] TUI 客户端可正常操作
- [ ] 中文设计文档完整
- [ ] S000 测试卡包包含 8+ 张覆盖所有类型的卡
