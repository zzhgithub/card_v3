# TUI 重构 — 可正常游玩的对战界面

## TL;DR

> **Quick Summary**: 重构 card-tui crate 的游戏对战界面，从纯垂直堆叠布局改为左右分栏式布局（左场地+右信息面板），新增场地光标浏览系统、效果转中文自然语言接口、HP/RP图形化显示、半透明弹窗系统、快捷键导航和彩色可滚动日志。
>
> **Deliverables**:
> - 全新左右分栏游戏布局（左=场地，右=卡片信息+操作面板）
> - Effect→中文自然语言转换模块（card-core）+ 单元测试
> - 场地光标系统（浏览场地卡、高亮选中、查看详情）
> - HP星星/RP圆圈图形化显示
> - 前后场横排+费用区竖排布局
> - 手卡显示（自己=名称+编号，对手=背面）
> - 半透明叠加弹窗（可最小化/切换）
> - 全区域快捷键导航
> - 彩色可滚动日志区域
> - app.rs 拆分重构（渲染/输入/状态分离）
>
> **Estimated Effort**: Large
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: Task 1(拆分) → Task 2(布局) → Task 5(光标) → Task 8(弹窗) → F1-F4

---

## Context

### Original Request
NewPlan002: "进行如下重构TUI使之可以正常游玩" — 12项具体UI需求，涵盖布局重构、效果展示、光标系统、弹窗交互、日志增强等。

### Interview Summary
**Key Discussions**:
- 效果转自然语言模块放在 card-core（所有客户端复用）
- 测试策略：仅效果文本模块写单元测试，TUI渲染通过 Agent QA 验证
- 半透明弹窗：终端无法真正透明，用叠加渲染+部分场地可见模拟
- 快捷键：自行设计合理映射

**Research Findings**:
- 当前 app.rs 1712行，渲染+逻辑+输入完全混合，需要拆分
- Effect AST 完整（12种触发、14种动作、条件树、选择、代价），需要为每种类型编写中文转换
- CardDefinition.effects 存储为 `HashMap<EffectKey, serde_json::Value>`，需反序列化为 Effect 结构
- ratatui 0.29 支持 Block, Paragraph, Scrollbar, StatefulWidget
- 现有弹窗用 `Clear` + `centered_rect` 完全覆盖，需改为保留底层部分可见

### Self-Performed Gap Analysis（Metis 超时，自行补充）
**Identified Gaps** (addressed):
- CardDefinition.effects 是 JSON Value，转文本前需先解析为 Effect 结构 — 在效果文本模块中处理
- 对手费用区卡片已有 `CardPublicInfo`，可以显示 — 确认数据可用
- 对手手卡数量已有 `hand_count` 字段 — 确认数据可用
- 卡片名称需要通过 CardRegistry 查找（CardInstance 只有 definition_id） — 需在 TUI 中持有 registry 引用或传入卡名映射
- 弹窗最小化需要新增 UI 状态管理 — 新增 overlay state
- 快捷键需要不与已有按键冲突 — 设计时检查

---

## Work Objectives

### Core Objective
将 card-tui 的对战界面从不可交互的纯展示改为可完整游玩的交互式界面，具备场地浏览、卡片信息查看、效果理解和流畅操作体验。

### Concrete Deliverables
- `card-core/src/effect/text.rs` — Effect→中文文本转换模块
- `card-core/src/effect/text_tests.rs` — 单元测试
- `card-tui/src/ui/` — 拆分后的渲染模块目录
  - `mod.rs`, `layout.rs`, `field.rs`, `info_panel.rs`, `hand.rs`, `log.rs`, `overlay.rs`, `hp_display.rs`
- `card-tui/src/input.rs` — 增强的输入处理（含快捷键）
- `card-tui/src/app.rs` — 精简后的应用状态管理
- `card-tui/src/cursor.rs` — 场地光标状态管理

### Definition of Done
- [ ] `cargo build -p card-tui` 编译通过无 warning
- [ ] `cargo test -p card-core` 效果文本测试全部通过
- [ ] 本地对战可以从头玩到结束（出牌、攻击、回收、跳过、结束）
- [ ] 场地光标可以浏览所有区域的卡片并查看信息
- [ ] 弹窗可以最小化查看场地后再恢复

### Must Have
- 左右分栏布局（左场地右信息）
- Effect→中文自然语言转换（覆盖所有 Trigger、Action、Condition 类型）
- 场地光标 + 高亮
- HP星/RP圈图形化
- 前后场横排 + 费用区竖排
- 手卡区（自己=名称编号，对手=背面）
- 弹窗半透明+可最小化
- 快捷键导航
- 彩色日志+滚动

### Must NOT Have (Guardrails)
- 不修改游戏引擎逻辑（card-core/engine/）
- 不修改网络协议（card-protocol/）
- 不修改服务器逻辑（card-server/）
- 不修改 Lua 脚本加载（card-script/）
- 不新增功能：费用支付选择UI、链式选择UI等（这些是后续任务，本次仅重构展示层）
- 不要过度抽象：保持直接的 ratatui widget 调用，不创建自己的 widget 框架
- 不要引入新的 crate 依赖（ratatui + crossterm 已足够）
- 不要改变 PhaseClient trait 接口
- 不要翻译或改写已有的中文 UI 字符串风格

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (cargo test 已配置)
- **Automated tests**: YES (Tests-after) — 仅 effect text 模块
- **Framework**: cargo test (标准 Rust #[test])
- **TUI rendering**: Agent QA only（通过 tmux 运行 TUI 截图验证）

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Effect text module**: Use Bash (cargo test) — Run tests, assert pass
- **TUI rendering**: Use interactive_bash (tmux) — Launch TUI, navigate, screenshot
- **Compilation**: Use Bash (cargo build/clippy) — Zero errors/warnings

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — foundation + independent modules):
├── Task 1: 拆分 app.rs 为模块结构 [deep]
├── Task 2: Effect→中文文本转换模块 + 测试 [deep]
├── Task 3: 光标状态管理模块 [quick]
└── Task 4: HP星/RP圈渲染工具函数 [quick]

Wave 2 (After Wave 1 — core layout + rendering):
├── Task 5: 左右分栏主布局 + 场地区域渲染 [deep]
├── Task 6: 右侧信息面板（卡片信息+效果文本+状态） [unspecified-high]
├── Task 7: 手卡区域渲染 [quick]
└── Task 8: 彩色可滚动日志区域 [unspecified-high]

Wave 3 (After Wave 2 — interaction + overlay):
├── Task 9: 场地光标导航+高亮+快捷键 [deep]
├── Task 10: 半透明弹窗系统（可最小化/切换） [deep]
└── Task 11: 输入系统重构+快捷键映射 [unspecified-high]

Wave 4 (After Wave 3 — integration + polish):
├── Task 12: 全流程集成联调 [deep]
└── Task 13: 编译检查+clippy清理 [quick]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | — | 5, 6, 7, 8, 9, 10, 11, 12 | 1 |
| 2 | — | 6 | 1 |
| 3 | — | 9 | 1 |
| 4 | — | 5 | 1 |
| 5 | 1, 4 | 9, 10, 12 | 2 |
| 6 | 1, 2 | 12 | 2 |
| 7 | 1 | 12 | 2 |
| 8 | 1 | 12 | 2 |
| 9 | 3, 5 | 12 | 3 |
| 10 | 5 | 12 | 3 |
| 11 | 1 | 12 | 3 |
| 12 | 5, 6, 7, 8, 9, 10, 11 | 13 | 4 |
| 13 | 12 | F1-F4 | 4 |

### Agent Dispatch Summary

- **Wave 1**: **4** — T1 → `deep`, T2 → `deep`, T3 → `quick`, T4 → `quick`
- **Wave 2**: **4** — T5 → `deep`, T6 → `unspecified-high`, T7 → `quick`, T8 → `unspecified-high`
- **Wave 3**: **3** — T9 → `deep`, T10 → `deep`, T11 → `unspecified-high`
- **Wave 4**: **2** — T12 → `deep`, T13 → `quick`
- **FINAL**: **4** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. 拆分 app.rs 为模块结构

  **What to do**:
  - 将 `card-tui/src/app.rs`（1712行）拆分为独立模块：
    - `card-tui/src/app.rs` — 保留 `App` struct 定义、`AppMode` enum、`run()` 主循环、状态管理方法（`drain_game_events`, `drain_game_result`, `reset_to_main_menu`）、游戏启动方法（`start_game_with_decks`, `start_local_game`, `start_online_game`）
    - `card-tui/src/ui/mod.rs` — 模块声明，re-export 渲染函数
    - `card-tui/src/ui/layout.rs` — `render_game_screen()` 主布局函数（从 app.rs 提取 `render_game_screen`）
    - `card-tui/src/ui/field.rs` — 场地渲染（`render_game_board` 从旧 ui.rs 迁移并重构）
    - `card-tui/src/ui/info_panel.rs` — 右侧信息面板（新建，占位）
    - `card-tui/src/ui/hand.rs` — 手卡渲染（从旧 ui.rs 提取 `render_hand`）
    - `card-tui/src/ui/log.rs` — 日志渲染（从旧 ui.rs 提取 `render_event_log`）
    - `card-tui/src/ui/overlay.rs` — 弹窗渲染（从 app.rs 提取 `render_action_overlay`, `render_recovery_overlay`, `centered_rect`）
    - `card-tui/src/ui/hp_display.rs` — HP/RP图形渲染工具（新建，占位）
    - `card-tui/src/ui/menu.rs` — 主菜单/卡组浏览/卡组选择渲染（从 app.rs 提取 `render_main_menu`, `render_deck_browser`, `render_deck_selection`, `render_deck_editor`）
  - 将辅助函数移至对应模块：`compact_card_id`, `render_card_slot`, `render_instance_slot`, `render_public_row`, `render_private_row`, `phase_name`
  - 将网络相关函数提取到 `card-tui/src/network.rs`：`run_online_game_session`, `run_host_flow`, `run_guest_flow`, `run_network_client`, `derive_host_bind_addr`, `available_actions_to_phase_actions`, `phase_action_to_command`, `build_demo_deck_ids`
  - 将游戏构建函数提取到 `card-tui/src/game_setup.rs`：`run_local_game`, `find_scripts_root`, `build_game_from_decks`, `build_demo_state`
  - 删除旧的 `card-tui/src/ui.rs`（内容已迁移到 `ui/` 目录）
  - **关键约束**：纯机械移动+重命名，不改变任何逻辑行为。所有 pub 函数保持相同签名。

  **Must NOT do**:
  - 不修改任何渲染逻辑或显示内容
  - 不修改输入处理逻辑
  - 不修改游戏启动流程
  - 不引入新的 trait 或抽象

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 大量文件拆分需要仔细追踪引用关系，确保无遗漏
  - **Skills**: []
  - **Skills Evaluated but Omitted**:
    - `test-driven-development`: 纯重构无新逻辑，不需要TDD

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4)
  - **Blocks**: Tasks 5, 6, 7, 8, 9, 10, 11, 12
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `crates/card-tui/src/app.rs:1-1712` — 完整文件，需拆分的源头。关注 `render_*` 方法（202-775行）、`handle_key`（392-532行）、网络函数（1327-1529行）、游戏构建（1619-1712行）
  - `crates/card-tui/src/ui.rs:1-191` — 当前渲染模块，内容将迁移到 `ui/field.rs` 等

  **API/Type References**:
  - `crates/card-tui/src/app.rs:42-66` — `UiEvent`, `AppMode` enum 定义，保留在 app.rs
  - `crates/card-tui/src/app.rs:74-92` — `TuiClient` struct，保留在 app.rs
  - `crates/card-tui/src/app.rs:121-148` — `App` struct 定义，保留在 app.rs

  **WHY Each Reference Matters**:
  - app.rs 是拆分的唯一源文件，需要逐函数决定目标位置
  - ui.rs 的内容将成为新 ui/ 目录的基础

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 编译验证 — 拆分后编译通过
    Tool: Bash
    Preconditions: Wave 1 Task 1 实施完成
    Steps:
      1. 运行 `cargo build -p card-tui`
      2. 检查输出无 error
      3. 运行 `cargo clippy -p card-tui -- -D warnings` 检查无 warning
    Expected Result: 编译成功，零 error，零 warning
    Failure Indicators: 任何编译错误或 unresolved import
    Evidence: .sisyphus/evidence/task-1-compile-check.txt

  Scenario: 功能保持 — 拆分后 TUI 仍可启动
    Tool: interactive_bash (tmux)
    Preconditions: 编译通过
    Steps:
      1. tmux new-session -d -s tui-test
      2. tmux send-keys -t tui-test "cargo run -p card-tui" Enter
      3. 等待 5 秒
      4. tmux capture-pane -t tui-test -p 检查输出包含 "主菜单"
      5. tmux send-keys -t tui-test "q"
    Expected Result: TUI 正常启动并显示主菜单
    Failure Indicators: panic 或无法启动
    Evidence: .sisyphus/evidence/task-1-startup-check.txt

  Scenario: 文件结构验证 — 新模块文件存在
    Tool: Bash
    Preconditions: 拆分完成
    Steps:
      1. 检查以下文件存在：ui/mod.rs, ui/layout.rs, ui/field.rs, ui/info_panel.rs, ui/hand.rs, ui/log.rs, ui/overlay.rs, ui/hp_display.rs, ui/menu.rs, network.rs, game_setup.rs
      2. 检查旧 ui.rs 已删除
    Expected Result: 所有目标文件存在，旧文件已移除
    Failure Indicators: 缺少文件或旧文件仍在
    Evidence: .sisyphus/evidence/task-1-file-structure.txt
  ```

  **Commit**: YES
  - Message: `refactor(tui): split app.rs into modular structure`
  - Files: `crates/card-tui/src/app.rs`, `crates/card-tui/src/ui/mod.rs`, `crates/card-tui/src/ui/layout.rs`, `crates/card-tui/src/ui/field.rs`, `crates/card-tui/src/ui/info_panel.rs`, `crates/card-tui/src/ui/hand.rs`, `crates/card-tui/src/ui/log.rs`, `crates/card-tui/src/ui/overlay.rs`, `crates/card-tui/src/ui/hp_display.rs`, `crates/card-tui/src/ui/menu.rs`, `crates/card-tui/src/network.rs`, `crates/card-tui/src/game_setup.rs`
  - Pre-commit: `cargo build -p card-tui`

- [x] 2. Effect→中文自然语言转换模块 + 单元测试

  **What to do**:
  - 新建 `card-core/src/effect/text.rs` — Effect→中文文本转换模块
  - 实现核心函数 `pub fn effect_to_chinese(effect: &Effect) -> String`
  - 实现辅助转换函数：
    - `fn trigger_text(trigger: &Trigger) -> &'static str` — 12种触发时点→中文（"登场时"、"攻击时"、"暴露时"、"破坏时"、"回合开始时"、"自己主要阶段"、"对方主要阶段"、"双方主要阶段"、"回合结束阶段"等）
    - `fn condition_text(condition: &Condition) -> String` — 条件树→中文（递归处理 And/Or/Not/Compare/CardIsOnField）
    - `fn action_text(action: &Action) -> String` — 14种动作→中文（"抽N张卡"、"造成N点伤害"、"破坏目标卡"、"不支付费用登场"、"返回手卡"、"送入墓地"、"攻击力增减N"、"获得N点真实点数"、"恢复N点生命值"、"丢弃N张手卡"、"施加修改器"、"移除修改器"）
    - `fn choice_text(choice: &Choice) -> String` — 选择→中文（"选择N张[筛选]卡"）
    - `fn cost_text(cost: &CostRequirement) -> String` — 代价→中文（"将场上N张卡送入墓地"、"丢弃N张手卡"、"将费用区N张卡送入墓地"）
    - `fn value_expr_text(expr: &ValueExpr) -> String` — 值表达式→中文（"自己手卡数"、"对手真实点数"等）
    - `fn compare_op_text(op: CompareOp) -> &'static str` — 比较符→中文（"大于"、"小于"等）
    - `fn player_ref_text(player: &PlayerRef) -> &'static str` — 玩家引用→中文（"自己"、"对手"、"此卡控制者"）
    - `fn card_ref_text(card: &CardRef) -> String` — 卡引用→中文
    - `fn modifier_text(modifier: &Modifier) -> String` — 修改器→中文
    - `fn activation_limit_text(limit: &ActivationLimit) -> &'static str` — 限制→中文
    - `fn card_filter_text(filter: &CardFilter) -> String` — 卡过滤器→中文
  - 实现 `pub fn effects_from_json(effects: &HashMap<EffectKey, serde_json::Value>) -> Vec<(EffectKey, Result<Effect, String>)>` — 从 CardDefinition 的 JSON Value 反序列化效果
  - 实现 `pub fn card_effects_text(effects: &HashMap<EffectKey, serde_json::Value>) -> Vec<String>` — 一步完成：JSON→Effect→中文文本列表
  - 新建测试文件或在 text.rs 内 `#[cfg(test)] mod tests`：
    - 测试每种 Trigger 的文本输出
    - 测试 Condition 树的组合（And+Compare、Or+Not）
    - 测试每种 Action 的文本输出
    - 测试完整 Effect 的端到端转换
    - 测试设计规范中的 7 个效果示例（plan/ReadMe.md 底部）
  - 在 `card-core/src/effect/mod.rs` 中 `pub mod text;`

  **Must NOT do**:
  - 不修改 Effect 结构体本身
  - 不修改其他已有模块
  - 不引入新依赖

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 需覆盖大量枚举变体，逻辑密集，需要仔细处理每种类型的中文表述
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3, 4)
  - **Blocks**: Task 6
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `crates/card-core/src/effect/mod.rs:1-349` — 完整 Effect AST 定义：所有 Trigger(47-80行), Condition(118-134行), Action(189-229行), Choice(237-277行), CostRequirement(284-298行), Modifier(306-325行), ValueExpr(157-181行) 变体。每种都需要中文转换。

  **API/Type References**:
  - `crates/card-core/src/types/mod.rs:17-21` — CardFilter, CardRef, PlayerRef, Zone 类型
  - `crates/card-core/src/types/card_definition.rs:6-18` — CardDefinition.effects 字段为 `HashMap<EffectKey, serde_json::Value>`
  - `crates/card-core/src/types/card_types.rs` — CardType, Property, Category 枚举定义

  **External References**:
  - `plan/ReadMe.md` 底部"效果例子" — 7个完整效果示例，用于验证自然语言输出的准确性

  **WHY Each Reference Matters**:
  - effect/mod.rs 是转换的输入类型定义，每个变体都需对应中文文本
  - ReadMe.md 的效果示例是验证输出正确性的金标准

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 单元测试通过
    Tool: Bash
    Preconditions: text.rs 编写完成
    Steps:
      1. 运行 `cargo test -p card-core effect::text`
      2. 检查所有测试通过
    Expected Result: 所有测试 PASS，覆盖全部 Trigger/Action/Condition 类型
    Failure Indicators: 任何测试 FAIL 或编译错误
    Evidence: .sisyphus/evidence/task-2-unit-tests.txt

  Scenario: 效果示例验证 — 登场召唤效果
    Tool: Bash
    Preconditions: text.rs 编写完成
    Steps:
      1. 编写测试：构造一个 OnSummon trigger + SummonFromZone action（从卡组/费用区/墓地召唤指定卡，不支付费用）
      2. 调用 effect_to_chinese()
      3. 断言输出包含"登场时"、"不支付费用"、"登场"
    Expected Result: 输出类似"登场时，将卡组、费用区、墓地中一张【XXX】不支付费用登场。"
    Failure Indicators: 输出不包含关键中文词汇
    Evidence: .sisyphus/evidence/task-2-summon-example.txt

  Scenario: 条件树组合验证
    Tool: Bash
    Preconditions: text.rs 编写完成
    Steps:
      1. 构造 And(Compare(HandCount(Opponent) > HandCount(Self)), Compare(CostZonePropertyCount(Self, Rational) >= Literal(3)))
      2. 调用 condition_text()
      3. 断言输出包含"对手手卡数"、"大于"、"自己手卡数"、"且"、"理性值"、"大于等于"、"3"
    Expected Result: 条件树正确组合为中文逻辑表达式
    Failure Indicators: 逻辑连接词缺失或条件顺序错误
    Evidence: .sisyphus/evidence/task-2-condition-tree.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add Effect-to-Chinese-text conversion module with tests`
  - Files: `crates/card-core/src/effect/text.rs`, `crates/card-core/src/effect/mod.rs`
  - Pre-commit: `cargo test -p card-core effect::text`

- [x] 3. 光标状态管理模块

  **What to do**:
  - 新建 `card-tui/src/cursor.rs` — 场地光标状态管理
  - 定义 `FieldZone` 枚举：
    ```
    enum FieldZone {
        OpponentBack(usize),    // 对手后场 0-4
        OpponentFront(usize),   // 对手前场 0-4
        MyFront(usize),         // 己方前场 0-4
        MyBack(usize),          // 己方后场 0-4
        MyCostZone(usize),      // 己方费用区 0-5
        MyHand(usize),          // 己方手卡 index
        OpponentCostZone(usize), // 对手费用区
    }
    ```
  - 定义 `FieldCursor` struct：
    ```
    struct FieldCursor {
        zone: FieldZone,
        active: bool,  // 光标是否激活（在场地浏览模式时）
    }
    ```
  - 实现导航方法：
    - `move_up()` — 在区域间垂直移动（对手后场↔对手前场↔己方前场↔己方后场↔手卡）
    - `move_down()` — 反方向
    - `move_left()` — 同区域内左移
    - `move_right()` — 同区域内右移
    - `jump_to(zone: FieldZone)` — 快捷键直接跳转
    - `current_zone() -> &FieldZone` — 获取当前位置
    - `slot_index() -> usize` — 获取当前槽位索引
  - 导航逻辑：
    - 垂直移动时保持水平位置（如前场[2]→后场[2]）
    - 移动到费用区时切换为竖排逻辑
    - 到达边界时环绕（wrap around）
    - 空槽位可以选中但不显示信息

  **Must NOT do**:
  - 不涉及渲染逻辑（纯数据层）
  - 不依赖 ratatui
  - 不修改其他文件

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 纯数据结构+简单导航逻辑，不涉及复杂渲染
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 4)
  - **Blocks**: Task 9
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `crates/card-core/src/state/mod.rs:86-93` — PlayerZones 结构（front[5], back[5], cost_zone Vec, hand Vec）定义了光标需要导航的区域尺寸

  **API/Type References**:
  - `crates/card-core/src/types/zone.rs` — Zone 枚举定义（Front(usize), Back(usize), CostZone, Hand, Deck, Grave）
  - `crates/card-client/src/api.rs:41-59` — OpponentView 有 front[5], back[5], cost_zone Vec — 对手区域也需要光标支持

  **WHY Each Reference Matters**:
  - PlayerZones 决定了光标的有效范围和边界
  - 光标的 FieldZone 枚举需要对齐引擎的 Zone 枚举

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 编译检查
    Tool: Bash
    Preconditions: cursor.rs 编写完成
    Steps:
      1. 运行 `cargo build -p card-tui`
    Expected Result: 编译成功
    Failure Indicators: 编译错误
    Evidence: .sisyphus/evidence/task-3-compile.txt

  Scenario: 导航逻辑正确 — 基本移动
    Tool: Bash
    Preconditions: cursor.rs 包含 #[cfg(test)] 内联测试
    Steps:
      1. 创建 FieldCursor 初始位置 MyFront(0)
      2. move_right() → MyFront(1)
      3. move_up() → OpponentFront(1)
      4. move_up() → OpponentBack(1)
      5. move_down() 3次 → MyBack(1)
      6. 运行 cargo test -p card-tui cursor
    Expected Result: 所有断言通过
    Failure Indicators: 位置不正确或越界
    Evidence: .sisyphus/evidence/task-3-navigation.txt
  ```

  **Commit**: YES (groups with Task 4)
  - Message: `feat(tui): add field cursor state management and HP/RP display utils`
  - Files: `crates/card-tui/src/cursor.rs`
  - Pre-commit: `cargo build -p card-tui`

- [x] 4. HP星/RP圈渲染工具函数

  **What to do**:
  - 在 `card-tui/src/ui/hp_display.rs`（Task 1 创建的占位文件）中实现：
  - `pub fn render_hp(hp: u8, max_hp: u8) -> Span<'static>` — 生命值图形化
    - 满HP：★（实心星），空HP：☆（空心星）
    - 例：HP=3, max=5 → "★★★☆☆"
    - 颜色：≥3 绿色，2 黄色，1 红色加粗
  - `pub fn render_rp(rp: u8, max_rp: u8) -> Span<'static>` — 真实点数图形化
    - 满RP：●（实心圆），空RP：○（空心圆）
    - 例：RP=2, max=6 → "●●○○○○"
    - 颜色：蓝色
  - 如果 Task 1 尚未完成（文件不存在），直接在 `card-tui/src/` 下创建 `hp_display.rs` 并在 main.rs 中声明模块

  **Must NOT do**:
  - 不依赖游戏状态，纯输入→Span输出
  - 不涉及布局

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 简单的字符串格式化+颜色设置
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3)
  - **Blocks**: Task 5
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `crates/card-tui/src/ui.rs:9` — `DEFAULT_MAX_HP: u8 = 5` — 当前HP最大值常量
  - `crates/card-tui/src/app.rs:31-33` — `use ratatui::style::{Color, Modifier, Style}; use ratatui::text::{Line, Span};` — ratatui 的样式和文本类型

  **WHY Each Reference Matters**:
  - 需要知道 HP/RP 的最大值范围来设计图形输出
  - 需要使用 ratatui 的 Span + Style 来设置颜色

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: HP 图形正确
    Tool: Bash
    Preconditions: hp_display.rs 编写完成
    Steps:
      1. 编写 #[test] 测试 render_hp(3, 5) 的内容
      2. 断言文本内容为 "★★★☆☆"
      3. 测试 render_hp(0, 5) → "☆☆☆☆☆"
      4. 测试 render_hp(5, 5) → "★★★★★"
      5. 运行 `cargo test -p card-tui hp_display`
    Expected Result: 所有断言通过
    Failure Indicators: 星星数量不对或字符错误
    Evidence: .sisyphus/evidence/task-4-hp-display.txt

  Scenario: RP 图形正确
    Tool: Bash
    Preconditions: hp_display.rs 编写完成
    Steps:
      1. 测试 render_rp(2, 6) 的内容
      2. 断言文本内容为 "●●○○○○"
      3. 运行 `cargo test -p card-tui hp_display`
    Expected Result: 圆圈数量和样式正确
    Failure Indicators: 圆圈数量不对
    Evidence: .sisyphus/evidence/task-4-rp-display.txt
  ```

  **Commit**: YES (groups with Task 3)
  - Message: `feat(tui): add field cursor state management and HP/RP display utils`
  - Files: `crates/card-tui/src/ui/hp_display.rs` (或独立 hp_display.rs)
  - Pre-commit: `cargo test -p card-tui hp_display`

- [x] 5. 左右分栏主布局 + 场地区域渲染

  **What to do**:
  - 重写 `card-tui/src/ui/layout.rs` 的 `render_game_screen()` 函数，实现左右分栏：
    - 整体布局：上方标题栏(3行) + 中间主体(填满) + 下方提示栏(2行)
    - 中间主体左右分栏：左70% = 场地区域，右30% = 信息面板+操作面板
    - 右侧面板再垂直分：上60% = 卡片信息面板（Task 6），下40% = 操作选择面板
  - 重写 `card-tui/src/ui/field.rs` 的场地渲染：
    - 场地区域垂直布局：对手后场 → 对手前场 → **信息分隔栏**（回合/阶段/连锁） → 己方前场 → 己方后场 → 费用区 → 手卡区
    - **前场/后场横排**：5个槽位水平排列，每槽位等宽，使用 Layout::Horizontal
    - **费用区竖排**：己方费用区竖着排列（每行一张卡），最多6张
    - 对手费用区在对手侧也以简化形式竖排显示
    - 每个卡槽渲染：有卡时显示 `[ID ATK]`，空时显示 `[  ---  ]`
    - **HP/RP 显示**：使用 Task 4 的 render_hp/render_rp 在对应区域旁边显示
      - 对手 HP★ + RP● 在对手前场旁
      - 己方 HP★ + RP● 在己方前场旁
    - **状态信息**放在场地右手侧（信息分隔栏内或场地区的右边缘）
  - 手卡区渲染初步实现（详细版在 Task 7）：
    - 己方手卡：显示卡片名称和编号 `[S001-C-001 训练木桩]`
    - 对手手卡：显示背面 `[?]×N`

  **Must NOT do**:
  - 不实现光标高亮（Task 9）
  - 不实现弹窗叠加（Task 10）
  - 不改变游戏逻辑

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 核心布局重构，需要精确的 ratatui Layout 嵌套和约束计算
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 6, 7, 8)
  - **Blocks**: Tasks 9, 10, 12
  - **Blocked By**: Tasks 1, 4

  **References**:

  **Pattern References**:
  - `crates/card-tui/src/ui.rs:11-102` — 当前 render_game_board 实现（纯垂直Layout，7个Constraint），需要完全重写为左右分栏
  - `crates/card-tui/src/app.rs:272-331` — 当前 render_game_screen 的三段式布局（标题+板面+提示），保持外层结构但重写中间板面部分
  - `crates/card-tui/src/ui.rs:112-135` — render_public_row/render_private_row/render_hand — 当前简单拼接显示，需重写为水平 Layout 的独立卡槽

  **API/Type References**:
  - `crates/card-client/src/api.rs:98-112` — VisibleGameState 结构（my_state, opponent_public, phase, turn_number, chain_size, is_my_turn）— 渲染的数据源
  - `crates/card-core/src/state/mod.rs:86-93` — PlayerZones（front[5], back[5], cost_zone, hand）— 场地区域尺寸

  **External References**:
  - ratatui Layout 文档：Constraint::Percentage, Constraint::Length, Direction::Horizontal

  **WHY Each Reference Matters**:
  - 现有布局代码是重写的基础，需要理解当前结构才能正确迁移
  - VisibleGameState 决定了渲染函数的输入数据形状

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 布局正确 — 左右分栏可见
    Tool: interactive_bash (tmux)
    Preconditions: Task 1, 4, 5 完成
    Steps:
      1. tmux new-session -d -s layout-test -x 120 -y 40
      2. tmux send-keys -t layout-test "cargo run -p card-tui" Enter
      3. 等待 3 秒显示主菜单
      4. 按 Enter 选择"本地对战"并选择卡组
      5. 等待进入游戏界面
      6. tmux capture-pane -t layout-test -p > evidence
      7. 检查输出包含左右两个区域（Block 有不同的 title）
    Expected Result: 左侧显示场地区域（对手后场/前场/己方前场/后场/费用区/手卡），右侧显示信息面板
    Failure Indicators: 纯垂直布局未改变，或渲染错乱
    Evidence: .sisyphus/evidence/task-5-layout-check.txt

  Scenario: 前后场横排验证
    Tool: interactive_bash (tmux)
    Preconditions: 游戏界面已显示
    Steps:
      1. 在上述 tmux 会话中截取场地区域
      2. 检查前场/后场的5个槽位在同一行水平排列
      3. 检查费用区的卡片竖着排列
    Expected Result: 前场后场各5个 [---] 横排，费用区竖排
    Failure Indicators: 槽位不在同一行或费用区横排
    Evidence: .sisyphus/evidence/task-5-field-layout.txt

  Scenario: HP星/RP圈显示
    Tool: interactive_bash (tmux)
    Preconditions: 游戏界面已显示
    Steps:
      1. 截取界面
      2. 检查输出包含 ★ 或 ☆ 字符（HP）
      3. 检查输出包含 ● 或 ○ 字符（RP）
    Expected Result: HP 显示为星星，RP 显示为圆圈
    Failure Indicators: 仍然是纯数字显示
    Evidence: .sisyphus/evidence/task-5-hp-rp-visual.txt
  ```

  **Commit**: YES
  - Message: `feat(tui): implement left-right split layout with field rendering`
  - Files: `crates/card-tui/src/ui/layout.rs`, `crates/card-tui/src/ui/field.rs`
  - Pre-commit: `cargo build -p card-tui`

- [x] 6. 右侧信息面板（卡片信息+效果文本+状态）

  **What to do**:
  - 实现 `card-tui/src/ui/info_panel.rs`：
  - `pub fn render_info_panel(frame, area, state, cursor, registry, card_defs)` — 右侧信息面板
  - 面板内容根据光标位置动态变化：
    - **有卡时**：显示卡片详细信息
      - 卡片名称（大字/高亮）
      - ID
      - 类型（人物/策略/物品/传奇）+ 子类型
      - 属性（理性/神性/灵性）
      - 范畴（数学/科学/文艺/哲学/神秘）
      - 费用
      - 攻击力（人物卡）
      - 效果文本（使用 Task 2 的 effect_to_chinese）— 每个效果一段
    - **无卡时**：显示区域信息（"己方前场 - 槽位 3 (空)"）
    - **光标未激活时**：显示当前状态概览（回合数、阶段、连锁数）
  - 需要访问 CardRegistry/CardDefinition 来获取卡片名称和效果
    - 在 App struct 中添加 `card_registry: Option<CardRegistryImpl>` 字段
    - 或在 App struct 中添加 `card_name_map: HashMap<CardId, String>` 缓存
    - 在游戏启动时（`start_game_with_decks`）加载并存储
  - 效果文本显示：调用 `card_effects_text()` 从 CardDefinition.effects 获取中文效果列表
  - 对于对手的卡（CardPublicInfo），显示有限信息：类型+攻击力，效果不可见

  **Must NOT do**:
  - 不修改 CardDefinition 或 VisibleGameState 结构
  - 不在网络协议中传输效果详情

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: 需要整合多个数据源（cursor位置、CardRegistry、效果文本），逻辑中等复杂度
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 7, 8)
  - **Blocks**: Task 12
  - **Blocked By**: Tasks 1, 2

  **References**:

  **Pattern References**:
  - `crates/card-tui/src/app.rs:638-775` — 现有 render_deck_editor 的右侧信息展示模式（ID/名称/类型/属性/范畴/费用/攻击/效果数量）— 可参考但需增强
  - `crates/card-tui/src/app.rs:1147-1171` — describe_instance 方法，展示了如何在各区域查找卡片

  **API/Type References**:
  - `crates/card-core/src/types/card_definition.rs:6-18` — CardDefinition 完整字段（name, card_type, property, category, cost, attack, effects）
  - `crates/card-core/src/state/mod.rs:286-320` — CardRegistryImpl 和 get() 方法
  - Task 2 产出的 `card-core/src/effect/text.rs` — card_effects_text() 函数

  **WHY Each Reference Matters**:
  - render_deck_editor 已有类似的卡片信息展示模式，可复用思路
  - CardRegistryImpl.get() 是通过 CardId 获取完整定义的唯一途径

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 卡片信息显示 — 有卡位置
    Tool: interactive_bash (tmux)
    Preconditions: 游戏进行中，场上有卡
    Steps:
      1. 激活场地光标（按 Tab 或指定键进入浏览模式）
      2. 移动光标到有卡的前场槽位
      3. 截取右侧面板内容
      4. 检查包含：卡片名称、ID、类型、属性、费用
    Expected Result: 右侧面板显示选中卡片的完整信息
    Failure Indicators: 面板为空或显示错误信息
    Evidence: .sisyphus/evidence/task-6-card-info.txt

  Scenario: 效果文本显示
    Tool: interactive_bash (tmux)
    Preconditions: 光标在有效果的卡片上
    Steps:
      1. 截取右侧面板
      2. 检查效果部分显示中文自然语言（非 JSON/Debug 格式）
    Expected Result: 效果显示为可读中文，如"登场时，自己恢复一点生命值。"
    Failure Indicators: 显示 {:?} 格式或 "效果: 1 个" 这样的计数
    Evidence: .sisyphus/evidence/task-6-effect-text.txt
  ```

  **Commit**: YES (groups with Tasks 7, 8)
  - Message: `feat(tui): add info panel, hand display, and colored log`
  - Files: `crates/card-tui/src/ui/info_panel.rs`, `crates/card-tui/src/app.rs`(添加 registry 字段)
  - Pre-commit: `cargo build -p card-tui`

- [x] 7. 手卡区域渲染增强

  **What to do**:
  - 重写 `card-tui/src/ui/hand.rs`：
  - 己方手卡渲染：
    - 每张卡显示为 `[S001-C-001 训练木桩]` 格式（ID + 名称）
    - 需要通过 CardRegistry 查找卡片名称
    - 如果手卡过多（>10张），换行显示
    - 使用 Wrap 保证不截断
  - 对手手卡渲染：
    - 显示为 `[?] [?] [?] ...` 或 `[?] × N` 格式
    - 颜色为灰色/暗色
  - 对手信息在场地上方区域已有位置（Task 5布局中对手区域）
  - 在手卡区域显示手卡数量统计 `(M/20)`

  **Must NOT do**:
  - 不修改手卡数据结构
  - 不泄露对手手卡信息

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 简单的渲染格式变更，逻辑简单
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6, 8)
  - **Blocks**: Task 12
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `crates/card-tui/src/ui.rs:126-135` — 当前 render_hand 实现（仅显示 compact ID），需增强为 ID+名称
  - `crates/card-tui/src/app.rs:708-728` — deck_editor 中卡片显示格式（ID+名称+费用）的参考

  **API/Type References**:
  - `crates/card-core/src/state/mod.rs:54-67` — CardInstance（definition_id: CardId）— 手卡的数据来源
  - `crates/card-client/src/api.rs:43-44` — OpponentView.hand_count — 对手手卡数量

  **WHY Each Reference Matters**:
  - render_hand 是被替换的函数
  - CardInstance.definition_id 是查找卡片名称的键

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 己方手卡显示名称
    Tool: interactive_bash (tmux)
    Preconditions: 游戏进行中，己方有手卡
    Steps:
      1. 截取手卡区域
      2. 检查手卡显示格式包含卡片名称（非仅ID编号）
    Expected Result: 手卡显示如 "[S000-C-001 训练木桩]" 包含中文名称
    Failure Indicators: 仅显示 "[C001]" 格式
    Evidence: .sisyphus/evidence/task-7-hand-display.txt

  Scenario: 对手手卡显示背面
    Tool: interactive_bash (tmux)
    Preconditions: 对手有手卡
    Steps:
      1. 截取对手手卡区域
      2. 检查仅显示 [?] 或背面标识
    Expected Result: 不泄露对手手卡名称/ID
    Failure Indicators: 显示了对手手卡的具体信息
    Evidence: .sisyphus/evidence/task-7-opponent-hand.txt
  ```

  **Commit**: YES (groups with Tasks 6, 8)
  - Message: `feat(tui): add info panel, hand display, and colored log`
  - Files: `crates/card-tui/src/ui/hand.rs`
  - Pre-commit: `cargo build -p card-tui`

- [x] 8. 彩色可滚动日志区域

  **What to do**:
  - 重写 `card-tui/src/ui/log.rs`：
  - 实现 `LogState` struct：
    - `entries: Vec<LogEntry>` — 日志条目
    - `scroll_offset: usize` — 滚动偏移
    - `max_visible: usize` — 可见行数
  - 实现 `LogEntry` struct：
    - `message: String`
    - `source: LogSource` — 枚举：`Self_`(己方), `Opponent`(对手), `System`(系统)
    - `timestamp: Option<String>` — 可选时间戳
  - 渲染逻辑：
    - 己方日志：绿色
    - 对手日志：红色
    - 系统日志：灰色/白色
    - 日志前缀：`[己] ` / `[敌] ` / `[系] `
  - 滚动功能：
    - ratatui 的 Scrollbar widget 渲染滚动条
    - 日志区域有焦点时，上下键滚动
    - 默认自动滚动到底部（最新日志）
    - 手动滚动后停止自动滚动，直到滚到底部
  - 在 App 中将 `game_events: Vec<String>` 改为 `game_log: LogState`
  - 修改 `drain_game_events()` 方法，将事件转换为 LogEntry（根据事件内容判断 source）

  **Must NOT do**:
  - 不修改事件产生端（GameEngine/UiEvent）
  - 不修改网络协议

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: 需要状态管理（滚动偏移）+ 渲染逻辑 + 与现有事件系统集成
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6, 7)
  - **Blocks**: Task 12
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `crates/card-tui/src/ui.rs:137-149` — 当前 render_event_log（仅显示最后4条，单色，无滚动）
  - `crates/card-tui/src/app.rs:1269-1304` — drain_game_events 中 UiEvent→日志字符串的映射逻辑

  **API/Type References**:
  - `crates/card-tui/src/app.rs:43-54` — UiEvent 枚举（ActionPrompt, RecoveryPrompt, StateUpdate, Log, EnterOnlineBattle）— 需要区分己方/对手/系统

  **External References**:
  - ratatui Scrollbar widget API — 用于渲染滚动条

  **WHY Each Reference Matters**:
  - drain_game_events 是日志条目的来源，需要修改以支持 LogEntry 格式
  - UiEvent 的类型决定了日志着色规则

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 日志颜色区分
    Tool: interactive_bash (tmux)
    Preconditions: 游戏进行了几个回合
    Steps:
      1. 截取日志区域
      2. 检查日志包含 [己] 和 [敌] 或 [系] 前缀
    Expected Result: 不同来源的日志有不同前缀标识
    Failure Indicators: 所有日志格式相同
    Evidence: .sisyphus/evidence/task-8-log-colors.txt

  Scenario: 日志滚动
    Tool: interactive_bash (tmux)
    Preconditions: 日志区域有焦点（通过快捷键聚焦）
    Steps:
      1. 游戏中产生超过可见行数的日志
      2. 按上箭头滚动查看历史日志
      3. 按下箭头滚回底部
    Expected Result: 可以查看历史日志，滚动顺畅
    Failure Indicators: 无法滚动或日志丢失
    Evidence: .sisyphus/evidence/task-8-log-scroll.txt
  ```

  **Commit**: YES (groups with Tasks 6, 7)
  - Message: `feat(tui): add info panel, hand display, and colored log`
  - Files: `crates/card-tui/src/ui/log.rs`, `crates/card-tui/src/app.rs`(LogState 替换 game_events)
  - Pre-commit: `cargo build -p card-tui`

- [x] 9. 场地光标导航+高亮+快捷键

  **What to do**:
  - 将 Task 3 的 `FieldCursor` 集成到 App 和渲染系统：
  - 在 App struct 添加 `field_cursor: FieldCursor` 字段
  - 光标激活/退出：
    - 在无弹窗时按 `Tab` 键进入场地浏览模式（cursor.active = true）
    - 再按 `Tab` 或 `Esc` 退出浏览模式
    - 浏览模式下方向键移动光标
  - 渲染高亮：
    - 修改 field.rs 的渲染逻辑，接收 `&FieldCursor` 参数
    - 当 cursor 指向某个槽位时，该槽位使用高亮样式（黄色背景或反色 `Style::default().bg(Color::Yellow).fg(Color::Black)`）
    - 空槽位也可选中但用虚线高亮
  - 快捷键映射（场地浏览模式下）：
    - `1-5` — 快速跳到己方前场槽位 1-5
    - `Shift+1-5` (或 `!@#$%`) — 快速跳到己方后场槽位 1-5
    - `h` — 跳到己方手卡区
    - `c` — 跳到己方费用区
    - `g` — 跳到己方墓地区
    - `f` — 跳到对手前场
    - `b` — 跳到对手后场
    - `上下左右` — 在区域间/内移动
  - 浏览模式下选中卡片时，右侧信息面板自动更新（与 Task 6 联动）
  - 在跳过（Pass）之前的操作面板中添加"浏览场地"选项

  **Must NOT do**:
  - 不修改游戏引擎的 PhaseClient 接口
  - 不改变操作提交逻辑

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 需要整合光标状态、渲染系统和输入处理三个维度
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 10, 11)
  - **Blocks**: Task 12
  - **Blocked By**: Tasks 3, 5

  **References**:

  **Pattern References**:
  - Task 3 产出的 `crates/card-tui/src/cursor.rs` — FieldCursor 和 FieldZone 定义
  - Task 5 产出的 `crates/card-tui/src/ui/field.rs` — 场地渲染函数，需要添加 cursor 参数
  - `crates/card-tui/src/app.rs:333-356` — 现有 render_action_overlay（弹窗中的选项渲染模式，可参考高亮样式）

  **API/Type References**:
  - `crates/card-tui/src/input.rs:1-61` — 现有输入映射模式，需要扩展
  - `crates/card-tui/src/app.rs:392-532` — handle_key 方法，需要添加浏览模式分支

  **WHY Each Reference Matters**:
  - cursor.rs 是导航状态的数据层
  - field.rs 是接收光标位置进行高亮渲染的渲染层
  - handle_key 是接收按键并更新光标的输入层

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 光标导航 — 在场地区域间移动
    Tool: interactive_bash (tmux)
    Preconditions: 游戏进行中
    Steps:
      1. 按 Tab 进入场地浏览模式
      2. 按方向键在各区域间移动
      3. 截取每次移动后的界面，确认高亮位置变化
      4. 按 Tab 退出浏览模式
    Expected Result: 光标可见并在区域间移动，高亮正确跟随
    Failure Indicators: 无高亮显示或光标不动
    Evidence: .sisyphus/evidence/task-9-cursor-navigation.txt

  Scenario: 快捷键跳转
    Tool: interactive_bash (tmux)
    Preconditions: 场地浏览模式激活
    Steps:
      1. 按 "1" — 光标应跳到己方前场槽位1
      2. 按 "h" — 光标应跳到手卡区
      3. 按 "c" — 光标应跳到费用区
      4. 按 "f" — 光标应跳到对手前场
    Expected Result: 每次按键后高亮正确跳转到目标区域
    Failure Indicators: 按键无响应或跳转位置错误
    Evidence: .sisyphus/evidence/task-9-quick-keys.txt

  Scenario: 选中卡片信息联动
    Tool: interactive_bash (tmux)
    Preconditions: 场地上有卡
    Steps:
      1. Tab 进入浏览模式
      2. 移动光标到有卡的位置
      3. 检查右侧信息面板更新显示该卡信息
    Expected Result: 信息面板实时显示光标所指卡片的详情
    Failure Indicators: 面板不更新或显示上一张卡的信息
    Evidence: .sisyphus/evidence/task-9-info-sync.txt
  ```

  **Commit**: YES
  - Message: `feat(tui): add field cursor navigation with highlighting and quick keys`
  - Files: `crates/card-tui/src/app.rs`, `crates/card-tui/src/ui/field.rs`, `crates/card-tui/src/input.rs`
  - Pre-commit: `cargo build -p card-tui`

- [x] 10. 半透明弹窗系统（可最小化/切换）

  **What to do**:
  - 重写 `card-tui/src/ui/overlay.rs`：
  - 实现 `OverlayState` struct：
    ```
    struct OverlayState {
        visible: bool,       // 弹窗是否显示
        minimized: bool,     // 弹窗是否最小化
        overlay_type: OverlayType,  // 当前弹窗类型
    }
    enum OverlayType {
        ActionSelect,   // 操作选择
        RecoverySelect, // 回收选择
        None,
    }
    ```
  - 半透明叠加效果实现：
    - **不使用 Clear**（Clear 会完全覆盖底层内容）
    - 渲染弹窗时，先渲染场地（底层），然后渲染弹窗区域
    - 弹窗使用 `Block` 带边框 + 半透明模拟：弹窗区域内的背景使用深色（如 `Color::Rgb(20, 20, 40)`）
    - 弹窗外的场地区域保持完全可见
    - 弹窗占中间 60%×50% 区域
  - 最小化功能：
    - 弹窗显示时按 `m` 键最小化
    - 最小化后弹窗缩为底部一行提示：`[操作选择] 按 m 恢复`
    - 最小化时可以使用场地光标浏览（与 Task 9 联动）
    - 再按 `m` 恢复弹窗
  - 切换功能：
    - 如果有多个待处理的弹窗（虽然当前不太可能），Tab 可切换
  - 修改 App 中 `render_action_overlay` 和 `render_recovery_overlay` 使用新的 overlay 系统
  - 修改 handle_key 支持 `m` 键最小化/恢复

  **Must NOT do**:
  - 不修改弹窗中的选择逻辑（PhaseAction 提交不变）
  - 不修改 UiEvent 枚举

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 叠加渲染是 TUI 中较复杂的模式，需要处理渲染顺序和状态管理
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 9, 11)
  - **Blocks**: Task 12
  - **Blocked By**: Task 5

  **References**:

  **Pattern References**:
  - `crates/card-tui/src/app.rs:333-390` — 现有 render_action_overlay 和 render_recovery_overlay（用 Clear + centered_rect），需要替换
  - `crates/card-tui/src/app.rs:1599-1617` — centered_rect 函数，可保留或改进

  **API/Type References**:
  - `crates/card-tui/src/app.rs:1011-1076` — handle_action_key 和 handle_recovery_key — 弹窗内的输入处理，需要添加最小化支持

  **WHY Each Reference Matters**:
  - 现有弹窗代码是替换目标
  - 输入处理需要新增最小化/恢复键

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 弹窗半透明 — 场地可见
    Tool: interactive_bash (tmux)
    Preconditions: 游戏进行中，操作提示弹出
    Steps:
      1. 等待操作选择弹窗出现
      2. 截取完整界面
      3. 检查弹窗外的场地区域仍可见（不是完全空白）
    Expected Result: 弹窗覆盖中央区域，但弹窗外的场地内容可见
    Failure Indicators: 整个场地被覆盖为空白（Clear 行为）
    Evidence: .sisyphus/evidence/task-10-overlay-transparency.txt

  Scenario: 弹窗最小化/恢复
    Tool: interactive_bash (tmux)
    Preconditions: 操作选择弹窗显示中
    Steps:
      1. 按 m 最小化弹窗
      2. 截取界面 — 弹窗应缩为一行提示
      3. 使用方向键浏览场地（光标应可用）
      4. 按 m 恢复弹窗
      5. 截取界面 — 弹窗应恢复完整显示
    Expected Result: 最小化后可浏览场地，恢复后弹窗完整
    Failure Indicators: 最小化无效或恢复后弹窗内容丢失
    Evidence: .sisyphus/evidence/task-10-minimize-restore.txt
  ```

  **Commit**: YES
  - Message: `feat(tui): add semi-transparent overlay system with minimize support`
  - Files: `crates/card-tui/src/ui/overlay.rs`, `crates/card-tui/src/app.rs`
  - Pre-commit: `cargo build -p card-tui`

- [x] 11. 输入系统重构+快捷键映射

  **What to do**:
  - 重构 `card-tui/src/input.rs`：
  - 定义统一的 `InputMode` 枚举：
    ```
    enum InputMode {
        Menu,            // 主菜单/选择列表
        FieldBrowse,     // 场地浏览（光标）
        ActionSelect,    // 操作选择弹窗
        RecoverySelect,  // 回收选择弹窗
        LogScroll,       // 日志滚动
        DeckEditor,      // 卡组编辑
    }
    ```
  - 定义统一的 `InputAction` 枚举（替换现有的 ActionInput/RecoveryInput）：
    ```
    enum InputAction {
        MoveUp, MoveDown, MoveLeft, MoveRight,
        Confirm, Escape, Toggle,
        QuickJump(FieldZone),     // 快捷跳转
        MinimizeOverlay,          // 最小化弹窗 (m)
        ToggleFieldBrowse,        // 切换浏览模式 (Tab)
        FocusLog,                 // 聚焦日志 (l)
        Quit,                     // 退出 (q)
        Noop,
    }
    ```
  - 实现 `pub fn map_key(mode: InputMode, key: KeyEvent) -> InputAction` — 统一按键映射
  - 快捷键总览（field browse 模式）：
    - `1-5` → QuickJump(MyFront(0-4))
    - `Shift+1-5` → QuickJump(MyBack(0-4))
    - `h` → QuickJump(MyHand(0))
    - `c` → QuickJump(MyCostZone(0))
    - `f` → QuickJump(OpponentFront(0))
    - `b` → QuickJump(OpponentBack(0))
    - `m` → MinimizeOverlay
    - `Tab` → ToggleFieldBrowse
    - `l` → FocusLog
    - `Esc` → 退出当前模式
  - 修改 App.handle_key 使用新的 map_key 统一路由

  **Must NOT do**:
  - 不修改操作提交逻辑
  - 不修改 PhaseClient 接口

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: 需要统一多种输入模式，确保快捷键不冲突
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 9, 10)
  - **Blocks**: Task 12
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `crates/card-tui/src/input.rs:1-61` — 现有 ActionInput/RecoveryInput 和 map 函数，需要统一替换
  - `crates/card-tui/src/app.rs:392-532` — handle_key 中的多层 match，需要简化为 map_key 路由

  **WHY Each Reference Matters**:
  - 现有输入系统是替换目标
  - handle_key 是集成点

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 编译通过
    Tool: Bash
    Preconditions: 输入重构完成
    Steps:
      1. cargo build -p card-tui
    Expected Result: 编译成功
    Failure Indicators: 编译错误
    Evidence: .sisyphus/evidence/task-11-compile.txt

  Scenario: 快捷键不冲突
    Tool: Bash
    Preconditions: 输入重构完成
    Steps:
      1. 检查每个 InputMode 下的按键映射无重复
      2. 特别检查 'q' 在游戏内不直接退出（仅在主菜单退出）
    Expected Result: 无按键冲突，游戏内 q 不退出
    Failure Indicators: 同一按键在同一模式下映射到多个动作
    Evidence: .sisyphus/evidence/task-11-key-conflicts.txt
  ```

  **Commit**: YES
  - Message: `refactor(tui): unify input system with InputMode and quick-key mappings`
  - Files: `crates/card-tui/src/input.rs`, `crates/card-tui/src/app.rs`
  - Pre-commit: `cargo build -p card-tui`

- [x] 12. 全流程集成联调

  **What to do**:
  - 确保所有 Wave 1-3 的模块正确组装：
  - 检查 `card-tui/src/main.rs` 的模块声明包含所有新模块
  - 检查 `card-tui/Cargo.toml` 依赖正确（card-core 需要 effect::text 模块的访问）
  - 运行完整本地对战流程验证：
    1. 启动 TUI → 主菜单正常
    2. 进入卡组选择 → 选择双方卡组
    3. 进入游戏 → 左右分栏布局正常
    4. 抽卡阶段 → 手卡增加一张
    5. 回收阶段 → 回收弹窗正常
    6. 主要阶段 → 操作弹窗正常，可出牌
    7. Tab 浏览场地 → 光标移动+高亮+信息面板联动
    8. 弹窗最小化 → 浏览场地后恢复
    9. 战斗阶段 → 攻击选择正常
    10. 日志区域 → 颜色区分+可滚动
    11. 对局结束 → 返回主菜单
  - 修复发现的集成问题：
    - 渲染函数参数不匹配
    - 状态传递遗漏
    - 生命周期/借用问题
    - 布局计算溢出

  **Must NOT do**:
  - 不新增功能
  - 不修改游戏逻辑

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 集成联调需要运行真实游戏并修复交叉模块问题
  - **Skills**: [`systematic-debugging`]
    - `systematic-debugging`: 联调中发现问题时需要系统化定位和修复

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (Sequential)
  - **Blocks**: Task 13
  - **Blocked By**: Tasks 5, 6, 7, 8, 9, 10, 11

  **References**:

  **所有前置 Task 的产出文件** — 需要整体审查

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 完整游戏流程 — 从启动到结束
    Tool: interactive_bash (tmux)
    Preconditions: 所有前置任务完成
    Steps:
      1. tmux new-session -d -s full-test -x 120 -y 40
      2. 启动 cargo run -p card-tui
      3. 选择"本地对战" → 选择卡组
      4. 游戏开始后：
         a. 等待操作选择弹窗
         b. 选择出牌/攻击/跳过
         c. Tab 进入场地浏览，移动光标查看卡片
         d. 按 m 最小化弹窗，浏览场地
         e. 按 m 恢复
         f. 选择操作
      5. 重复多个回合直到游戏结束
      6. 确认返回主菜单
    Expected Result: 完整游戏流程无 panic，所有界面正常显示
    Failure Indicators: panic, 死循环, 界面错乱, 无法操作
    Evidence: .sisyphus/evidence/task-12-full-game.txt

  Scenario: 编译零警告
    Tool: Bash
    Steps:
      1. cargo clippy -p card-tui -- -D warnings
      2. cargo clippy -p card-core -- -D warnings
    Expected Result: 零 warning
    Evidence: .sisyphus/evidence/task-12-clippy.txt
  ```

  **Commit**: YES
  - Message: `fix(tui): integration fixes for complete game flow`
  - Files: 视修复范围而定
  - Pre-commit: `cargo build -p card-tui && cargo test -p card-core`

- [x] 13. 编译检查+clippy清理

  **What to do**:
  - 运行全面检查：
    - `cargo build --workspace` — 全 workspace 编译
    - `cargo clippy --workspace -- -D warnings` — 全 workspace clippy
    - `cargo test -p card-core` — 核心测试
    - `cargo test -p card-tui` — TUI 测试（如果有）
  - 修复所有 clippy warning：
    - unused imports
    - unused variables
    - needless borrows
    - redundant clones
  - 确保无 dead_code warning（允许 #[allow(dead_code)] 仅用于占位的未来扩展点）
  - 确认 `cargo run -p card-tui` 正常启动

  **Must NOT do**:
  - 不修改功能逻辑
  - 不添加新功能

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 纯机械式清理
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (after Task 12)
  - **Blocks**: F1-F4
  - **Blocked By**: Task 12

  **References**:
  - 所有修改过的文件

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 全面编译检查
    Tool: Bash
    Steps:
      1. cargo build --workspace
      2. cargo clippy --workspace -- -D warnings
      3. cargo test -p card-core
    Expected Result: 全部通过，零 error，零 warning
    Evidence: .sisyphus/evidence/task-13-final-check.txt
  ```

  **Commit**: YES
  - Message: `chore(tui): clippy cleanup and compilation fixes`
  - Files: 视修复范围
  - Pre-commit: `cargo clippy --workspace -- -D warnings`

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build -p card-tui`, `cargo clippy -p card-tui`, `cargo test -p card-core`. Review all changed files for: `unwrap()` in render paths, `panic!` in UI code, unused imports. Check AI slop: excessive comments, over-abstraction, generic names.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Launch `cargo run -p card-tui` in tmux. Execute EVERY QA scenario from EVERY task — follow exact steps, capture evidence. Test cross-task integration: navigate field, view card info, start game, play through. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff. Verify 1:1 — everything in spec was built, nothing beyond spec was built. Check "Must NOT do" compliance. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **Wave 1**: `refactor(tui): split app.rs into modular structure` — 拆分文件
- **Wave 1**: `feat(core): add Effect-to-Chinese-text conversion module` — 效果文本
- **Wave 2**: `feat(tui): implement left-right split layout with field rendering` — 主布局
- **Wave 2**: `feat(tui): add info panel, hand display, and colored log` — 右面板+手卡+日志
- **Wave 3**: `feat(tui): add field cursor navigation and overlay system` — 光标+弹窗
- **Wave 4**: `fix(tui): integration fixes and clippy cleanup` — 集成修复

---

## Success Criteria

### Verification Commands
```bash
cargo build -p card-tui    # Expected: 编译成功，无 error
cargo clippy -p card-tui   # Expected: 无 warning
cargo test -p card-core    # Expected: 所有测试通过（含效果文本测试）
cargo run -p card-tui      # Expected: 启动后可正常进入主菜单并开始本地对战
```

### Final Checklist
- [ ] 左右分栏布局正常显示
- [ ] 场地光标可浏览所有区域（前场/后场/费用区/手卡）
- [ ] 选中卡片高亮 + 右侧显示卡片详情含效果中文文本
- [ ] HP显示星★ / RP显示圈○
- [ ] 弹窗可最小化查看场地
- [ ] 快捷键可快速跳转各区域
- [ ] 日志区分己方/对手颜色 + 可滚动
- [ ] 本地对战可完整游玩（出牌→攻击→回收→结束）
