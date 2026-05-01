# 对局端到端交互时间线

> **列约定**：
> - **Server**：服务端处理逻辑，包括引擎动作和消息推送
> - **Active**：当前回合的玩家（拥有操作权，如抽卡、登场、攻击等）
> - **Opponent**：等待对方操作的玩家（只能看对手回合进展）
> - 两个玩家在不同回合交替担任 Active/Opponent 角色
> - 状态标注：✅ 已修复 / ⚠️ 待修复 / ⏸️ 延后
> 
> **重要设计说明**：
> - `PhaseRunner::run_turn()` 中的 `events` Vec **在整个回合内累积不清空**，每次 `notify_clients` 发送的是从 TurnStart 到当前时刻的全部事件。
> - ✅ `notify_clients` 调用两个 `NetworkPhaseClient::on_events()`（P1 和 P2 各一次），每个 client **只发送自己玩家的 StateUpdate**（通过 `self.player_id`），共计 2 条消息。`convert_room_message` 按 `for_player` 过滤，每个玩家收到 1 条。

---

## 阶段 0：游戏启动

| # | Server | Active | Opponent |
|---|--------|--------|----------|
| 1 | `RoomManager::join_room` → 广播 `PlayerJoined{p1}` | — | — |
| 2 | `RoomManager::join_room` → 广播 `PlayerJoined{p2}` + `WaitingForDecks` | 收到 `RoomState`，显示房间信息 | 同左 |
| 3 | 双方 `submit_deck`，都 Ready 后广播 `GameStarting` | 收到 `GameStarting` | 同左 |
| 4 | `game_starter::start_game()`：<br>1) 加载双方牌库的卡定义<br>2) 校验卡组合法性<br>3) 创建 `GameState`，为双方牌组分配 InstanceId<br>4) 创建 `NetworkPhaseClient` + `ActionRequestState` | — | — |
| 5 | **广播** `GameStarted`（纯通知，无状态数据） | 收到 `game_started`，等待 StateUpdate | 同左 |
| 6 | **广播** `StateUpdate{for_player: P1}` + `StateUpdate{for_player: P2}`<br>（各自视角，此时手牌为 0，因为初始抽卡在 `GameEngine::run()` 中执行） | `StateUpdate` 到达：手牌 0 张的空初始状态<br>等待引擎启动后的第一次 StateUpdate | 同左 |
| 7 | `spawn_blocking(GameEngine::run)` 启动 | — | — |
| 8 | `GameEngine::run()` 内部：<br>1) 为双方各抽 5 张初始手牌（`DrawCard × 10`） ✅<br>2) 调用 `client.on_events(initial_draw_events, &state)` 通知双方<br>3) `on_events` 推 `StateUpdate`（含 `recent_events: [DrawCard{player:P1}, ...×5, DrawCard{player:P2}, ...×5]`） | 收到 `StateUpdate`：手牌变成 5 张<br>`recent_events` 含全部初始抽卡事件 | 同左，各自看到不同视角 |

---

## 阶段 1：TurnStart — 回合开始

| # | Server | Active | Opponent |
|---|--------|--------|----------|
| 1 | 引擎：`state.phase = TurnStart`，清空 `activated_this_turn` | — | — |
| 2 | 引擎 push `PhaseChanged{TurnStart}` → `on_events()` | — | — |
| 3 | `on_events()` 为双方各推 `StateUpdate`（`recent_events` 累积含 TurnStart 阶段变化事件） | 收到 `StateUpdate`：状态 + 事件<br>可读到 `PhaseChanged` 驱动阶段切换动画 | 同左 |
| 4 | 引擎自动推进到 Draw | — | — |

---

## 阶段 2：Draw — 抽卡阶段

| # | Server | Active | Opponent |
|---|--------|--------|----------|
| 1 | 引擎：`state.phase = Draw`，push `PhaseChanged{Draw}` | — | — |
| 2 | 若 Turn1 且 `first_player_draws = false`（默认），跳过抽卡 | — | — |
| 3 | 引擎：从 Active 牌组顶移除 1 张，加入手牌<br>push `DrawCard{player: Active, instance_id}` | — | — |
| 4 | `notify_clients()` 为双方各推 `StateUpdate`（`recent_events` 累积含 `PhaseChanged{Draw}` + `DrawCard`） | 收到 `StateUpdate`：手牌 +1<br>`recent_events` 包含抽到的 `instance_id`，可直接驱动"从牌组飞来这张卡"动画 | 收到 `StateUpdate`：`hand_count` +1<br>`recent_events` 含对手抽卡事件（只有 player 和 instance_id，无卡内容），驱动通用"对手抽卡"动画 |
| 5 | 引擎自动推进到 Recovery | — | — |

---

## 阶段 3：Recovery — 回收阶段

| # | Server | Active | Opponent |
|---|--------|--------|----------|
| 1 | 引擎：`state.phase = Recovery`，push `PhaseChanged{Recovery}` | — | — |
| 2 | 引擎计算 `recovery_count` = 对手前场最大费用 | — | — |
| 3a | 若 `recovery_count == 0`：跳过回收，推进到 Main1 | — | — |
| 3b | 若 `recovery_count > 0`：<br>Server → Active 发 `RecoveryRequested{count, options}`<br>（options 来自 Active 费用区的 instance_id + definition_id）<br>**✅ Opponent 不收到任何消息** | 收到 `RecoveryRequested`<br>客户端高亮费用区的卡，等待玩家选择 | —（无消息） |
| 4 | Server 阻塞等待 Active 的 `recovery` 响应 | 玩家选择 N 张卡，发送 `recovery{cards: [iid...]}` | — |
| 5 | 引擎：选中卡从 CostZone → Hand，push `CardMoved{from: CostZone, to: Hand}` × N | — | — |
| 6 | `notify_clients()` 推 `StateUpdate`（`recent_events` 累积含 `PhaseChanged{Recovery}` + `CardMoved` × N） | 收到 `StateUpdate`：手牌多了回收的卡 + 事件 | 收到 `StateUpdate`：`hand_count` 增加 + 事件 |
| 7 | 引擎推进到 Main1 | — | — |

---

## 阶段 4：Main1 — 主要阶段 1

| # | Server | Active | Opponent |
|---|--------|--------|----------|
| 1 | 引擎：`state.phase = Main1`，push `PhaseChanged{Main1}` | — | — |
| 2 | 引擎构建 `available_actions`：<br>Pass / Surrender / **PlayCard** / **ActivateEffect** ✅<br>* ActivateEffect 扫描范围：手牌（Trick 策略卡）、后场（所有策略/物品/传奇卡）、前场（所有类型，含传奇和角色卡）<br>* PlayCard 的 `ActionOption` 不含费用信息（`cost_payment` 字段始终为空，费用在 Godot 端从卡定义中读取） ⚠️ | — | — |
| 3 | Server → Active 发 `ActionRequested{actions, timeout}`<br>**✅ Opponent 不收到任何消息** | 收到 `action_request`：高亮可出手牌 + 可发动效果的卡，显示 Pass/Surrender | — |
| 4a | Active 选择 **Pass**：结束 Main1，推进到 Battle | 发 `action{pass}` | — |
| 4b | Active 选择 **Surrender**：游戏结束（Opponent 胜） | 发 `action{surrender}` | — |
| 4c | Active 选择 **PlayCard**（含 `cost_payment`）：<br>**事件顺序**（按实际 push 顺序）：<br>1) `RealPointChanged`（若用了 RP）<br>2) `CardExposed{instance_id}` + `CardMoved{Hand→CostZone}` × 费用数<br>3) `CardMoved{slot→Grave}`（仅当替换已有物品）<br>4) `CardSummoned{to: target_zone}` | 收到 `StateUpdate`：状态 + 事件列表 | 收到 `StateUpdate`：可见对手场上的变化 + 事件 |
| 4d | Active 选择 **ActivateEffect**：<br>**事件顺序**：<br>1) 费用支付 `CardMoved`（`SendFieldCardToGrave`/`DiscardHand`/`SendCostZoneToGrave`）<br>2) `EffectActivated{instance_id, effect_key}`<br>3) 链事件（`ChainStarted` → `ChainLink` → `ChainResolving`×N → `ChainComplete`，来自 `ChainManager::open_chain_window`，含交互式连锁窗口 + FILO 结算）<br>4) `CardMoved{→Grave}`（非存留策略/物品卡发动后送墓） | 收到 `StateUpdate`：效果结算后的状态 + 完整事件序列 | 收到 `StateUpdate`：可见变化 + 事件 |
| 5 | `notify_clients` → 推 `StateUpdate`，回到步骤 2 循环 | — | — |

---

## 阶段 5：Battle — 战斗阶段

| # | Server | Active | Opponent |
|---|--------|--------|----------|
| 1 | 引擎：`state.phase = Battle`，push `PhaseChanged{Battle}` | — | — |
| 2 | 引擎收集 Active 前场未攻击的卡（`attacked_this_turn == false`），逐个询问<br>注：不显式检查 `CardType::Character`，由登场规则保证前场只有人物卡 | — | — |
| 3 | 每张可攻击卡**单独一次** `ActionRequested`：<br>`DeclareAttack`（每个对方前场有卡的目标 slot）+ Pass<br>若对方前场无卡：`DeclareAttack{direct}` + Pass<br>**✅ Opponent 不收到消息** | 收到 `action_request` | — |
| 4a | Active 选择 **DeclareAttack（打前场卡）**：<br>push `AttackDeclared{attacker}`<br>比攻击力后的事件：<br>&nbsp;&nbsp;• 攻击方 > 被攻击方：`CardDestroyed{defender}` + RP + 1（`RealPointChanged`）<br>&nbsp;&nbsp;• 攻击方 < 被攻击方：`CardDestroyed{attacker}`（攻击方**不**获得 RP）<br>&nbsp;&nbsp;• 平局：双 `CardDestroyed`（双方都不获得 RP） | 发 `action{declare_attack}` → 收到 `StateUpdate` | 收到 `StateUpdate` |
| 4b | Active 选择 **DeclareAttack（直接攻击）✅**：<br>1) 攻击方获得 1 RP（`RealPointChanged`）<br>2) 若 RP > 0，调用 `client.choose_direct_attack_rp(max_rp, timeout)`（默认全部 RP）<br>3) RP -= selected（`RealPointChanged`）<br>4) 对手 HP -= selected（`HpChanged`，伤害上限为对手当前 HP）<br>5) HP = 0 则 push `GameOver` | 发 `action{declare_attack}` → RP 选择 → 收到 `StateUpdate` | 收到 `StateUpdate` |
| 4c | Active 选择 **Pass**：跳过该卡，进入下一张（`break` 出此卡的内层循环，继续 `for attacker_id in attackers`） | — | — |
| 5 | 每次攻击结算后 `notify_clients` 推 `StateUpdate`（`recent_events` 累积含整回合到当前的事件） | 状态 + 事件 | 对手可见变化 + 事件 |
| 6 | 所有攻击卡处理完毕，推进到 Main2 | — | — |

---

## 阶段 6：Main2 — 主要阶段 2

与 Main1 完全相同的流程（参见阶段 4），调用同一个 `run_main_phase` 函数，循环直到 Active 选择 Pass 或 Surrender。

---

## 阶段 7：TurnEnd — 回合结束

| # | Server | Active | Opponent |
|---|--------|--------|----------|
| 1 | 引擎：`state.phase = TurnEnd`，push `PhaseChanged{TurnEnd}` | — | — |
| 2 | `ModifierManager::cleanup_expired(state)` 移除 `UntilEndOfTurn` 修饰器<br>（注：函数返回 `Vec::new()`，无实际事件产出）<br>重置所有场地卡 `attacked_this_turn = false` | — | — |
| 3 | 回合切换：先 `turn_player = next`，再 `turn_number += 1`，最后 push `TurnChanged{new_active_player, turn_number}`（含递增后的值） | — | — |
| 4 | `notify_clients()` 推 `StateUpdate` 给双方（`recent_events` 累积含整回合所有事件） | `current_player` 变为 Opponent，角色互换 | `current_player` 变为自己，成为新 Active |
| 5 | 引擎继续 loop，先 `decrement_turn_counts`（减少 `TurnCount` 修饰器计数），再进入 TurnStart（阶段 1） | — | — |

---

## 阶段 8：游戏结束

| # | Server | Active | Opponent |
|---|--------|--------|----------|
| 1 | 引擎返回 `GameResult{winner, reason, ...}` | — | — |
| 2 | `broadcast_tx.send(GameOver{winner, reason})` → 双方都收到 `GameOver` 消息<br>Godot `NetworkManager` 收到后 emit `game_over` 信号 → `game_board` 弹窗显示胜负 + "返回大厅"按钮 | 弹窗显示结果 | 同左 |
| 3 | 房间状态 → `Finished` | — | — |

---

## ⏸️ 阶段 9：连锁流程（FILO）— 交互部分延后

> **已实现**：`ChainManager::open_chain_window` — 交互式连锁窗口框架完成：
> - 回合玩家优先 → 交替询问 → Instant 不入栈立即执行 → 双方连续 Pass 后 FILO 结算
> - `PhaseClient` trait 已有 `choose_chain_action` 回调（默认返回 `ChainPass`）
> - `PhaseRunner::build_chainable_effects` 扫描所有区域的可连锁效果
> - 服务端 `ChainActionRequested` 消息类型 + Godot `chain_action_request` 信号 + 连锁面板 UI 均已完成
> 
> **待实现**：`NetworkPhaseClient` 尚未覆盖 `choose_chain_action`（使用默认 `ChainPass` 实现），因此**当前所有连锁窗口都自动跳过交互直接进入 FILO 结算**。要启用交互式连锁，需在 `game_starter.rs` 的 `NetworkPhaseClient` 中实现该方法，通过 `ActionRequestState` 桥接 WebSocket 的 `ChainActivate`/`ChainPass` 响应。

---

## 问题状态汇总

### ✅ 已修复

| # | 严重度 | 问题 | 修复 |
|---|--------|------|------|
| #2 | P1 | CoreGameEvent 不推送到客户端 | `VisibleGameState` 新增 `recent_events`；`on_events` 传入事件并序列化 |
| #3 | P1 | 抽卡后客户端不知道抽到哪张卡 | 随 #2：`DrawCard{player, instance_id}` 在 `recent_events` 中 |
| #4 | P2 | 非行动方收到 null StateUpdate | `convert_room_message` 返回 `Option`，非目标方 `None` 不发送 |
| #5 | P0 | 主阶段不支持效果发动 | `PhaseAction::ActivateEffect` + `build_activatable_effects` |
| #7 | P2 | 直接攻击未实现 RP 策略选择 | `PhaseClient::choose_direct_attack_rp`，默认使用全部 RP |
| #8 | P3 | GameState 连到旧 Network autoload | `/root/Network` → `/root/NetworkManager` |
| #9 | P1 | 初始手牌 DrawCard 事件未到达客户端 | `GameEngine::run()` 初始抽卡后调用 `on_events` |

### ⚠️ 保留观察

| # | 严重度 | 问题 | 说明 |
|---|--------|------|------|
| #1 | P3 | `game_started` 与首个 `StateUpdate` 间消息空洞 | 同一 broadcast 通道保证顺序；Godot 客户端未设置 `waiting_for_state_update` 标志跟踪 |
| — | P3 | PlayCard 的 `ActionOption` 不含费用信息 | `cost_payment` 字段在 `build_main_actions` 中始终为 `CostPayment::new()` 且未通过网络传输；Godot 端从本地 JSON 读取费用 |

### ⏸️ 延后实现

| # | 严重度 | 问题 | 说明 |
|---|--------|------|------|
| #6 | P0 | 玩家主动连锁（交互式） | 框架已完成（`open_chain_window` + `choose_chain_action` trait + 协议 + Godot UI）。`NetworkPhaseClient` 尚未覆盖 `choose_chain_action`，当前自动走 `ChainPass` 默认实现。需在 `game_starter.rs` 中桥接 WebSocket 响应。 |

---

## 完整事件类型清单（含 CoreGameEvent 和 JSON 序列化名）

| CoreGameEvent 变体 | JSON key（serde 序列化） | 触发时点 | Godot 客户端处理 |
|---|---|---|---|
| `DrawCard` | `"DrawCard"` | 初始抽卡、Draw 阶段 | ✅ 已处理 |
| `CardSummoned` | `"CardSummoned"` | PlayCard 登场、SummonFromZone 动作 | ✅ 已处理 |
| `CardMoved` | `"CardMoved"` | 费用支付、回收、送墓等 | ✅ 已处理 |
| `CardDestroyed` | `"CardDestroyed"` | 战斗破坏、Destroy 动作 | ✅ 已处理 |
| `CardExposed` | `"CardExposed"` | 手牌进入费用区（支付费用时） | ❌ 未处理！需在 `_process_recent_events` 中添加 |
| `HpChanged` | `"HpChanged"` | 伤害、治疗 | ✅ 已处理 |
| `RealPointChanged` | `"RealPointChanged"` | RP 增减 | ✅ 已处理 |
| `RealPointOverflow` | `"RealPointOverflow"` | RP 达上限 | ✅ 已处理 |
| `PhaseChanged` | `"PhaseChanged"` | 每个阶段切换 | ✅ 已处理 |
| `TurnChanged` | `"TurnChanged"` | 回合结束切换玩家 | ✅ 已处理 |
| `EffectActivated` | `"EffectActivated"` | ActivateEffect 发动、被动触发 | ✅ 已处理 |
| `AttackDeclared` | `"AttackDeclared"` | 战斗宣言（打前场和直接攻击均有） | ✅ 已处理 |
| `ChainStarted` | `"ChainStarted"` | 连锁开始 | ❌ 未实现（延后） |
| `ChainLink` | `"ChainLink"` | 链入栈 | ❌ 未实现（延后） |
| `ChainResolving` | `"ChainResolving"` | 链结算 | ❌ 未实现（延后） |
| `ChainComplete` | `"ChainComplete"` | 链完成 | ❌ 未实现（延后） |
| `GameOver` | `"GameOver"` | 游戏结束 | ✅ 已处理 |

---

## 数据流总览

```
Client → WebSocket → ClientMessage { type: "action", action: ClientAction }
                              │
                  convert_client_action(ClientAction) → PhaseAction
                              │
                  PlayerAction::PhaseAction(phase_action)
                              │
                  submit_action → ActionRequestState.respond()
                              │
                  NetworkPhaseClient::choose_action() 阻塞等待 → 返回 PhaseAction
                              │
                  PhaseRunner::run_main_phase / run_battle_phase
                              │
                              ▼
                         GameEngine
                              │
            PhaseRunner::notify_clients(state, events, c1, c2)
                              │
            c1.on_events → build_visible_state(state, P1, events) → StateUpdate{for_player: P1}
            c2.on_events → build_visible_state(state, P2, events) → StateUpdate{for_player: P2}
                              │
            convert_room_message → Option<ServerMessage>
              非目标方 None 过滤 / 目标方 ServerMessage::StateUpdate
                              │
            Godot: _on_state_update → _update_from_state + _process_recent_events
```

**关键过滤点**：`convert_room_message` 对 `StateUpdate`、`ActionRequested`、`RecoveryRequested` 按 `for_player`/`player_id` 过滤，非目标方返回 `None` 不发送。`PlayerJoined`、`GameOver`、`GameStarted` 等无需过滤，双方都收到。
