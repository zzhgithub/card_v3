# Learnings — tui-refactor

## 2026-03-24 Session Start

### Codebase State (pre-refactor)
- card-tui/src/app.rs: 1712 lines — 渲染/逻辑/输入完全混合
- card-tui/src/ui.rs: 191 lines — 纯垂直 Layout，7个区域 Constraint 堆叠
- card-tui/src/input.rs: 61 lines — ActionInput/RecoveryInput 基础映射
- card-tui/src/deck_ui.rs: 空文件（Task 1 之前需要清理）
- card-tui/src/matchmaker_client.rs: 网络匹配客户端

### Key Types
- VisibleGameState: my_state(PlayerState), opponent_public(OpponentView), phase, turn_number, chain_size, is_my_turn
- CardInstance: instance_id, definition_id, current_attack, modifiers, attacked_this_turn, turn_summoned
- PlayerZones: deck/hand/front[5]/back[5]/cost_zone/grave
- CardDefinition.effects: HashMap<EffectKey, serde_json::Value> — JSON Value，需反序列化为 Effect
- Effect: trigger, optional, activation_limit, conditions, choices, actions, costs

### Build State
- cargo build --workspace: 编译通过，仅 2 warnings (dead_code on build_demo_state)
- cargo test --workspace: 全部通过

### Pattern Notes
- ratatui 0.29, crossterm 0.28
- 弹窗用 centered_rect + Clear 覆盖（Task 10 要改为不 Clear）
- App struct 字段全部 pub(方便子模块访问)

## 2026-03-24 Task 1 Mechanical Split

- 将 `card-tui/src/ui.rs` 机械拆分为 `ui/` 目录：`field.rs`(场地+槽位辅助)、`hand.rs`、`log.rs`，并删除旧 `ui.rs`。
- 将 `app.rs` 中渲染函数迁移到 `ui/layout.rs`、`ui/menu.rs`、`ui/overlay.rs`，`app.rs::render()` 改为参数透传调用。
- Overlay 保持原行为（仍使用 `Clear` + `centered_rect`），仅把描述字符串构造移动到 `app.rs` 调用侧。
- 网络流程函数迁移到 `card-tui/src/network.rs`；本地对局构建函数迁移到 `card-tui/src/game_setup.rs`。
- `main.rs` 新增 `mod network; mod game_setup; mod ui;`，占位文件 `ui/info_panel.rs` 与 `ui/hp_display.rs` 按任务要求保留 `// placeholder`。
- 验证结果：`cargo build -p card-tui` 无 error；tmux 不可用导致启动场景仅记录环境错误（见证据文件）。

## 2026-03-24 Task 2 Effect 文本化

- 在 `crates/card-core/src/effect/text.rs` 建立 AST→中文文本转换层，按 `Trigger/Condition/Action/Cost/Modifier` 分层映射，避免在 `effect_to_chinese` 内硬编码大分支。
- `effects_from_json` 采用 `serde_json::from_value::<Effect>(v.clone())`，并按 `EffectKey` 排序输出，保证 `HashMap` 输入下测试结果稳定。
- `card_effects_text` 对反序列化失败统一返回 `（效果数据异常）`，保持渲染流程可继续而不 panic。
- 组合文案格式实践：`[时点][限制]，当[条件]时，[可选+费用]，[行动序列]。`；多行动统一用 `，然后` 串联，卡牌文案更接近自然读感。
- 覆盖策略：为所有 Trigger/EventTrigger、Action、CostRequirement、Modifier 枚举分支建立直接断言，再补 3 组端到端 + ReadMe 示例断言，降低漏分支风险。

## 2026-03-29 Task 9 场地光标导航

- `app.rs` 采用“弹窗优先”分支不变：仅在 `pending_actions/pending_recovery` 都为空时处理浏览模式键位，避免破坏既有确认流。
- 浏览模式入口/出口采用单点函数 `handle_field_cursor_key`：`Tab` 切换激活，激活后 `Esc` 退出，方向键与快捷键都集中在同一匹配分支。
- `layout.rs` 右栏改为真实渲染：`render_info_panel` + `render_event_log`，并透传 `FieldCursor` 与 `card_defs`，保证光标激活时信息面板随位置变化。
- `field.rs` 槽位高亮用统一 `slot_style`（黄底黑字+BOLD），前后场 5 槽和双方费用区 6 槽都支持空槽高亮。

## 2026-03-29 Task 10 半透明弹窗（可最小化）

- 在 `crates/card-tui/src/ui/overlay.rs` 新增 `OverlayState { visible, minimized, overlay_type }` 与 `OverlayType`（`ActionSelect/RecoverySelect/None`），并提供 `for_action/for_recovery/clear/toggle_minimized`，用于明确弹窗状态切换。
- 弹窗渲染去掉 `Clear`，改为仅在居中区域渲染带深色背景的 `Block + Paragraph`，保留弹窗外场地可见；弹窗尺寸统一为 `60% x 50%`。
- 在 `crates/card-tui/src/ui/layout.rs` 增加 `overlay_state` 透传：`minimized=true` 时只渲染底部单行提示（`[操作选择] 按 m 恢复` / `[回收选择] 按 m 恢复`），否则维持原弹窗内容渲染。
- 在 `crates/card-tui/src/app.rs` 接入 `m` 切换：当 overlay 可见时优先消费 `m`；未最小化时保持原有 popup-first 输入优先级；最小化时放行 Task 9 的场地浏览路径（`Tab/方向键`）。
- 现有提交语义保持不变：`handle_action_key/handle_recovery_key` 与 `submit_action/submit_recovery` 的确认/取消逻辑未改，仅在提交后清理 overlay 状态。

## 2026-03-29 Task 11 输入系统重构+快捷键映射

### 新增内容
- 创建 `InputMode` 枚举统一管理所有输入模式：Menu、FieldBrowse、ActionSelect、RecoverySelect、LogScroll、DeckEditor
- 创建 `InputAction` 枚举统一输入动作：导航类(MoveUp/Down/Left/Right)、确认类(Confirm/Escape/Toggle)、场地浏览专用(QuickJump/MinimizeOverlay/ToggleFieldBrowse/FocusLog)
- 实现 `map_key(mode, key) -> InputAction` 统一映射函数，根据模式分发到具体映射逻辑
- 场地浏览模式快捷键：
  - 数字 1-5 → MyFront(0-4)
  - Shift+数字 !@#$% → MyBack(0-4)
  - h → MyHand, c → MyCostZone, f → OpponentFront, b → OpponentBack
  - Tab → ToggleFieldBrowse, m → MinimizeOverlay, l → FocusLog, Esc → Escape
- app.rs 更新：handle_action_key、handle_recovery_key、handle_field_cursor_key 均改用新 map_key 系统
- 为 FieldZone 添加 Copy trait 以支持 InputAction::QuickJump(FieldZone) 的 Copy 派生

### 兼容策略
- 保留 ActionInput/RecoveryInput 枚举作为 legacy 导出
- 保留 map_action_key/map_recovery_key 函数，内部调用 map_key 并转换结果
- 避免一次性破坏性变更，允许逐步迁移

### 代码组织
- 输入映射逻辑按模式拆分为独立函数(map_menu_key, map_field_browse_key 等)
- 测试覆盖每个模式的键位映射和 legacy 函数兼容性

## 2026-03-30 Task 12 LogState 集成

### 变更摘要
- App struct: `game_events: Vec<String>` → `log_state: LogState`
- 所有事件推送点改为 `log_state.push(LogEntry { message, source })`
- 日志来源区分：玩家操作 → `LogSource::Me`，系统消息 → `LogSource::System`
- `layout.rs`: 使用 `render_log()` 替代旧 `render_event_log()`，右下区域显示带滚动条的日志面板
- `render_game_screen()` 签名更新：`&[String]` → `&LogState`

### 遗留接口
- `render_event_log()` 仍保留在 `log.rs` 供 backwards compatibility，但 `layout.rs` 已不再调用
- `LogSource::Opponent` 未使用（待联机对战实现后启用）

### 验证
- `cargo build -p card-tui` 通过
- `cargo test -p card-core` 125 测试通过
- `cargo test -p card-tui` 51 测试通过
