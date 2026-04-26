# Rust Server 修复方案汇总

> **说明**：本文档为每个已识别的问题提供独立的修复方案与验证集。每个问题单独成节，按依赖关系排序（先修复底层数据缺失，再修复上层表现）。所有方案均基于当前代码上下文推导，未实际执行。

---

## 问题 1：WebSocket 路径缺少持续的状态更新

### 问题描述
`game_starter::start_game()` 仅在游戏开始时通过 `RoomMessage::GameStarted` + `StateUpdate` 发送一次状态。之后 `GameEngine` 在 `spawn_blocking` 中同步运行，每次阶段推进、抽卡、战斗结算、连锁解算都会产生大量 `CoreGameEvent`，但**没有任何机制**将这些状态变化推送给 WebSocket 客户端。

Raw TCP 路径通过 `RemoteClient::on_event()` 发送 `EventNotification`，但 WebSocket 路径的 `NetworkPhaseClient` 只实现了 `PhaseClient`（`choose_action` / `choose_recovery_cards`），没有事件回调通道。

### 影响
- Godot 客户端在游戏开始后收不到任何状态更新，UI 完全冻结在初始画面
- 客户端无法根据服务器权威来源更新手牌、HP、场地等

### 修复方案

#### 方案 A：扩展 PhaseClient trait 增加事件回调（推荐）

**思路**：在 `PhaseClient` 中增加一个 `on_events` 方法，让 `GameEngine` 在每次 `run_turn` 后将本回合产生的事件列表传给客户端。`NetworkPhaseClient` 实现中将事件转换为状态更新广播。

**修改文件**：
1. `crates/card-core/src/engine/phase.rs`
   - 在 `PhaseClient` trait 中增加新方法：
   ```rust
   fn on_events(&self, _events: &[CoreGameEvent], _state: &GameState) {}
   ```
   - 在 `PhaseRunner::run_turn` 返回前，或 `GameEngine::run` 的每次循环后调用此方法

2. `crates/card-core/src/engine/game_engine.rs`
   - 在 `run()` 方法中，每次 `PhaseRunner::run_turn` 返回后：
   ```rust
   let turn_events = PhaseRunner::run_turn(...);
   // ... existing follow-up resolution ...
   self.clients[0].on_events(&turn_events, &self.state);
   self.clients[1].on_events(&turn_events, &self.state);
   ```
   - 同样在处理完 `resolve_follow_up_effects` 后，将后续事件也广播

3. `crates/card-server/src/game_starter.rs`
   - 在 `NetworkPhaseClient` 中实现 `on_events`：
   ```rust
   fn on_events(&self, events: &[CoreGameEvent], state: &GameState) {
       // 构建两个视角的 VisibleGameState
       let visible_p1 = build_visible_state(state, PlayerId::Player1);
       let visible_p2 = build_visible_state(state, PlayerId::Player2);
       
       // 广播 StateUpdate（通过 RoomMessage）
       let _ = self.broadcast_tx.send(RoomMessage::StateUpdate {
           for_player: PlayerId::Player1,
           state: visible_p1,
       });
       let _ = self.broadcast_tx.send(RoomMessage::StateUpdate {
           for_player: PlayerId::Player2,
           state: visible_p2,
       });
       
       // 可选：将 CoreGameEvent 也作为独立消息广播
       // 这样客户端可以播放动画/音效
       for event in events {
           let _ = self.broadcast_tx.send(RoomMessage::EventOccurred {
               for_player: self.player_id,
               event: event.clone(),
           });
       }
   }
   ```
   - 如果需要在 `RoomMessage` 中新增 `EventOccurred` 变体，同步更新 `convert_room_message`

4. `crates/card-server/src/room.rs`
   - 新增 `RoomMessage::EventOccurred { for_player: PlayerId, event: CoreGameEvent }`
   - （可选，如果客户端需要事件驱动动画）

5. `crates/card-server/src/websocket_server.rs`
   - 在 `convert_room_message` 中处理新的消息类型（如有新增）

#### 方案 B：不修改 trait，改为在 NetworkPhaseClient 内部维护状态引用（不推荐）

通过 `Arc<Mutex<GameState>>` 让 `NetworkPhaseClient` 和 `GameEngine` 共享状态，由 `NetworkPhaseClient` 在 `choose_action` 被调用时顺便推送状态。但这个方案的问题在于：非当前行动玩家也会经历状态变化（如对手抽卡、战斗结算），他们不会在此时收到 `choose_action` 调用，所以状态依然不同步。

### 验证集

#### 单元测试 1：PhaseClient trait 兼容性
```rust
// crates/card-core/src/engine/phase.rs 的 tests 模块中
#[test]
fn phase_client_on_events_default_no_panic() {
    struct NoOpClient;
    impl PhaseClient for NoOpClient {
        fn choose_action(&self, _: &[PhaseAction], _: Duration) -> Option<PhaseAction> {
            Some(PhaseAction::Pass)
        }
        fn choose_recovery_cards(&self, _: &[RecoveryCardOption], _: usize, _: Duration) -> Option<Vec<InstanceId>> {
            Some(vec![])
        }
    }
    let client = NoOpClient;
    let state = make_state();
    client.on_events(&[CoreGameEvent::PhaseChanged { new_phase: Phase::Draw }], &state);
    // 默认实现不应 panic
}
```

#### 单元测试 2：NetworkPhaseClient 状态广播
```rust
// crates/card-server/src/game_starter.rs 的 tests 模块中
#[test]
fn network_phase_client_on_events_broadcasts_state_update() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let action_state = Arc::new(ActionRequestState::new());
    let client = NetworkPhaseClient::new(PlayerId::Player1, tx, action_state);
    
    let mut state = GameState::new(GameRules::default(), 42);
    // 放置一些卡牌使状态有区分度
    state.players[0].zones.hand.push(CardInstance::new(InstanceId(1), CardId::new("TEST"), Some(100)));
    
    client.on_events(&[], &state);
    
    // 应该收到两条 StateUpdate（P1 和 P2 视角）
    let msg1 = rx.try_recv().expect("Expected StateUpdate for P1");
    let msg2 = rx.try_recv().expect("Expected StateUpdate for P2");
    
    match msg1 {
        RoomMessage::StateUpdate { for_player, state } => {
            assert_eq!(for_player, PlayerId::Player1);
            assert_eq!(state.your_state.hand.len(), 1);
        }
        _ => panic!("Expected StateUpdate, got {:?}", msg1),
    }
    match msg2 {
        RoomMessage::StateUpdate { for_player, state } => {
            assert_eq!(for_player, PlayerId::Player2);
            assert_eq!(state.opponent_state.hand_count, 1); // P2 看 P1 的手牌数量
        }
        _ => panic!("Expected StateUpdate, got {:?}", msg2),
    }
}
```

#### 集成测试 3：完整对局中的状态更新流
在 `crates/card-server/tests/room_websocket_test.rs` 的 `test_complete_game_flow` 基础上扩展：
```rust
// 在发送 Pass action 后，等待 StateUpdate
let timeout_result = timeout(Duration::from_millis(1000), client1.next()).await;
if let Ok(Some(Ok(Message::Text(text)))) = timeout_result {
    let msg = serde_json::from_str::<TestServerMessage>(text.as_str()).unwrap();
    match msg {
        TestServerMessage::StateUpdate { state } => {
            let parsed: VisibleGameState = serde_json::from_value(state).unwrap();
            // 验证回合数或阶段发生了变化
            assert!(parsed.turn_number >= 1);
            println!("[Test] ✓ Received StateUpdate after action: Turn {:?}, Phase {:?}", 
                parsed.turn_number, parsed.current_phase);
        }
        _ => panic!("Expected StateUpdate after action, got {:?}", msg),
    }
}
```

#### 手动验证 4：Python 端到端测试
修改 `test_game_start.py` 或 `test_two_player_flow.py`，在游戏开始后发送一个 `action: pass`，然后断言收到 `state_update` 消息（JSON 中 `type == "state_update"` 且 `state` 非空）。

---

## 问题 2：ActionRequested 的 available_actions 被丢弃（发送空列表）

### 问题描述
`websocket_server.rs::convert_room_message()` 处理 `RoomMessage::ActionRequested` 时：
```rust
ActionRequested { player_id, available_actions: _, timeout_secs } => {
    if my_player_id == Some(player_id) {
        ServerMessage::ActionRequest {
            available_actions: vec![], // Simplified - 硬编码为空!
            timeout_secs,
        }
    } else { ... }
}
```
客户端收到 `action_request` 时不知道有哪些合法行动，UI 无法展示可操作的按钮。

### 影响
- Godot 客户端收到 `action_request` 但 `available_actions` 为空数组
- 测试脚本 `test_complete_game_flow` 中虽然能检测到 Pass，但那是因为 `PhaseAction` 的 `Debug` 格式化字符串中恰好包含 "Pass" 字样——实际上所有行动的字符串描述都是空的

### 修复方案

#### 步骤 1：设计 JSON 可用的行动描述格式
观察 `AvailableAction`（Raw TCP 协议中已定义）的结构：
```rust
pub enum AvailableAction {
    PlayCard { instance_id: InstanceId },
    DeclareAttack { attacker: InstanceId, possible_targets: Vec<AttackTarget> },
    ChainPass,
    Surrender,
}
```

WebSocket JSON 协议中，`action_request` 的 `available_actions` 目前是 `Vec<String>`（见 PROTOCOL.md 示例为 `["Pass", "Surrender", "PlayCard"]`）。但 Godot 客户端在 `game_board.gd` 和 `debug.gd` 中只是打印这个数组，没有做结构化解析。

**有两种选择**：
- **选项 A**：维持 `Vec<String>`，但生成有意义的字符串（如 `"Pass"`, `"Surrender"`, `"PlayCard:1"`, `"DeclareAttack:1->Direct"`）
- **选项 B**：改为发送结构化 JSON（如 `[{"action":"PlayCard","instance_id":1,"target":"Front(0)"}]`），但需同步修改 Godot 客户端解析逻辑

基于最小改动原则，**推荐选项 A**：先用字符串描述，后期如需 richer UI 再改为结构化。

#### 步骤 2：修改 RoomMessage::ActionRequested 的可用行动类型
目前 `RoomMessage::ActionRequested` 中 `available_actions` 是 `Vec<String>`，且来源是 `PhaseAction` 的 `{:?}` Debug 格式化。这个做法本身就不对——Debug 格式不适合协议传输。

需要在 `game_starter.rs::NetworkPhaseClient::choose_action` 中，将 `PhaseAction` 转换为有意义的字符串描述：

```rust
// 在 NetworkPhaseClient::choose_action 中
let action_strings: Vec<String> = available
    .iter()
    .map(|a| match a {
        PhaseAction::Pass => "Pass".to_string(),
        PhaseAction::Surrender => "Surrender".to_string(),
        PhaseAction::PlayCard { instance_id, target_zone, .. } => {
            format!("PlayCard:{}:{:?}", instance_id.0, target_zone)
        }
        PhaseAction::DeclareAttack { attacker, target_slot } => {
            match target_slot {
                Some(slot) => format!("DeclareAttack:{}:Slot:{}", attacker.0, slot),
                None => format!("DeclareAttack:{}:Direct", attacker.0),
            }
        }
    })
    .collect();
```

这样 `RoomMessage::ActionRequested` 中的 `available_actions` 就已经是有意义的字符串了。然后 `convert_room_message` 只需原样透传：

```rust
ActionRequested { player_id, available_actions, timeout_secs } => {
    if my_player_id == Some(player_id) {
        ServerMessage::ActionRequest {
            available_actions, // 直接透传，不再丢弃
            timeout_secs,
        }
    } else { ... }
}
```

#### 步骤 3：Godot 客户端是否需要修改？
查看 `godot-client/scenes/debug.gd` 第 85-90 行，它打印 `available_actions` 但不解析。`game_board.gd` 中没有连接 `action_request` 信号到具体 UI 操作（只是预留了信号连接）。

所以**不需要修改 Godot 代码**——修复后客户端至少能看到正确的字符串数组，后续 UI 开发可以基于此进行。

### 验证集

#### 单元测试 1：字符串转换正确性
```rust
#[test]
fn phase_action_to_string_format() {
    use card_core::types::Zone;
    let actions = vec![
        PhaseAction::Pass,
        PhaseAction::Surrender,
        PhaseAction::PlayCard {
            instance_id: InstanceId(42),
            target_zone: Zone::Front(2),
            cost_payment: vec![],
        },
        PhaseAction::DeclareAttack {
            attacker: InstanceId(10),
            target_slot: Some(3),
        },
        PhaseAction::DeclareAttack {
            attacker: InstanceId(10),
            target_slot: None,
        },
    ];
    
    let strings: Vec<String> = actions.iter().map(|a| match a {
        PhaseAction::Pass => "Pass".to_string(),
        PhaseAction::Surrender => "Surrender".to_string(),
        PhaseAction::PlayCard { instance_id, target_zone, .. } => {
            format!("PlayCard:{}:{:?}", instance_id.0, target_zone)
        }
        PhaseAction::DeclareAttack { attacker, target_slot } => {
            match target_slot {
                Some(slot) => format!("DeclareAttack:{}:Slot:{}", attacker.0, slot),
                None => format!("DeclareAttack:{}:Direct", attacker.0),
            }
        }
    }).collect();
    
    assert_eq!(strings[0], "Pass");
    assert_eq!(strings[1], "Surrender");
    assert!(strings[2].starts_with("PlayCard:42:"));
    assert_eq!(strings[3], "DeclareAttack:10:Slot:3");
    assert_eq!(strings[4], "DeclareAttack:10:Direct");
}
```

#### 集成测试 2：WebSocket 测试验证 ActionRequest 包含内容
在 `room_websocket_test.rs` 的 `test_complete_game_flow` 中，收到 `ActionRequest` 时断言 `available_actions` 非空：
```rust
TestServerMessage::ActionRequest { available_actions, timeout_secs } => {
    println!("[Test] Available actions: {:?}", available_actions);
    assert!(!available_actions.is_empty(), "available_actions should not be empty");
    assert!(available_actions.iter().any(|a| a == "Pass"), "Pass should always be available");
}
```

#### 手动验证 3：Python 测试
运行 `test_two_player_flow.py`，在收到 `action_request` 后打印 `available_actions`，验证其非空且包含可识别的行动字符串。

---

## 问题 3：RecoveryRequested 的 options 被丢弃（发送空列表）

### 问题描述
与问题 2 同理，`convert_room_message` 中对 `RecoveryRequested` 的处理：
```rust
RecoveryRequested { player_id, count, options: _ } => {
    if my_player_id == Some(player_id) {
        ServerMessage::RecoveryRequest {
            count,
            options: vec![], // Simplified - 硬编码为空!
        }
    }
}
```

### 影响
- 客户端收到 `recovery_request` 但不知道可选哪些卡
- 虽然 `InstanceId` 可以作为选项 ID 发送，但当前被完全丢弃

### 修复方案

#### 步骤 1：定义 RecoveryCardOption 的字符串表示
`RecoveryCardOption` 结构：
```rust
pub struct RecoveryCardOption {
    pub instance_id: InstanceId,
    pub definition_id: CardId,
}
```

在 `NetworkPhaseClient::choose_recovery_cards` 中，将 `options` 转换为字符串：
```rust
let options_strings: Vec<String> = options
    .iter()
    .map(|o| format!("{}:{}", o.instance_id.0, o.definition_id.0))
    .collect();
```

#### 步骤 2：修改 convert_room_message 透传 options
```rust
RecoveryRequested { player_id, count, options } => {
    if my_player_id == Some(player_id) {
        ServerMessage::RecoveryRequest {
            count,
            options, // 直接透传
        }
    }
}
```

#### 步骤 3：Godot 客户端兼容性
`PROTOCOL.md` 中 `recovery_request` 的示例是 `{"count": 2, "options": ["card1", "card2"]}`，说明协议设计时就已经预期 `options` 是字符串数组。修复后与此一致。

Godot 的 `scripts/network.gd` 第 125-127 行：
```gdscript
"recovery_request":
    var count = data.get("count", 0)
    var options = data.get("options", [])
```
只是读取数组，不做结构化解析，所以**无需修改 Godot 代码**。

### 验证集

#### 单元测试 1：NetworkPhaseClient options 字符串转换
```rust
#[test]
fn recovery_options_to_string_format() {
    let options = vec![
        RecoveryCardOption {
            instance_id: InstanceId(5),
            definition_id: CardId::new("S000-C-001"),
        },
        RecoveryCardOption {
            instance_id: InstanceId(8),
            definition_id: CardId::new("S000-C-002"),
        },
    ];
    let strings: Vec<String> = options.iter()
        .map(|o| format!("{}:{}", o.instance_id.0, o.definition_id.0))
        .collect();
    assert_eq!(strings[0], "5:S000-C-001");
    assert_eq!(strings[1], "8:S000-C-002");
}
```

#### 集成测试 2：WebSocket 测试中验证 RecoveryRequest 含 options
需要在集成测试中构造一个会触发 Recovery 阶段的情景。由于 recovery_count = 对手前场最高费用，可以在测试中：
1. 使用带有费用的 Character 卡构建 deck
2. 在 Main1 阶段打出一张卡到前场
3. 在下一回合的 Recovery 阶段，验证收到的 `RecoveryRequest` 的 `options` 非空

或者更简单地，在 `room_websocket_test.rs` 中新增一个测试，直接模拟 `RoomMessage::RecoveryRequested` 的广播和转换：
```rust
#[test]
fn test_recovery_request_conversion() {
    let msg = RoomMessage::RecoveryRequested {
        player_id: PlayerId::Player1,
        count: 2,
        options: vec!["5:S000-C-001".to_string(), "8:S000-C-002".to_string()],
    };
    let server_msg = convert_room_message(msg, Some(PlayerId::Player1));
    match server_msg {
        ServerMessage::RecoveryRequest { count, options } => {
            assert_eq!(count, 2);
            assert_eq!(options.len(), 2);
            assert_eq!(options[0], "5:S000-C-001");
        }
        _ => panic!("Expected RecoveryRequest"),
    }
    
    // 验证对手收到的是 Null state update
    let msg = RoomMessage::RecoveryRequested {
        player_id: PlayerId::Player1,
        count: 2,
        options: vec!["5:S000-C-001".to_string()],
    };
    let server_msg = convert_room_message(msg, Some(PlayerId::Player2));
    match server_msg {
        ServerMessage::StateUpdate { state } => assert!(state.is_null()),
        _ => panic!("Expected null StateUpdate for opponent"),
    }
}
```

---

## 问题 4：GameStarted 消息只广播 P1 视角的状态

### 问题描述
`game_starter.rs` 第 264 行：
```rust
room_guard.broadcast_message(RoomMessage::GameStarted {
    state: visible_p1.clone(),  // 只带 P1 视角!
});
```

随后虽然单独发送了 `StateUpdate`（P1/P2 各一条），但 `GameStarted` 本身没有做玩家过滤，所有订阅者收到的是同一个 P1 视角。

### 影响
- P2 收到的 `game_started` 消息中，`your_state` 实际上是 P1 的状态（包括 P1 的手牌内容！）
- 信息隐藏被破坏，P2 可以看到 P1 的手牌
- 虽然后续的 `StateUpdate` 会纠正这个错误，但 Race condition 可能导致 Godot 客户端先用错误的 `game_started` 渲染了一帧

### 修复方案

#### 方案 A：移除 GameStarted 中的 state 载荷，改为纯通知（推荐）
最简单、最安全的方式：让 `GameStarted` 只作为一个"游戏已开始"的通知，不含具体状态数据。真实状态由后续的 `StateUpdate` 提供。

**修改**：
1. `crates/card-server/src/room.rs`
   ```rust
   pub enum RoomMessage {
       // ...
       GameStarted,  // 移除 { state: VisibleGameState } 字段
       // ...
   }
   ```

2. `crates/card-server/src/game_starter.rs`
   ```rust
   room_guard.broadcast_message(RoomMessage::GameStarted);  // 无状态
   room_guard.broadcast_message(RoomMessage::StateUpdate { for_player: P1, state: visible_p1 });
   room_guard.broadcast_message(RoomMessage::StateUpdate { for_player: P2, state: visible_p2 });
   ```

3. `crates/card-server/src/websocket_server.rs`
   ```rust
   GameStarted => ServerMessage::GameStarted {
       state: serde_json::Value::Object(Default::default()), // 空对象
   },
   ```

4. `godot-client/` 中检查是否有代码依赖 `game_started.state`
   - `scripts/network.gd:110`：`var state = data.get("state", {})` — 已经做了默认值处理
   - `managers/network_manager.gd:209`：`var state = data.get("state", {})` — 同样有空默认值
   - `scenes/game_boads/game_board.gd:127`：`var state = game_data.get("state", {})`
   - `scenes/debug.gd:66-69`：打印 turn/phase，如果 state 为空则打印空值
   
   所有代码都能处理空 `state`，因为都使用了 `.get("...", default)`。

#### 方案 B：为每个玩家单独发送 GameStarted（复杂，不推荐）
如果要保留 `GameStarted` 中带状态的设计，需要修改广播机制，让每个订阅者收到不同的消息。这要求 `broadcast_message` 支持按订阅者差异化发送，改动较大。

### 验证集

#### 单元测试 1：GameStarted 转换后 state 为空
```rust
#[test]
fn test_game_started_no_state_leak() {
    let msg = RoomMessage::GameStarted;
    let server_msg = convert_room_message(msg, Some(PlayerId::Player1));
    match server_msg {
        ServerMessage::GameStarted { state } => {
            assert!(state.as_object().map(|o| o.is_empty()).unwrap_or(false),
                "GameStarted should not contain state data");
        }
        _ => panic!("Expected GameStarted"),
    }
}
```

#### 集成测试 2：WebSocket 测试中验证 GameStarted 后 StateUpdate 正确
在 `test_complete_game_flow` 中：
```rust
// 收到 GameStarted 后（可能 state 为空或旧数据），紧接着应该收到 StateUpdate
let msg = recv_message(&mut client2).await;
match msg {
    TestServerMessage::StateUpdate { state } => {
        let parsed: VisibleGameState = serde_json::from_value(state).unwrap();
        // P2 的 your_state.hand 应该是 P2 自己的手牌（长度应为 initial_hand_size）
        // P2 的 opponent_state.hand_count 应该是 P1 的手牌数量
        assert_eq!(parsed.current_player, PlayerId::Player1); // P1 先手
    }
    _ => panic!("Expected StateUpdate after GameStarted"),
}
```

#### 手动验证 3：信息隐藏检查
在 Python 测试或 WebSocket 测试中，截获 P2 收到的第一条 `game_started` + 第一条 `state_update`，验证：
- P2 的 `your_state.hand` 中的 `definition_id` 与 P1 提交的 deck 一致（而非 P2 的 deck）
- 两个玩家的 `your_state.hand` 不应该是相同的卡牌

---

## 问题 5：WebSocket PlayCard 的 cost_payment 被硬编码为空

### 问题描述
`websocket_server.rs::convert_client_action()` 中：
```rust
ClientAction::PlayCard { instance_id, target_zone } => {
    Ok(PhaseAction::PlayCard {
        instance_id: InstanceId(instance_id),
        target_zone: zone,
        cost_payment: vec![], // Simplified - should calculate from client
    })
}
```

当前 `ClientAction::PlayCard` 的 JSON 定义中**不包含** `cost_payment` 字段，所以服务器无法从客户端获取费用支付信息。

### 影响
- 当前引擎的 `build_main_actions` 中 `cost_payment` 也是 `vec![]`（`phase.rs:458`），且打出卡牌时"简化：不做费用检查"
- 所以当前这个硬编码不影响现有对局，但会阻塞后续费用系统的完整实现

### 修复方案

#### 方案 A：扩展 ClientMessage 支持费用支付（推荐）

**步骤 1**：修改 `ClientAction::PlayCard`，增加 `cost_payment` 字段
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action_type")]
pub enum ClientAction {
    // ...
    PlayCard {
        instance_id: u32,
        target_zone: ClientZone,
        cost_cards: Vec<u32>, // 新增：用于支付的 cost zone 卡牌 instance IDs
    },
    // ...
}
```

**步骤 2**：修改 `convert_client_action`
```rust
ClientAction::PlayCard { instance_id, target_zone, cost_cards } => {
    let zone = match target_zone { ... };
    let cost_payment: Vec<InstanceId> = cost_cards.iter().map(|c| InstanceId(*c)).collect();
    Ok(PhaseAction::PlayCard {
        instance_id: InstanceId(instance_id),
        target_zone: zone,
        cost_payment,
    })
}
```

**步骤 3**：Godot 客户端修改
`godot-client/` 中发送 `play_card` action 的地方需要增加 `cost_cards` 字段。目前搜索发现 Godot 客户端中**没有实际发送 play_card action 的代码**（debug.gd 中自动 pass，game_board 中未实现出牌按钮）。所以**暂时不需要修改 Godot 代码**，但需要在 PROTOCOL.md 中记录这个字段。

**步骤 4**：在 `build_main_actions` 中增加费用检查
当前 `phase.rs::build_main_actions` 已经硬编码 `cost_payment: vec![]`。如果要完整实现费用系统，这里需要根据卡牌费用和玩家手牌/费用区生成合法的 `cost_payment` 组合。但这是一个更大的功能（完整的费用系统），**不在本次修复范围内**。

### 验证集

#### 单元测试 1：带费用的 PlayCard 转换
```rust
#[test]
fn convert_client_action_with_cost_payment() {
    let action = ClientAction::PlayCard {
        instance_id: 42,
        target_zone: ClientZone::Front { slot: 2 },
        cost_cards: vec![10, 11],
    };
    let phase_action = convert_client_action(action).unwrap();
    match phase_action {
        PhaseAction::PlayCard { instance_id, target_zone, cost_payment } => {
            assert_eq!(instance_id, InstanceId(42));
            assert_eq!(target_zone, Zone::Front(2));
            assert_eq!(cost_payment, vec![InstanceId(10), InstanceId(11)]);
        }
        _ => panic!("Expected PlayCard"),
    }
}
```

#### 单元测试 2：不带费用的 PlayCard 兼容旧格式
由于 `serde` 的 `Deserialize` 对缺失字段的处理，如果直接增加 `cost_cards` 字段，旧客户端（不含此字段）发送的消息会反序列化失败。

解决方案：使用 `#[serde(default)]`
```rust
PlayCard {
    instance_id: u32,
    target_zone: ClientZone,
    #[serde(default)]
    cost_cards: Vec<u32>,
}
```

验证：
```rust
#[test]
fn client_action_backward_compatible() {
    let json = r#"{"action_type":"play_card","instance_id":42,"target_zone":{"zone_type":"front","slot":2}}"#;
    let action: ClientAction = serde_json::from_str(json).unwrap();
    match action {
        ClientAction::PlayCard { cost_cards, .. } => {
            assert!(cost_cards.is_empty());
        }
        _ => panic!("Expected PlayCard"),
    }
}
```

#### 集成测试 3：端到端带费用出牌
（在费用系统完整实现后再做，当前仅保证协议层兼容）

---

## 问题 6：同步/异步桥接的健壮性问题

### 问题描述
1. **断线不唤醒 Condvar**：玩家断线时，`handle_disconnect` 仅记录日志，没有主动通知 `ActionRequestState` 的 Condvar。Sync 线程（`GameEngine`）会继续阻塞等待，直到 `operation_timeout`（默认可能 30-60 秒）超时后才以 `Pass` 继续。
2. **重连无法接续**：WebSocket 断线后重新连接是一个新的 `handle_client` 任务，会分配新的 `mpsc channel`（`room_rx`），但 `Room` 中的 `action_states` 仍绑定旧的 WebSocket 连接。新连接可以加入房间、查询状态，但无法响应正在等待的 action 请求。
3. **ActionRequestState 的 Mutex 在跨线程使用时可能 panic**：`ActionRequestState` 使用 `std::sync::Mutex`，如果在 `wait_timeout_while` 期间被其他逻辑错误地多次 `notify`，目前不会 panic，但 `respond` 在已响应后返回 `false` 是安全的。

### 修复方案

#### 子问题 6a：断线时主动唤醒等待中的 ActionRequestState

**修改**：`crates/card-server/src/room.rs::Room::handle_disconnect`
```rust
pub async fn handle_disconnect(&mut self, player_id: PlayerId) {
    let slot = match player_id { ... };
    
    if let Some(ref mut player_slot) = self.players[slot] {
        player_slot.connected = false;
        // ... existing logging ...
    }
    
    // 新增：如果有待处理的 action 请求，发送一个虚拟的 Pass 响应
    // 让 GameEngine 继续运行，而不是挂起等待超时
    if let Some(ref action_state) = self.action_states[slot] {
        let _ = action_state.respond(PlayerAction::PhaseAction(PhaseAction::Pass));
        // 或使用一个特殊的 Disconnect 响应类型
    }
    
    // 广播断线事件
    let _ = self.broadcast(RoomMessage::PlayerDisconnected { ... });
    
    // 如果游戏进行中，结束游戏
    if self.state == RoomState::Playing {
        // ... existing game over broadcast ...
    }
}
```

**注意**：这里直接用 `Pass` 作为断线的 fallback 行为。更精确的做法是定义一个特殊的 `PlayerAction::Disconnected` 变体，让 `NetworkPhaseClient::choose_action` 识别并返回 `None`，从而触发引擎的 `GameOver::Timeout`。但这需要修改 `PlayerAction` 枚举和 `PhaseClient` 的交互逻辑。

**权衡**：简单做法（发送 Pass）会让游戏继续运行几回合直到 DeckOut 或其他结束条件；精确做法（返回 None 触发 GameOver）更符合预期。推荐**精确做法**：

```rust
// room.rs 中 PlayerAction 新增
pub enum PlayerAction {
    PhaseAction(PhaseAction),
    RecoverySelection(Vec<InstanceId>),
    Disconnected,  // 新增
}

// game_starter.rs 中 NetworkPhaseClient::choose_action
match self.action_state.wait_response(timeout) {
    Some(PlayerAction::PhaseAction(phase_action)) => Some(phase_action),
    Some(PlayerAction::Disconnected) => {
        println!("[NetworkPhaseClient] Player {:?} disconnected during action request", self.player_id);
        None  // 返回 None 让引擎判定为 Timeout/GameOver
    }
    // ...
}
```

但 `PhaseClient::choose_action` 返回 `None` 时，引擎的 `run_main_phase` 会 `push CoreGameEvent::GameOver { winner: opponent, reason: Timeout }`。这个行为是准确的——断线相当于超时判负。

#### 子问题 6b：支持断线重连

这是一个更复杂的功能，涉及：
1. 为每个 `PlayerSlot` 分配持久化的 `player_token`（断线重连凭证）
2. 新连接携带 `player_token` 而非仅 `player_name`
3. 重连时复用现有的 `action_states` 和 `room_rx`
4. 发送当前完整游戏状态给重连客户端

**建议**：这个问题超出"数据交互修复"的范围，属于"断线重连功能"。在本文档中**只做标注，不给出完整修复方案**，因为：
- 需要修改客户端协议（`join_room` 增加 token 字段）
- 需要修改 Godot 客户端的重连逻辑
- 需要设计 token 生成和过期机制

### 验证集

#### 单元测试 1：ActionRequestState 支持 Disconnected 响应
```rust
#[test]
fn action_request_state_disconnected_response() {
    let state = ActionRequestState::new();
    assert!(state.respond(PlayerAction::Disconnected));
    let response = state.wait_response(Duration::from_secs(1));
    assert!(matches!(response, Some(PlayerAction::Disconnected)));
}
```

#### 单元测试 2：handle_disconnect 唤醒等待中的 action
```rust
// 在 room.rs 的 tests 模块中
#[tokio::test]
async fn test_disconnect_wakes_pending_action() {
    let (tx, _rx) = mpsc::unbounded_channel();
    let mut room = Room::new("test".to_string(), GameRules::default(), tx);
    
    room.add_player("Alice".to_string()).await.unwrap();
    room.add_player("Bob".to_string()).await.unwrap();
    
    // 模拟游戏开始后的 action state
    let action_state = Arc::new(ActionRequestState::new());
    room.action_states[0] = Some(action_state.clone());
    room.state = RoomState::Playing;
    
    // 在另一个线程中模拟等待
    let state_clone = action_state.clone();
    let handle = std::thread::spawn(move || {
        state_clone.wait_response(Duration::from_secs(10))
    });
    
    // 模拟断线
    tokio::time::sleep(Duration::from_millis(50)).await;
    room.handle_disconnect(PlayerId::Player1).await;
    
    // 等待线程应在超时前返回
    let result = handle.join().unwrap();
    assert!(result.is_some(), "Action request should be resolved by disconnect");
    
    // 验证房间已广播 GameOver
}
```

#### 集成测试 3：WebSocket 断线导致 GameOver
在 `room_websocket_test.rs` 的 `test_player_disconnect` 中（已有此测试，需扩展）：
```rust
// 游戏开始后断开 client1
// 验证 client2 收到 GameOver，且 winner 为 Player2
let msg = recv_message(&mut client2).await;
match msg {
    TestServerMessage::GameOver { winner, reason } => {
        assert_eq!(winner, Some("Player2".to_string()));
        assert!(reason.contains("disconnected") || reason.contains("timeout"));
    }
    _ => panic!("Expected GameOver after disconnect, got {:?}", msg),
}
```

---

## 问题 7：Raw TCP 与 WebSocket 的状态机不统一

### 问题描述
- WebSocket 路径：`Room` 管理状态（Waiting → DeckSubmission → Playing → Finished）
- Raw TCP 路径：`GameSession` 管理状态（WaitingForPlayers → DeckSubmission → LoadingScripts → Playing → Finished）
- 两者独立维护，代码重复，行为不一致

### 影响
- 维护成本高：修改游戏规则或生命周期需要同时改两套代码
- 行为不一致：WebSocket 路径没有 "LoadingScripts" 阶段（直接在 `game_starter` 中加载），Raw TCP 路径在 `GameSession` 中加载
- 测试需要分别覆盖两套逻辑

### 修复方案

#### 方案：提取统一的游戏生命周期管理器（长期重构）

这个问题属于**架构层面**的债务，不是单个 bug。推荐的做法是：

1. **短期（保持现状，局部对齐）**：
   - 在 `Room::set_ready` 中启动游戏前，明确设置 `RoomState::LoadingScripts`（或等效状态）
   - 在 `game_starter::start_game` 的脚本加载阶段广播一个 `RoomMessage::LoadingScripts`（可选），让客户端知道"正在加载"
   - 统一 `RoomState` 和 `SessionPhase` 的命名

2. **长期（统一入口）**：
   - 将 `GameSession` 提取为通用的游戏生命周期管理器
   - `WebSocketServer` 在双方就绪后，不再直接调用 `game_starter::start_game`，而是创建 `GameSession`，注入 `NetworkPhaseClient` 作为 `PhaseClient`
   - `GameServer`（Raw TCP）也使用同一个 `GameSession`
   - 移除 `Room` 中的游戏内状态管理（`action_states` 等），全部交给 `GameSession`

由于这个重构涉及面广（`card-server` 的多个模块 + 可能的协议变更），**建议作为独立的技术债务项单独排期**，不在本次数据交互修复中实施。

### 验证集
- 无（架构重构的验证需要完整的集成测试套件覆盖两条路径）

---

## 问题 8：scripts_json 路径扫描重复实现

### 问题描述
`game_starter.rs` 中同时存在：
1. 被注释/绕过的 `ScriptIndex::scan` 调用
2. 手写的 `scan_dir_recursive` 函数（约 20 行）
3. 额外的 `find_scripts_path` 函数（约 50 行，尝试多个路径）
4. 大量的 `println!` 调试输出（`[GAME_STARTER] ...`）

### 影响
- 代码冗余，可读性差
- 测试环境与运行时的脚本扫描行为可能不一致（`CARGO_MANIFEST_DIR` 仅在 cargo 构建时存在）
- 大量调试 `println!` 污染日志

### 修复方案

#### 步骤 1：统一使用 card-script 的 ScriptIndex
`card-script` crate 提供了 `ScriptIndex::scan`，且已有测试覆盖。应恢复使用标准 API：

```rust
use card_script::loader::ScriptIndex;

let scripts_path = find_scripts_path();
let index = ScriptIndex::scan(&scripts_path)
    .map_err(|e| anyhow!("Failed to scan scripts: {}", e))?;
let loader = ScriptLoader::new(index);
let registry = loader.load_for_game(&deck1, &deck2)
    .map_err(|e| anyhow!("Failed to load registry: {}", e))?;
```

如果 `ScriptIndex::scan` 有已知问题导致之前被绕过，应**修复 `ScriptIndex` 本身**，而非在调用方重写扫描逻辑。

#### 步骤 2：检查 ScriptIndex::scan 的问题
查看 `card-script/src/loader.rs` 或相关文件中 `ScriptIndex::scan` 的实现，确认它是否能正确递归扫描 `scripts_json/` 目录下的 `.json` 文件。

如果 `ScriptIndex::scan` 的问题是路径格式不匹配（如它期望 `scripts/` 而非 `scripts_json/`），可以在 `find_scripts_path` 中优先返回 `scripts_json`，然后让 `ScriptIndex::scan` 处理。

#### 步骤 3：移除手写扫描和调试输出
- 删除 `scan_dir_recursive` 函数
- 删除 `find_scripts_path` 中的大量 `println!`（保留 `info!` 日志）
- 删除第 117-142 行的"Manual scan test"调试块

#### 步骤 4：增强 find_scripts_path 的健壮性
```rust
fn find_scripts_path() -> std::path::PathBuf {
    // 优先当前目录
    let candidates = [
        std::path::PathBuf::from("scripts_json"),
        std::path::PathBuf::from("scripts"),
    ];
    for path in &candidates {
        if path.exists() && path.is_dir() {
            return path.clone();
        }
    }
    
    // 回退到 workspace 根目录
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest = std::path::PathBuf::from(manifest_dir);
        if let Some(workspace) = manifest.parent().and_then(|p| p.parent()) {
            for subdir in &["scripts_json", "scripts"] {
                let path = workspace.join(subdir);
                if path.exists() {
                    return path;
                }
            }
        }
    }
    
    // 最终回退
    std::path::PathBuf::from("scripts_json")
}
```

### 验证集

#### 单元测试 1：ScriptIndex 能正确扫描 scripts_json
```rust
// 在 card-script 的 tests 中（如不存在则新建）
#[test]
fn script_index_scans_json_files() {
    let index = ScriptIndex::scan(Path::new("scripts_json")).unwrap();
    assert!(!index.is_empty(), "ScriptIndex should find JSON card definitions");
}
```

#### 单元测试 2：find_scripts_path 回退逻辑
```rust
#[test]
fn find_scripts_path_returns_existing_dir() {
    let path = find_scripts_path();
    assert!(path.exists() || std::env::var("CARGO_MANIFEST_DIR").is_ok(),
        "Should find scripts_json or fallback gracefully in test environment");
}
```

#### 集成测试 3：游戏启动使用标准加载流程
在 `room_websocket_test.rs` 的 `test_complete_game_flow` 中，游戏成功启动即证明 `ScriptIndex::scan` + `ScriptLoader::load_for_game` 工作正常。如果修复后测试仍然通过，说明标准流程可用。

---

## 附录：修复优先级与依赖关系

```
高优先级（阻塞游戏运行）:
  问题 1: 缺少持续状态更新 ──→ 影响 Godot 客户端整个游戏过程
  问题 4: GameStarted 状态泄露 ──→ 影响信息隐藏，有安全隐患
  问题 6a: 断线不唤醒 Condvar ──→ 导致游戏挂起

中优先级（影响体验）:
  问题 2: ActionRequested 空列表 ──→ 客户端不知道能做什么
  问题 3: RecoveryRequested 空列表 ──→ 同上
  问题 5: cost_payment 硬编码 ──→ 阻塞费用系统完整实现

低优先级（技术债务）:
  问题 7: 状态机不统一 ──→ 长期架构重构
  问题 8: 路径扫描重复 ──→ 代码清理
```

**依赖关系**：
- 问题 2 和问题 3 可以独立修复，互不影响
- 问题 1 依赖于 `PhaseClient` trait 的修改（如选择方案 A）
- 问题 4 可以独立修复
- 问题 5 需要协议变更（`ClientAction`），但向后兼容
- 问题 6a 需要 `PlayerAction` 枚举扩展
- 问题 7 和问题 8 是独立的
