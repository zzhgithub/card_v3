# 卡组管理系统 + 引擎修复

## TL;DR

> **快速摘要**: 实现卡组 (Deck) 持久化模块、TUI 卡组编辑器/选择器，并修复两个引擎 Bug（初始手牌抽取缺失 + 对战结束后无法返回菜单）。
>
> **交付物**:
> - `card-core/src/deck.rs` — Deck 数据模型 + DeckManager (文件 I/O)
> - `card-script/src/loader.rs` — 新增 `load_all()` / `load_card()` 公共 API
> - `desks/Example.json` — 示例卡组文件
> - `card-core/src/engine/game_engine.rs` — 修复初始手牌抽取
> - `card-tui/src/app.rs` — 修复 GameOver 返回菜单 + 新增卡组管理/编辑/选择页面
> - `card-tui/src/deck_ui.rs` — 卡组浏览器 + 编辑器渲染逻辑
>
> **预估工作量**: Medium（7个核心需求，10个实现任务）
> **并行执行**: YES — 3 waves
> **关键路径**: Task 1 → Task 5 → Task 7 → Task 8

---

## Context

### Original Request
来自 `plan/NewPlan001.md` 的 7 个需求：
1. 新增 Deck 模块，`desks/` 文件夹存储卡组
2. 卡组 JSON 格式：`{ name, cards: [CardId] }`，ID 必须在 scripts 中定义
3. 生成示例卡组 "Example"
4. **[Fix]** 游戏开始前每人抽 5 张初始手牌（`initial_hand_size` 可配）
5. TUI 卡组编辑页面：浏览、新建、编辑卡组，左右布局查看卡片信息，3 张上限提示
6. 本地对战前选择己方和 AI 的卡组
7. **[Fix]** 对战结束后返回菜单，不直接退出

### Metis 审查发现

**关键缺口（已解决）**:
- `ScriptLoader::load_script()` 是私有的 → 需新增 `load_all()` 公共方法
- 没有 `CardId → CardInstance` 的填充函数 → 需新增 `populate_deck()` 工具函数
- `handle_key` 中 `'q'` 退出是全局的（350行）→ 需修复为仅在 MainMenu 退出
- S000 只有 9 张唯一卡（最多 27 张）→ 示例卡组不满足默认 40 张最小要求

**设计决策**:
- **卡组保存不做验证**，仅在游戏启动时验证 → 自由编辑，启动时报错
- **文件夹命名**: `desks/`（遵循规格书字面意思）
- **Deck 结构体放 card-core** — 属于领域数据模型，card-server 未来也需要
- **'q' 键行为全局修复** — 仅 MainMenu 退出，其他页面 Esc 返回

---

## Work Objectives

### Core Objective
为卡牌游戏添加完整的卡组管理系统（持久化 + TUI 编辑器 + 对战前选择），同时修复两个引擎级 Bug。

### Concrete Deliverables
- `card-core/src/deck.rs` — Deck struct + DeckManager
- `card-script/src/loader.rs` — 新增 `load_all()`, `load_card()` 方法
- `desks/Example.json` — 27 张 S000 卡的示例卡组
- `card-core/src/engine/game_engine.rs` — 初始手牌抽取逻辑
- `card-tui/src/app.rs` — 5 个新 AppMode + GameOver 修复
- `card-tui/src/deck_ui.rs` — 卡组浏览/编辑器 UI 渲染

### Definition of Done
- [ ] `cargo test --workspace` 全部通过
- [ ] `cargo build --workspace` 无新错误
- [ ] 示例卡组文件 `desks/Example.json` 存在且可反序列化
- [ ] 游戏开始时每人抽取 `initial_hand_size` 张初始手牌
- [ ] GameOver 按 Enter 返回主菜单

### Must Have
- Deck JSON 持久化（保存/加载/列举）
- 示例卡组
- 初始手牌抽取修复
- GameOver 返回菜单修复
- TUI 卡组浏览器 + 编辑器
- 对战前卡组选择

### Must NOT Have (Guardrails)
- 不修改 `validate_deck()` 签名或逻辑
- 不新增 S000 Lua 卡脚本
- 不修改 `PhaseRunner::run_turn()`（初始抽牌放 GameEngine）
- 不修改 `GameSession` 或 `GameServer`（网络路径不在范围内）
- 不实现搜索/过滤/排序/撤销等高级编辑器功能
- 不创建多个 Deck 相关结构体（只用一个 `Deck` struct）
- 不实现 ELO 或卡组统计分析

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — 所有验证由代理执行。

### Test Decision
- **Infrastructure exists**: YES (cargo test 已有 229 tests)
- **Automated tests**: YES (Tests-after)
- **Framework**: cargo test

### QA Policy
每个任务包含 QA 场景，证据保存到 `.sisyphus/evidence/`。

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — 4 parallel tasks):
├── Task 1: ScriptLoader 新增 load_all()/load_card() 公共 API [quick]
├── Task 2: Deck 数据模型 + DeckManager + 示例卡组 [deep]
├── Task 3: [Fix] 初始手牌抽取 [quick]
└── Task 4: [Fix] 'q' 退出行为 + GameOver 返回菜单 [quick]

Wave 2 (TUI 页面 — depends on Wave 1):
├── Task 5: 主菜单更新 + TUI 卡组浏览器 (depends: 1, 2) [deep]
├── Task 6: TUI 卡组编辑器（左右布局） (depends: 1, 2, 5) [deep]
└── Task 7: TUI 对战前卡组选择 + 替换 build_demo_state (depends: 2, 3) [deep]

Wave 3 (集成 + 测试):
├── Task 8: 集成验证 + 补充测试 (depends: all) [deep]

Wave FINAL:
├── F1: 计划合规审计
├── F2: 代码质量审查
├── F3: 完整 QA
└── F4: 范围忠实度检查

Critical Path: Task 1 → Task 5 → Task 6
Parallel Speedup: ~60% faster than sequential
Max Concurrent: 4 (Wave 1)
```

### Dependency Matrix

| Task | Blocked By | Blocks |
|------|-----------|--------|
| 1 | — | 5, 6, 7 |
| 2 | — | 5, 6, 7 |
| 3 | — | 7, 8 |
| 4 | — | 5, 8 |
| 5 | 1, 2, 4 | 6 |
| 6 | 1, 2, 5 | 8 |
| 7 | 2, 3 | 8 |
| 8 | 5, 6, 7 | FINAL |

### Agent Dispatch Summary

- **Wave 1**: 4 tasks — T1 `quick`, T2 `deep`, T3 `quick`, T4 `quick`
- **Wave 2**: 3 tasks — T5 `deep`, T6 `deep`, T7 `deep`
- **Wave 3**: 1 task — T8 `deep`
- **FINAL**: 4 tasks — F1-F4

---

## TODOs

- [x] 1. ScriptLoader 新增 load_all() / load_card() 公共 API

  **What to do**:
  - 在 `crates/card-script/src/loader.rs` 的 `ScriptLoader` 中新增两个公共方法：
    - `pub fn load_all(&self) -> Result<CardRegistryImpl, ScriptError>` — 加载 ScriptIndex 中所有卡的 CardDefinition
    - `pub fn load_card(&self, card_id: &CardId) -> Result<CardDefinition, ScriptError>` — 加载单张卡的定义
  - 现有的 `load_script()` 是私有的，`load_all` 内部调用它遍历 index 中所有条目
  - 添加 2-3 个测试验证 load_all 和 load_card 正确返回

  **Must NOT do**:
  - 不修改现有 `load_for_game()` 的签名或行为
  - 不修改 `ScriptIndex` 结构

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4)
  - **Blocks**: Tasks 5, 6, 7
  - **Blocked By**: None

  **References**:
  - `crates/card-script/src/loader.rs:119` — 私有 `load_script()` 方法，`load_all` 需要调用它
  - `crates/card-script/src/loader.rs:80` — `ScriptIndex::iter()` 用于遍历所有条目
  - `crates/card-script/src/loader.rs:96-116` — `load_for_game()` 展示了 load_script 的调用模式
  - `crates/card-core/src/state/mod.rs:358-375` — `CardRegistryImpl` 的 `insert()` 方法

  **Acceptance Criteria**:
  - [ ] `ScriptLoader::load_all()` 返回包含所有 S000 卡定义的 CardRegistryImpl
  - [ ] `ScriptLoader::load_card(&CardId::new("S000-C-001"))` 返回对应 CardDefinition
  - [ ] `ScriptLoader::load_card()` 对不存在的 ID 返回 ScriptError
  - [ ] `cargo test -p card-script` 通过（现有 32 + 新增）

  **QA Scenarios**:
  ```
  Scenario: load_all 返回所有 S000 卡
    Tool: Bash (cargo test)
    Steps:
      1. cargo test -p card-script -- load_all
    Expected Result: test pass, registry.len() >= 9
    Evidence: .sisyphus/evidence/task-1-load-all.txt

  Scenario: load_card 对不存在的 ID 返回错误
    Tool: Bash (cargo test)
    Steps:
      1. cargo test -p card-script -- load_card_not_found
    Expected Result: test pass, returns Err(ScriptError)
    Evidence: .sisyphus/evidence/task-1-load-card-error.txt
  ```

  **Commit**: YES
  - Message: `feat(script): add ScriptLoader::load_all() and load_card() public API`
  - Files: `crates/card-script/src/loader.rs`
  - Pre-commit: `cargo test -p card-script`

- [x] 2. Deck 数据模型 + DeckManager + 示例卡组

  **What to do**:
  - 创建 `crates/card-core/src/deck.rs`：
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct Deck {
        pub name: String,
        pub cards: Vec<CardId>,
    }
    ```
  - `DeckManager` 静态方法：
    - `pub fn save(path: &Path, deck: &Deck) -> Result<()>` — `serde_json::to_string_pretty` + `fs::write`
    - `pub fn load(path: &Path) -> Result<Deck>` — `fs::read_to_string` + `serde_json::from_str`
    - `pub fn list_decks(dir: &Path) -> Result<Vec<DeckSummary>>` — 扫描 `.json` 文件返回 name + card_count
    - `pub fn delete(path: &Path) -> Result<()>`
  - `DeckSummary { name: String, card_count: usize, file_path: PathBuf }`
  - 在 `card-core/src/lib.rs` 中添加 `pub mod deck;`
  - 创建 `desks/` 目录和 `desks/Example.json`：
    ```json
    {
      "name": "Example",
      "cards": ["S000-C-001","S000-C-001","S000-C-001","S000-C-002","S000-C-002","S000-C-002",
                "S000-C-003","S000-C-003","S000-C-003","S000-S-001","S000-S-001","S000-S-001",
                "S000-S-002","S000-S-002","S000-S-002","S000-S-003","S000-S-003","S000-S-003",
                "S000-I-001","S000-I-001","S000-I-001","S000-I-002","S000-I-002","S000-I-002",
                "S000-L-001","S000-L-001","S000-L-001"]
    }
    ```
  - 编写 5+ 单元测试

  **Must NOT do**:
  - 不修改 `validate_deck()` 签名
  - 不创建多个 Deck 类结构体
  - 不添加 card-core 以外的依赖

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3, 4)
  - **Blocks**: Tasks 5, 6, 7
  - **Blocked By**: None

  **References**:
  - `crates/card-server/src/replay.rs` — JSON 文件 I/O 模式参考（`serde_json::to_string_pretty` + `fs::write`）
  - `crates/card-core/src/types/card_id.rs` — CardId 的 Serialize/Deserialize 实现
  - `crates/card-core/src/rules/mod.rs:99-157` — `validate_deck()` 现有验证逻辑
  - `scripts/S000/` — 9 个 S000 Lua 脚本，用于构建示例卡组

  **Acceptance Criteria**:
  - [ ] `Deck` struct 可 JSON 序列化/反序列化
  - [ ] `DeckManager::save()` + `DeckManager::load()` roundtrip 一致
  - [ ] `DeckManager::list_decks()` 扫描目录返回正确 DeckSummary
  - [ ] `desks/Example.json` 存在，反序列化后 name="Example", cards.len()=27
  - [ ] `cargo test -p card-core` 通过

  **QA Scenarios**:
  ```
  Scenario: Example 卡组文件可加载
    Tool: Bash
    Steps:
      1. cat desks/Example.json | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['name'], len(d['cards']))"
    Expected Result: "Example 27"
    Evidence: .sisyphus/evidence/task-2-example-deck.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add Deck data model, DeckManager, and Example deck`
  - Files: `crates/card-core/src/deck.rs`, `crates/card-core/src/lib.rs`, `desks/Example.json`
  - Pre-commit: `cargo test -p card-core`

- [x] 3. [Fix] 初始手牌抽取

  **What to do**:
  - 在 `crates/card-core/src/engine/game_engine.rs` 的 `GameEngine::run()` 方法中，主循环 `loop {` 之前添加初始手牌抽取逻辑：
    ```rust
    // Draw initial hand for both players
    let initial_hand_size = self.state.rules.initial_hand_size;
    for player_idx in 0..2 {
        for _ in 0..initial_hand_size {
            if self.state.players[player_idx].zones.deck.is_empty() {
                // Deck out during initial draw
                let loser = if player_idx == 0 { PlayerId::Player1 } else { PlayerId::Player2 };
                return GameResult {
                    winner: Some(loser.opponent()),
                    reason: GameOverReason::DeckOut,
                    final_state: self.state,
                    event_log: vec![CoreGameEvent::GameOver {
                        winner: Some(loser.opponent()),
                        reason: GameOverReason::DeckOut,
                    }],
                };
            }
            let card = self.state.players[player_idx].zones.deck.remove(0);
            let iid = card.instance_id;
            self.state.players[player_idx].zones.hand.push(card);
            event_log.push(CoreGameEvent::DrawCard {
                player: if player_idx == 0 { PlayerId::Player1 } else { PlayerId::Player2 },
                instance_id: iid,
            });
        }
    }
    ```
  - 编写 3 个测试：
    1. 正常抽取 5 张初始手牌
    2. `initial_hand_size = 0` 时不抽牌
    3. 牌库不足时触发 DeckOut

  **Must NOT do**:
  - 不修改 `PhaseRunner::run_turn()` — 初始抽牌只在 GameEngine 中
  - 不修改 `GameRules::default()` 的 initial_hand_size 值

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 4)
  - **Blocks**: Tasks 7, 8
  - **Blocked By**: None

  **References**:
  - `crates/card-core/src/engine/game_engine.rs:40-82` — `run()` 方法主循环，初始抽牌放在第 42-43 行之间
  - `crates/card-core/src/engine/phase.rs:68-91` — Draw 阶段的单卡抽取逻辑，参考其抽卡 + 事件格式
  - `crates/card-core/src/state/events.rs` — `CoreGameEvent::DrawCard` 和 `GameOverReason::DeckOut`
  - `crates/card-core/src/rules/mod.rs:52` — `initial_hand_size: usize` 字段（已存在，值为 5）

  **Acceptance Criteria**:
  - [ ] 游戏开始后双方手牌数等于 `initial_hand_size`
  - [ ] `initial_hand_size = 0` 时双方初始手牌为空
  - [ ] 牌库少于 `initial_hand_size` 时返回 DeckOut
  - [ ] `cargo test -p card-core` 通过

  **Commit**: YES
  - Message: `fix(core): draw initial hand cards before first turn in GameEngine`
  - Files: `crates/card-core/src/engine/game_engine.rs`
  - Pre-commit: `cargo test -p card-core`

- [x] 4. [Fix] 'q' 退出行为修复 + GameOver 返回菜单

  **What to do**:
  - 修改 `crates/card-tui/src/app.rs` 中 `handle_key()` 方法（第 349-386 行）：
    - 移除全局 `'q'` 退出（第 350-353 行）
    - 仅在 `AppMode::MainMenu` 时 `'q'` 退出
    - `AppMode::GameOver` 时：Enter/Esc/q → 调用 `reset_to_main_menu()`
    - 其他模式的 Esc 键 → 返回上一级（后续任务会用到）
  - 新增 `App::reset_to_main_menu()` 方法：
    ```rust
    fn reset_to_main_menu(&mut self) {
        self.mode = AppMode::MainMenu;
        self.selected_index = 0;
        self.game_events.clear();
        self.visible_state = None;
        self.pending_actions = None;
        self.pending_recovery = None;
        self.ui_event_rx = None;
        self.action_tx = None;
        self.recovery_tx = None;
        self.game_result_rx = None;
    }
    ```
  - 更新 `render_game_screen` 中 GameOver 状态的提示文字：显示 "按 Enter 返回主菜单"

  **Must NOT do**:
  - 不修改游戏逻辑
  - 不影响 InGame 时的操作输入

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3)
  - **Blocks**: Tasks 5, 8
  - **Blocked By**: None

  **References**:
  - `crates/card-tui/src/app.rs:349-386` — 当前 `handle_key()` 方法，第 350 行全局 'q' 退出
  - `crates/card-tui/src/app.rs:234-288` — `render_game_screen()` 中 GameOver 显示逻辑
  - `crates/card-tui/src/app.rs:109-145` — App 结构体字段（需在 reset 时清除）

  **Acceptance Criteria**:
  - [ ] MainMenu 按 'q' 退出程序
  - [ ] GameOver 按 Enter 返回 MainMenu
  - [ ] GameOver 按 Esc 返回 MainMenu
  - [ ] InGame 按 'q' 不退出（保持当前行为）
  - [ ] `cargo build -p card-tui` 通过

  **Commit**: YES
  - Message: `fix(tui): fix quit behavior and add GameOver return-to-menu`
  - Files: `crates/card-tui/src/app.rs`
  - Pre-commit: `cargo build -p card-tui`

- [x] 5. 主菜单更新 + TUI 卡组浏览器

  **What to do**:
  - 在 `AppMode` 枚举中新增 `DeckBrowser`
  - 主菜单改为 4 项：`["本地对战 (vs AI)", "联网对战", "卡组管理", "退出"]`
  - 更新 `handle_key` 主菜单的 index 映射（index 2 = 卡组管理, index 3 = 退出）
  - 创建 `crates/card-tui/src/deck_ui.rs`，实现卡组浏览器渲染：
    - 显示 `desks/` 下所有卡组列表（使用 `DeckManager::list_decks()`）
    - 每项显示：卡组名称 + 卡片数量
    - 底部操作栏：`↑/↓ 选择 | Enter 编辑 | N 新建 | D 删除 | Esc 返回`
    - 选中高亮
  - App 新增字段：`deck_list: Vec<DeckSummary>`, `deck_selected: usize`
  - 进入 DeckBrowser 时调用 `DeckManager::list_decks("desks/")` 填充 deck_list
  - 在 `main.rs` 中添加 `mod deck_ui;`

  **Must NOT do**:
  - 不实现卡组编辑器逻辑（Task 6）
  - 不实现卡组选择器（Task 7）

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 7)
  - **Blocks**: Task 6
  - **Blocked By**: Tasks 1, 2, 4

  **References**:
  - `crates/card-tui/src/app.rs:177-231` — `render_main_menu()` 当前菜单渲染逻辑
  - `crates/card-tui/src/app.rs:365-384` — `handle_key` 主菜单分支
  - `crates/card-tui/src/ui.rs` — 现有 UI 渲染模式参考
  - `crates/card-core/src/deck.rs` — DeckManager::list_decks() API（Task 2 创建）

  **Acceptance Criteria**:
  - [ ] 主菜单显示 4 个选项（本地对战/联网对战/卡组管理/退出）
  - [ ] 选择"卡组管理"进入 DeckBrowser 页面
  - [ ] DeckBrowser 列出 desks/ 下所有 JSON 卡组
  - [ ] Esc 返回主菜单
  - [ ] `cargo build -p card-tui` 通过

  **Commit**: YES
  - Message: `feat(tui): add deck browser page with card listing`
  - Files: `crates/card-tui/src/app.rs`, `crates/card-tui/src/deck_ui.rs`, `crates/card-tui/src/main.rs`
  - Pre-commit: `cargo build -p card-tui`

- [x] 6. TUI 卡组编辑器（左右布局 + 添加/移除卡片）

  **What to do**:
  - 在 `AppMode` 枚举中新增 `DeckEditor`
  - 卡组编辑器状态：
    ```rust
    editing_deck: Option<Deck>,              // 当前编辑的卡组
    editing_deck_path: Option<PathBuf>,       // 保存路径（None = 新卡组）
    all_card_defs: Vec<CardDefinition>,       // 所有可用卡定义（从 ScriptLoader::load_all 获取）
    editor_panel: EditorPanel,                // Left | Right
    editor_left_index: usize,                 // 左侧卡组列表游标
    editor_right_index: usize,               // 右侧可用卡列表游标
    editor_show_info: bool,                  // 是否显示卡片详情面板
    ```
  - **左右布局**：
    ```
    ┌─── 卡组: Example (27/60) ──────────┬─── 卡片信息 ──────────────┐
    │ > S000-C-001 理性学者 ×3          │ ID: S000-C-001            │
    │   S000-C-002 xxx ×3               │ 名称: 理性学者            │
    │   S000-C-003 xxx ×2               │ 类型: 人物卡              │
    │   ...                              │ 属性: 理性                │
    │                                    │ 范畴: 数学                │
    │                                    │ 费用: 3                   │
    │                                    │ 攻击: 1500                │
    │                                    │ 效果:                     │
    │                                    │   (无效果)                │
    ├─── 可用卡片 ──────────────────────┤                           │
    │   S000-C-001 理性学者 [3费] (3/3) │                           │
    │ > S000-S-001 xxx [2费] (2/3)      │                           │
    └────────────────────────────────────┴───────────────────────────┘
    操作: ↑↓移动 | Tab切换面板 | A添加 | R移除 | S保存 | Esc返回
    ```
  - 添加卡片时检查：
    - 同 ID 不超过 3 张 → 显示 `(N/3)` 提示
    - 总数不超过 `max_deck_size` (60) → 显示 `(N/60)` 提示
  - 使用 `ScriptLoader::load_all()` 获取所有可用卡定义
  - 卡片详情显示: id, name, card_type, property, category, cost, attack, effects（JSON 格式展示 effects）
  - 保存：调用 `DeckManager::save()`
  - 新建卡组时：弹出输入卡组名称

  **Must NOT do**:
  - 不实现搜索/过滤/排序功能
  - 不实现撤销功能
  - 不修改 card-core 或 card-script 代码

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential after Task 5
  - **Blocks**: Task 8
  - **Blocked By**: Tasks 1, 2, 5

  **References**:
  - `crates/card-tui/src/app.rs:290-347` — overlay 渲染模式参考
  - `crates/card-tui/src/ui.rs:1-191` — 现有 UI 渲染（Layout, Block, Paragraph）
  - `crates/card-core/src/types/card_definition.rs:6-18` — CardDefinition 字段列表
  - `crates/card-script/src/loader.rs` — `ScriptLoader::load_all()` API（Task 1 创建）
  - `crates/card-core/src/deck.rs` — `Deck`, `DeckManager::save()` API（Task 2 创建）

  **Acceptance Criteria**:
  - [ ] 从 DeckBrowser 选择卡组进入 DeckEditor
  - [ ] 左侧显示当前卡组卡片列表（合并同 ID 显示 ×N）
  - [ ] 右侧显示选中卡片的详细信息
  - [ ] 可添加卡片到卡组，显示 (N/3) 上限提示
  - [ ] 可从卡组移除卡片
  - [ ] 按 S 保存卡组到文件
  - [ ] 按 Esc 返回 DeckBrowser
  - [ ] `cargo build -p card-tui` 通过

  **Commit**: YES
  - Message: `feat(tui): add deck editor with left-right card info layout`
  - Files: `crates/card-tui/src/app.rs`, `crates/card-tui/src/deck_ui.rs`
  - Pre-commit: `cargo build -p card-tui`

- [x] 7. TUI 对战前卡组选择 + 替换 build_demo_state

  **What to do**:
  - 在 `AppMode` 枚举中新增 `DeckSelection`
  - 卡组选择器状态：
    ```rust
    selection_phase: SelectionPhase,    // SelectPlayer | SelectAI
    available_decks: Vec<DeckSummary>,
    player_deck: Option<Deck>,
    ai_deck: Option<Deck>,
    selection_index: usize,
    ```
  - 选择流程：
    1. 主菜单选择"本地对战" → 进入 DeckSelection
    2. 第一步：选择己方卡组（显示 desks/ 下所有卡组）
    3. 第二步：选择 AI 卡组（同上）
    4. 双方卡组选好后 → 加载 Lua 脚本 → 构建 GameState → 开始游戏
  - **替换 `build_demo_state()`**：
    - 使用 `ScriptIndex::scan("scripts/")` 加载脚本
    - 使用 `ScriptLoader::load_for_game()` 加载两副卡组涉及的 CardDefinition
    - 创建 `GameState::new(rules, seed)`
    - 新增工具函数 `populate_deck(state, player_idx, deck_cards, registry)`:
      ```rust
      fn populate_deck(state: &mut GameState, player_idx: usize, card_ids: &[CardId], registry: &CardRegistryImpl) {
          let mut next_id = state.next_instance_id;
          for card_id in card_ids {
              let base_attack = registry.get(card_id).and_then(|def| def.attack.map(|a| a as i32));
              state.players[player_idx].zones.deck.push(
                  CardInstance::new(InstanceId(next_id), card_id.clone(), base_attack)
              );
              next_id += 1;
          }
          state.next_instance_id = next_id;
      }
      ```
    - 注意：需确认 `GameState` 是否有 `next_instance_id` 字段。如果没有，用手动递增的计数器。
  - 卡组验证：使用 `validate_deck()` 检查（可放宽 `deck_size_range` 为 `(1, 60)` 用于测试）
  - 验证失败时显示错误提示，不开始游戏

  **Must NOT do**:
  - 不修改 `GameSession` 或 `GameServer`
  - 不修改 `validate_deck()` 的签名
  - 不删除 `build_demo_state()` 函数（可保留作为 fallback，但不再在主流程中使用）

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 5)
  - **Blocks**: Task 8
  - **Blocked By**: Tasks 2, 3

  **References**:
  - `crates/card-tui/src/app.rs:551-601` — 当前 `start_local_game()` 使用 build_demo_state
  - `crates/card-tui/src/app.rs:1007-1037` — `build_demo_state()` 硬编码 20 张卡
  - `crates/card-core/src/deck.rs` — `Deck`, `DeckManager::load()` API（Task 2 创建）
  - `crates/card-script/src/loader.rs` — `ScriptIndex::scan()`, `ScriptLoader::load_for_game()`
  - `crates/card-core/src/rules/mod.rs:99-157` — `validate_deck()` 验证
  - `crates/card-core/src/state/mod.rs:69-79` — `CardInstance::new()` 构造

  **Acceptance Criteria**:
  - [ ] 主菜单"本地对战"先进入卡组选择页面
  - [ ] 可选择己方卡组和 AI 卡组
  - [ ] 选完后使用真实卡组数据启动游戏（非 build_demo_state）
  - [ ] 卡组验证失败时显示错误信息
  - [ ] `cargo build -p card-tui` 通过

  **Commit**: YES
  - Message: `feat(tui): add pre-game deck selection and replace demo state`
  - Files: `crates/card-tui/src/app.rs`
  - Pre-commit: `cargo build -p card-tui`

- [x] 8. 集成验证 + 补充测试

  **What to do**:
  - 创建 `crates/card-core/tests/deck_tests.rs`：
    - Deck JSON 序列化/反序列化 roundtrip
    - DeckManager save/load roundtrip（使用 tempdir）
    - DeckManager list_decks 扫描
    - Example deck 文件验证
    - 初始手牌抽取集成测试
  - 运行 `cargo test --workspace` 确认全部通过
  - 运行 `cargo build --workspace` 确认全部编译

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (sequential after Wave 2)
  - **Blocks**: FINAL
  - **Blocked By**: Tasks 5, 6, 7

  **References**:
  - `crates/card-core/tests/engine_tests.rs` — 现有测试模式参考
  - `crates/card-core/src/deck.rs` — Deck, DeckManager API

  **Acceptance Criteria**:
  - [ ] `cargo test --workspace` 全部通过
  - [ ] `cargo build --workspace` 无新错误
  - [ ] 5+ 新测试覆盖 Deck 模块

  **Commit**: YES
  - Message: `test: add deck module and integration tests`
  - Files: `crates/card-core/tests/deck_tests.rs`
  - Pre-commit: `cargo test --workspace`

---

## Final Verification Wave

> 4 review agents 并行。全部 APPROVE 后提交给用户确认。

- [ ] F1. **计划合规审计** — `oracle`
- [ ] F2. **代码质量审查** — `unspecified-high`
- [ ] F3. **完整 QA** — `unspecified-high`
- [ ] F4. **范围忠实度检查** — `deep`

---

## Commit Strategy

| # | Message | Files | Pre-commit |
|---|---------|-------|------------|
| 1 | `feat(script): add ScriptLoader::load_all() and load_card() public API` | card-script/src/loader.rs | cargo test -p card-script |
| 2 | `feat(core): add Deck data model, DeckManager, and Example deck` | card-core/src/deck.rs, card-core/src/lib.rs, desks/Example.json | cargo test -p card-core |
| 3 | `fix(core): draw initial hand cards before first turn in GameEngine` | card-core/src/engine/game_engine.rs | cargo test -p card-core |
| 4 | `fix(tui): fix quit behavior and add GameOver return-to-menu` | card-tui/src/app.rs | cargo build -p card-tui |
| 5 | `feat(tui): add deck browser page with card listing` | card-tui/src/app.rs, card-tui/src/deck_ui.rs | cargo build -p card-tui |
| 6 | `feat(tui): add deck editor with left-right card info layout` | card-tui/src/app.rs, card-tui/src/deck_ui.rs | cargo build -p card-tui |
| 7 | `feat(tui): add pre-game deck selection and replace demo state` | card-tui/src/app.rs | cargo build -p card-tui |
| 8 | `test: add deck module and integration tests` | tests/ | cargo test --workspace |

---

## Success Criteria

### Verification Commands
```bash
cargo test --workspace           # Expected: all pass, 0 failures
cargo build --workspace          # Expected: 0 errors
cat desks/Example.json | python3 -c "import json,sys; d=json.load(sys.stdin); print(f'name={d[\"name\"]}, cards={len(d[\"cards\"])}')"
                                  # Expected: name=Example, cards=27
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass
- [ ] desks/Example.json exists and is valid JSON
- [ ] GameEngine draws initial_hand_size cards before first turn
- [ ] GameOver returns to MainMenu on Enter
- [ ] TUI deck browser lists decks from desks/
- [ ] TUI deck editor shows card info in right panel
- [ ] Pre-game deck selection works for local game
