# Local Game Board Rewrite Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 重写 `card-bevy` 本地游戏场地，使其按 `plan/游戏场地.png` 呈现清晰的三栏静态布局，并为每个区域显示文字提示。

**Architecture:** 保留现有 `LocalGame` 进入流程、卡组初始化和最小 UI，只替换 `game_board.rs` 中的静态场地定义与布局资源。中央战场仍由精灵绘制，区域标题和说明文字继续使用 Bevy 文本组件叠加在静态场地上，不引入新的交互逻辑或状态刷新系统。

**Tech Stack:** Rust, Bevy, card-bevy 本地游戏状态系统

---

### Task 1: 先写布局回归测试

**Files:**
- Modify: `crates/card-bevy/src/game_board.rs`
- Test: `crates/card-bevy/src/game_board.rs`

**Step 1: Write the failing test**

在 `game_board.rs` 的 `#[cfg(test)]` 模块里新增最小断言，验证：
- `BoardLayout` 包含左侧详情区、中央战场区、右侧信息区的分栏宽度/锚点
- 上下玩家区域的 `deck/grave/cost/front/back/hand` 坐标满足镜像关系
- 中线回合信息条位于中央主战场中线附近

**Step 2: Run test to verify it fails**

Run: `cargo test --package card-bevy game_board -- --nocapture`

Expected: 新增测试失败，因为现有 `BoardLayout` 仍是旧的简化中央布局。

**Step 3: Write minimal implementation**

扩展 `BoardLayout` / `PlayerAreaLayout`，加入三栏结构和静态区块坐标，让测试能表达目标布局。

**Step 4: Run test to verify it passes**

Run: `cargo test --package card-bevy game_board -- --nocapture`

Expected: 新增布局测试通过，已有 `board_to_world` 测试继续通过。

### Task 2: 重写静态场地绘制

**Files:**
- Modify: `crates/card-bevy/src/game_board.rs`

**Step 1: Write the failing test**

补一个最小测试，验证 `BoardLayout::default()` 会为两名玩家都生成手卡区域和主战区坐标，避免再次遗漏 `Hand` 区域。

**Step 2: Run test to verify it fails**

Run: `cargo test --package card-bevy hand -- --nocapture`

Expected: 失败，因为旧布局未真正建模手卡区域。

**Step 3: Write minimal implementation**

重写 `setup_game_board`：
- 绘制整屏背景与三栏面板底色
- 左侧绘制卡牌详情图片区和信息区
- 中央绘制上方玩家、回合条、下方玩家
- 每个区域绘制背景块与文本标签：手卡、前场、后场、费用区、卡组、墓地、回合数/当前玩家/当前阶段、对手信息、己方信息、跳过区域、认输区域
- 保持 `GameBoardEntity` 统一清理

**Step 4: Run test to verify it passes**

Run: `cargo test --package card-bevy game_board -- --nocapture`

Expected: 布局相关测试全部通过。

### Task 3: 对齐本地游戏初始牌背位置

**Files:**
- Modify: `crates/card-bevy/src/local_game.rs`
- Test: `crates/card-bevy/src/local_game.rs`

**Step 1: Write the failing test**

增加一个最小测试，验证 `spawn_initial_cards` 依赖的新 `deck_pos` 仍然可用于上下双方卡组牌背堆叠，不需要新增交互字段。

**Step 2: Run test to verify it fails**

Run: `cargo test --package card-bevy local_game -- --nocapture`

Expected: 如果布局字段重命名/移动后未同步，这里会失败或编译不过。

**Step 3: Write minimal implementation**

只同步 `local_game.rs` 中对 `BoardLayout` 的访问，移除不再需要的旧导入，保证进入本地游戏时仍会在新卡组区域生成初始牌背。

**Step 4: Run test to verify it passes**

Run: `cargo test --package card-bevy local_game -- --nocapture`

Expected: 本地游戏相关测试通过，动作按钮文字测试保持通过。

### Task 4: 构建与手动验证

**Files:**
- Verify: `crates/card-bevy/src/game_board.rs`
- Verify: `crates/card-bevy/src/local_game.rs`

**Step 1: Run diagnostics**

Run: language server diagnostics on modified Rust files.

**Step 2: Build the app**

Run: `cargo build --package card-bevy`

Expected: exit code 0.

**Step 3: Manual QA**

Run: `cargo run --package card-bevy`

Expected:
- 进入本地游戏后能看到左详情 / 中战场 / 右信息操作三栏
- 上下双方的手卡、前场、后场、费用区、卡组、墓地区域都有文字提示
- 中线有回合信息条
- 本轮只显示静态“跳过”“认输”区域，不新增点击行为

**Step 4: Commit**

Run only if user asks for commit.
