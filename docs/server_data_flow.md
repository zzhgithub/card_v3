# Rust Server 数据交互全景分析

> **说明**：本文档仅做代码分析记录，不修改任何代码。按时间顺序梳理从服务器启动到游戏结束的完整数据交互流程。

---

## 1. 架构总览

`card-server` 存在**两条并行的网络入口**，分别使用不同的协议与数据格式：

| 路径 | 入口 | 协议 | 端口 | 用途 |
|------|------|------|------|------|
| **WebSocket Room 路径** (当前主要入口) | `main.rs` → `WebSocketServer` | JSON over WebSocket | 8080 | Godot 客户端、TUI 匹配对战 |
| **Raw TCP 路径** (传统入口) | `network.rs` → `GameServer` | bincode over TCP (长度前缀帧) | 动态 | 直接 P2P 对战、TUI 直连 |

两条路径最终都汇入 `card-core::GameEngine`，但**桥接方式完全不同**。

---

## 2. 阶段一：服务器启动

### 2.1 WebSocket Room 服务器启动
**文件**: `crates/card-server/src/main.rs`

```
main()
  └── 创建 GameRules::default()
  └── 创建 RoomManager::new(rules)        // Arc<RwLock<HashMap<room_id, Arc<Mutex<Room>>>>>
  └── 创建 WebSocketServer::new(room_manager, "127.0.0.1:8080")
  └── server.run().await                   // TcpListener::bind + accept 循环
```

**数据**: 无外部交互，纯内部初始化。

### 2.2 Raw TCP 服务器启动（如被使用）
**文件**: `crates/card-server/src/network.rs`

```
GameServer::new(bind_addr, script_index, rules, version, rng_seed)
  └── 等待后续显式调用 .start().await
```

---

## 3. 阶段二：WebSocket 房间生命周期（当前主路径）

### 3.1 客户端连接与房间加入
**文件**: `crates/card-server/src/websocket_server.rs` (`handle_client`)

**交互时序**:

```
Client ──WebSocket──> Server
  {"type": "join_room", "room_id": "xxx", "player_name": "Alice"}

Server ──WebSocket──> Client (Alice)
  {"type": "joined", "player_id": "Player1", "room_state": "DeckSubmission"}
  {"type": "room_state", "players": [...], "all_ready": false}

Server ──broadcast──> Client (Bob, 如已在房间)
  {"type": "opponent_joined", "player_name": "Alice"}
```

**内部数据流**:
```
handle_client()
  └── ClientMessage::JoinRoom
      └── room_manager.join_room(room_id, player_name)
          └── Room::add_player()
              ├── 分配 slot 0 → Player1, slot 1 → Player2
              ├── broadcast RoomMessage::PlayerJoined
              └── 若 slot == 1 (满员): 状态变为 DeckSubmission, broadcast WaitingForDecks
      └── state.room_rx = room_manager.subscribe(room_id)   // mpsc channel
      └── ws_tx.send(ServerMessage::Joined)
      └── ws_tx.send(ServerMessage::RoomState)
```

**关键类型**:
- `ClientMessage::JoinRoom { room_id, player_name }`
- `ServerMessage::Joined { player_id, room_state }`
- `ServerMessage::RoomState { players, all_ready }`
- `RoomMessage::PlayerJoined { player_id, player_name }`
- `RoomMessage::WaitingForDecks`

### 3.2 卡组提交与就绪

**交互时序**:

```
Client ──WebSocket──> Server
  {"type": "submit_deck", "deck_id": "MyDeck", "cards": ["S000-C-001", ...]}

Server ──broadcast──> All Clients in Room
  {"type": "player_ready", "player_name": "Alice", "deck_id": "MyDeck"}

// 当两名玩家都 ready 后:
Server ──broadcast──> All Clients
  {"type": "game_starting"}
  // 同时后台 spawn game_starter::start_game()
```

**内部数据流**:
```
handle_client()
  └── ClientMessage::SubmitDeck { deck_id, cards }
      └── room_manager.submit_deck(room_id, player_id, card_ids, deck_id)
          └── Room::submit_deck()      // 存储 deck 到 room.decks[slot]
          └── Room::set_ready()
              ├── ready_state[slot] = Some(deck_id)
              ├── broadcast RoomMessage::PlayerReady
              └── 若双方 ready:
                  ├── broadcast RoomMessage::GameStarting
                  └── tokio::spawn(crate::game_starter::start_game(room_clone, rules))
```

### 3.3 取消就绪 / 离开房间

```
Client ──WebSocket──> Server
  {"type": "unready"}

Server ──broadcast──> All Clients
  {"type": "player_unready", "player_name": "Alice"}
```

```
Client ──WebSocket──> Server
  {"type": "leave_room"}

Server ──WebSocket──> Client
  {"type": "left"}

Server ──broadcast──> Opponent
  {"type": "player_disconnected", "player_name": "Alice"}
  // 若游戏进行中，同时 broadcast GameOver
```

### 3.4 断线处理

```
WebSocket 连接断开
  └── handle_client 退出前:
      └── room_manager.player_disconnected(room_id, player_id)
          └── Room::handle_disconnect()
              ├── player.connected = false
              ├── broadcast RoomMessage::PlayerDisconnected
              └── 若 Playing: broadcast RoomMessage::GameOver (对方获胜)
```

---

## 4. 阶段三：游戏启动（game_starter）

**文件**: `crates/card-server/src/game_starter.rs`

当 `Room::set_ready()` 检测到双方就绪后，会 `tokio::spawn` 调用 `start_game()`。

### 4.1 启动前准备（async 侧）

```
start_game(Arc<Mutex<Room>>, GameRules)
  └── 等待 decks[0] 和 decks[1] 非空（轮询，30秒超时）
  └── find_scripts_path()                     // 查找 scripts_json/ 目录
  └── 手动递归扫描目录，建立 CardId -> PathBuf 索引
  └── load_registry_for_game(index, deck1, deck2)
      └── 对每个 CardId 调用 load_card_from_json(path)
          └── 解析 JSON: id, name, card_type, property, category, cost, attack, effects...
  └── validate_deck(deck1, rules, registry)   // Player 1 卡组校验
  └── validate_deck(deck2, rules, registry)   // Player 2 卡组校验
  └── GameState::new(rules, rand::random())   // 创建游戏状态
  └── 填充 deck: CardInstance::new(InstanceId, CardId, base_attack)
  └── 抽取初始手牌 (initial_hand_size)
  └── 创建 ActionRequestState × 2            // Condvar 桥接器
  └── 将 action_states 存入 Room
```

### 4.2 初始状态广播

```
  └── build_visible_state(&state, PlayerId::Player1)   // 玩家1视角（含手牌详情）
  └── build_visible_state(&state, PlayerId::Player2)   // 玩家2视角（对手手牌仅 count）
  └── room.broadcast_message(RoomMessage::GameStarted { state: visible_p1 })
  └── room.broadcast_message(RoomMessage::StateUpdate { for_player: P1, state: visible_p1 })
  └── room.broadcast_message(RoomMessage::StateUpdate { for_player: P2, state: visible_p2 })
```

**⚠️ 注意**: `GameStarted` 的消息体只包含 P1 的视角（代码里传入 visible_p1），而 `StateUpdate` 通过 `for_player` 字段做了玩家区分。在 `convert_room_message` 中，`StateUpdate` 会检查 `my_player_id == for_player`，匹配才发送真实状态，否则发送 `Value::Null`。

### 4.3 引擎启动

```
  └── 创建 NetworkPhaseClient::new(P1, broadcast_tx, action_state1)
  └── 创建 NetworkPhaseClient::new(P2, broadcast_tx, action_state2)
  └── tokio::task::spawn_blocking(move || {
          let engine = GameEngine::new(state, registry, Box::new(client1), Box::new(client2));
          engine.run()                          // 同步阻塞运行!
      })
```

**关键类型**:
- `VisibleGameState` { turn_number, current_phase, current_player, your_state, opponent_state }
- `PlayerVisibleState` { hp, real_point, deck_count, hand, front[5], back[5], cost_zone, grave }
- `OpponentVisibleState` { hp, real_point, deck_count, hand_count, front[5], back[5], cost_zone, grave }
- `CardVisibleInfo` { instance_id, definition_id, current_attack }

---

## 5. 阶段四：游戏进行中的核心交互（Sync/Async 桥接）

这是**最复杂**的阶段。`GameEngine` 是**同步阻塞**的（运行在 `spawn_blocking` 中），而客户端通信是**异步**的（WebSocket/tokio）。桥接通过 `ActionRequestState`（`Mutex + Condvar`）实现。

### 5.1 请求玩家行动（choose_action）

**文件**: `crates/card-server/src/game_starter.rs` (`NetworkPhaseClient`)

```
GameEngine (sync, spawn_blocking 线程)
  └── PhaseRunner::run_turn()
      └── client1.choose_action(available_actions, timeout)
          └── NetworkPhaseClient::choose_action()
              ├── action_state.reset()
              ├── broadcast_tx.send(RoomMessage::ActionRequested {
              │       player_id: P1,
              │       available_actions: vec!["..."],   // 用 {:?} 格式化的字符串
              │       timeout_secs
              │   })
              └── action_state.wait_response(timeout)   // Condvar 阻塞等待!

                  // 此时 sync 线程阻塞，等待 async WebSocket 层喂数据
```

**WebSocket 侧（async）**:

```
Client ──WebSocket──> Server
  {"type": "action", "action": {"action_type": "play_card", "instance_id": 1, "target_zone": {...}}}

handle_client()
  └── ClientMessage::Action { action }
      └── convert_client_action(action) -> PhaseAction
      └── room_manager.submit_action(room_id, player_id, PlayerAction::PhaseAction(phase_action))
          └── Room::send_action()
              └── action_state.respond(PlayerAction::PhaseAction(...))
                  ├── *response = Some(action)
                  └── condvar.notify_one()       // 唤醒 sync 线程!
```

**同步线程被唤醒后**:
```
action_state.wait_response() 返回 Some(PlayerAction::PhaseAction(phase_action))
  └── NetworkPhaseClient 将 PhaseAction 返回给 GameEngine
      └── GameEngine 继续执行
```

### 5.2 请求回收卡牌（choose_recovery_cards）

与 `choose_action` 结构完全一致，只是消息类型不同：

```
NetworkPhaseClient::choose_recovery_cards()
  ├── action_state.reset()
  ├── broadcast RoomMessage::RecoveryRequested { player_id, count, options }
  └── action_state.wait_response(timeout)      // Condvar 阻塞

Client ──WebSocket──> Server
  {"type": "recovery", "cards": [1, 2, 3]}

handle_client()
  └── ClientMessage::Recovery { cards }
      └── room_manager.submit_action(room_id, player_id, PlayerAction::RecoverySelection(instance_ids))
          └── action_state.respond(...)
              └── condvar.notify_one()
```

### 5.3 向客户端推送的消息映射

`RoomMessage` 在 `websocket_server.rs::convert_room_message()` 中被转换为 `ServerMessage`：

| RoomMessage (内部) | ServerMessage (JSON to Client) | 说明 |
|-------------------|-------------------------------|------|
| `PlayerJoined` | `OpponentJoined` | 对手加入 |
| `PlayerDisconnected` | `PlayerDisconnected` | 对手断线 |
| `PlayerReady` | `PlayerReady` | 某玩家就绪 |
| `PlayerUnready` | `PlayerUnready` | 某玩家取消就绪 |
| `WaitingForDecks` | `WaitingForDeck` | 等待提交卡组 |
| `GameStarting` | `GameStarting` | 游戏即将开始 |
| `GameStarted { state }` | `GameStarted { state }` | 游戏开始（全量状态）|
| `StateUpdate { for_player, state }` | `StateUpdate { state }` 或 `Null` | 状态更新 |
| `ActionRequested { player_id, ... }` | `ActionRequest { available_actions, timeout_secs }` 或 `Null` | 请求行动 |
| `RecoveryRequested { player_id, ... }` | `RecoveryRequest { count, options }` 或 `Null` | 请求回收 |
| `GameOver { winner, reason }` | `GameOver { winner, reason }` | 游戏结束 |
| `Error { message }` | `Error { message }` | 错误 |

**⚠️ 关键发现**: `convert_room_message` 对 `StateUpdate` / `ActionRequested` / `RecoveryRequested` 做了**玩家过滤**——只有 `my_player_id == for_player`（或 `player_id`）的客户端才会收到有效载荷，其他客户端收到 `{"type": "state_update", "state": null}`。这意味着对手看不到你的行动请求，也看不到你的状态细节。

---

## 6. 阶段五：Raw TCP 传统路径的数据交互

**文件**: `crates/card-server/src/network.rs`, `remote_client.rs`

这条路径不使用 Room/游戏启动器，而是直接 2 人 TCP 直连。

### 6.1 连接与握手

```
Server (GameServer::start)
  └── TcpListener::bind(bind_addr)
  └── accept_player(conn1, "1.0.0", P1)
  └── accept_player(conn2, "1.0.0", P2)

accept_player()
  └── TcpConnection::from_stream(stream)
  └── perform_handshake()
```

**握手时序**:

```
Server ──TCP(bincode)──> Client
  NetworkMessage::Hello { version: "1.0.0", player_name: "Server" }

Client ──TCP(bincode)──> Server
  NetworkMessage::Hello { version: "1.0.0", player_name: "Alice" }

Server 校验 version，不匹配则发送 Disconnect

Server ──TCP(bincode)──> Client
  NetworkMessage::HelloAck { player_id: Player1 }
```

### 6.2 卡组提交与校验

```
Server ──TCP──> Client (async recv)
  NetworkMessage::DeckSubmit { card_ids }

Server 调用 ScriptLoader + validate_deck()

// 若校验失败:
Server ──TCP──> 失败方
  NetworkMessage::DeckRejected { reason }
Server ──TCP──> 对方
  NetworkMessage::Disconnect { reason: "opponent deck invalid" }

// 若校验通过:
Server ──TCP──> Both
  NetworkMessage::DeckAccepted
```

### 6.3 游戏开始

```
Server ──TCP──> Both
  NetworkMessage::GameStart { rules_json: "..." }

Server 创建:
  - RemoteClient::new(conn1, P1)     // 实现 ClientApi
  - RemoteClient::new(conn2, P2)
  - RemotePhaseClient::new(client1, runtime)   // 实现 PhaseClient
  - RemotePhaseClient::new(client2, runtime)
  - GameSession::new(rules, script_index, rng_seed)
  - session.add_player(phase_client1, deck1)
  - session.add_player(phase_client2, deck2)

tokio::task::spawn_blocking(move || session.start())
```

### 6.4 游戏内请求/响应（Raw TCP）

`RemotePhaseClient` 把同步的 `PhaseClient` 调用桥接到异步的 `RemoteClient`：

```
GameEngine (sync, spawn_blocking)
  └── client1.choose_action(available, timeout)
      └── RemotePhaseClient::choose_action()
          ├── phase_actions_to_network(available) -> Vec<AvailableAction>
          ├── block_on(self.inner.choose_action(&network_actions, timeout))
          │   └── RemoteClient::choose_action() (async)
          │       ├── 发送 NetworkMessage::RequestAction { player, available_actions, timeout_secs }
          │       ├── timeout(op_timeout, conn.recv().await)
          │       └── 等待 Client 回复 NetworkMessage::CommandResponse { command }
          └── command_to_phase_action(command) -> PhaseAction
```

客户端侧（使用 `card-protocol` 的 TCP 连接）:

```
Server ──TCP(bincode)──> Client
  NetworkMessage::RequestAction {
      player: Player1,
      available_actions: [PlayCard { instance_id: ... }, ChainPass, ...],
      timeout_secs: 60
  }

Client ──TCP(bincode)──> Server
  NetworkMessage::CommandResponse {
      command: Command::PlayCard { instance_id, target_zone, cost_payment }
  }
```

其他请求同理：

| 服务器请求 | 客户端响应 | RemoteClient 方法 |
|-----------|-----------|------------------|
| `RequestAction` | `CommandResponse` | `choose_action()` |
| `RequestCardSelection` | `CardResponse` | `choose_cards()` |
| `RequestTargetSelection` | `TargetResponse` | `choose_targets()` |
| `EventNotification` | (无响应) | `on_event()` |

### 6.5 事件通知（Raw TCP 独有）

```
GameEngine 产生 CoreGameEvent
  └── (通过某种机制，如 ReplayRecorder 或事件回调)
  └── RemoteClient::on_event(event, visible_state)
      └── 发送 NetworkMessage::EventNotification { event }
```

**⚠️ 注意**: `EventNotification` 在 Raw TCP 路径中存在，但在 WebSocket Room 路径中**没有直接对应机制**——WebSocket 路径仅在游戏开始时发送一次 `StateUpdate`，之后没有持续的状态/事件推送。

---

## 7. 阶段六：游戏结束

### 7.1 WebSocket 路径

```
GameEngine.run() 返回 GameResult { winner, reason }
  └── spawn_blocking 任务结束
  └── broadcast_tx.send(RoomMessage::GameOver { winner, reason })
      └── convert_room_message() -> ServerMessage::GameOver
          └── {"type": "game_over", "winner": "Player1", "reason": "..."}
  └── room.state = RoomState::Finished
```

### 7.2 Raw TCP 路径

```
GameSession::start()
  └── engine.run()
  └── 返回 GameResult
  └── GameServer::start() 返回 result
  └── (可选) 发送 GameOver 事件给客户端
```

---

## 8. 核心数据类型速查

### 8.1 WebSocket Room 协议（JSON）

**Client → Server** (`ClientMessage`):
- `join_room { room_id, player_name }`
- `leave_room`
- `submit_deck { deck_id, cards: Vec<String> }`
- `unready`
- `action { action: ClientAction }`
- `recovery { cards: Vec<u32> }`  // Instance IDs
- `query_room_state`

**Server → Client** (`ServerMessage`):
- `joined { player_id, room_state }`
- `left`
- `player_ready { player_name, deck_id }`
- `player_unready { player_name }`
- `room_state { players, all_ready }`
- `waiting_for_deck`
- `game_starting`
- `game_started { state }`
- `state_update { state }`
- `action_request { available_actions, timeout_secs }`
- `recovery_request { count, options }`
- `game_over { winner, reason }`
- `error { message }`
- `opponent_joined { player_name }`
- `player_disconnected { player_name }`

### 8.2 Raw TCP 协议（bincode）

**NetworkMessage** (长度前缀 u32 + bincode payload):

| 分类 | 消息 |
|------|------|
| 握手 | `Hello { version, player_name }`, `HelloAck { player_id }` |
| 卡组 | `DeckSubmit { card_ids }`, `DeckAccepted`, `DeckRejected { reason }` |
| 开局 | `GameStart { rules_json }` |
| 服务器→客户端(游戏内) | `EventNotification { event }`, `RequestAction { player, available_actions, timeout_secs }`, `RequestTargetSelection { ... }`, `RequestCardSelection { ... }` |
| 客户端→服务器(游戏内) | `CommandResponse { command }`, `TargetResponse { targets }`, `CardResponse { instance_ids }` |
| 保活 | `Ping`, `Pong` |
| 断开 | `Disconnect { reason }` |

**Command** (客户端行动):
- `PlayCard { instance_id, target_zone, cost_payment }`
- `ActivateEffect { instance_id, effect_key, cost_payment }`
- `DeclareAttack { attacker, target }`
- `DirectAttackRealPoint { real_point_used }`
- `ChainActivate { instance_id, effect_key, cost_payment }`
- `ChainPass`
- `SelectRecoveryCards { instance_ids }`
- `SelectTargets { targets }`
- `SelectCards { instance_ids }`
- `Surrender`

### 8.3 内部桥接类型

| 类型 | 用途 | 位置 |
|------|------|------|
| `RoomMessage` | 房间内部广播消息 | `room.rs` |
| `PlayerAction` | WebSocket 玩家动作封装 | `room.rs` |
| `ActionRequestState` | `Mutex + Condvar` 同步/异步桥接 | `game_starter.rs` |
| `PhaseClient` trait | 引擎请求玩家决策的抽象 | `card-core` |
| `ClientApi` trait | 客户端接收事件/做出选择的抽象 | `card-client` |

---

## 9. 发现的关键问题（仅记录，未修复）

1. **WebSocket 路径缺少持续的状态更新**
   - `StateUpdate` 仅在 `game_starter::start_game()` 的初始阶段发送一次。
   - 游戏进行中的每次状态变更（抽卡、战斗、连锁结算等）**没有**通过 WebSocket 推送给客户端。
   - 对比 Raw TCP 路径：`RemoteClient::on_event()` 会发送 `EventNotification`。
   - 后果：WebSocket 客户端（Godot）在游戏开始后无法获知状态变化，除非自行推断。

2. **ActionRequested 的 available_actions 为空列表**
   - `convert_room_message()` 中对 `ActionRequested` 的处理：`available_actions: vec![]`（硬编码为空）。
   - 虽然 `RoomMessage::ActionRequested` 内部携带了 `available_actions`，但在转换为 `ServerMessage` 时被丢弃。
   - 后果：客户端收到 action_request 时不知道有哪些合法行动可选。

3. **RecoveryRequested 的 options 为空列表**
   - 与上一条类似，`options: vec![]` 被硬编码为空。
   - 后果：客户端收到 recovery_request 时不知道可选的卡牌有哪些。

4. **`GameStarted` 消息只广播 P1 视角的状态**
   - `game_starter.rs` 第 264 行：`RoomMessage::GameStarted { state: visible_p1.clone() }`
   - 虽然随后单独发送了针对 P1/P2 的 `StateUpdate`，但 `GameStarted` 本身只带 P1 视角。
   - `convert_room_message` 对 `GameStarted` 没有做玩家过滤，所以 P2 也会收到 P1 视角的状态。

5. **`cost_payment` 在 WebSocket 路径中被硬编码为空**
   - `convert_client_action()` 中 `PlayCard` 的 `cost_payment: vec![]`。
   - 注释写明 "Simplified - should calculate from client"。
   - 后果：费用支付逻辑可能不完整。

6. **同步/异步桥接的健壮性问题**
   - `ActionRequestState` 使用 `std::sync::Mutex + Condvar`，在 async 环境中被 `spawn_blocking` 持有。
   - 若客户端在请求期间断线，`handle_disconnect` 仅记录日志，**没有主动唤醒 Condvar**，依赖 30 秒超时。
   - 若玩家在超时前重新连接（WebSocket 重连），由于 `player_id` 和 `action_states` 的绑定关系，新连接可能无法接续之前的请求。

7. **Raw TCP 与 WebSocket 的状态机不统一**
   - WebSocket 路径使用 `RoomManager / Room` 管理状态机（Waiting → DeckSubmission → Playing → Finished）。
   - Raw TCP 路径使用 `GameSession` 管理状态机（WaitingForPlayers → DeckSubmission → LoadingScripts → Playing → Finished）。
   - 两套状态机代码独立维护，逻辑存在差异（如 WebSocket 路径没有 LoadingScripts 阶段）。

8. **`scripts_json` 路径扫描重复实现**
   - `game_starter.rs` 中同时存在 `ScriptIndex::scan`（被注释/绕过）和手写的 `scan_dir_recursive`。
   - 手写的扫描逻辑在测试和运行时的行为可能不一致。

---

## 10. 附录：文件职责对照

| 文件 | 职责 |
|------|------|
| `main.rs` | WebSocket 服务器入口，初始化 RoomManager |
| `websocket_server.rs` | WebSocket 连接管理、JSON 协议编解码、客户端消息分发 |
| `room.rs` | 房间状态机、玩家槽位管理、广播消息、ActionRequestState 注册 |
| `game_starter.rs` | 游戏启动流程（脚本加载、卡组校验、初始状态构建、sync/async 桥接） |
| `network.rs` | Raw TCP 服务器、握手、卡组接收、GameSession 编排 |
| `remote_client.rs` | Raw TCP 上的 `ClientApi` 实现（请求/响应 TCP 消息转换） |
| `session.rs` | 传统游戏会话生命周期（Waiting → Playing → Finished） |
| `ai_client.rs` | AI `PhaseClient` 实现（随机选择行动/回收） |
| `replay.rs` | 录像数据结构（ReplayData / ReplayRecorder） |
| `snapshot.rs` | 残局快照数据结构（GameSnapshot） |
| `card-protocol/message.rs` | Raw TCP 协议消息定义（NetworkMessage / Command / GameEvent） |
| `card-protocol/codec.rs` | TCP 长度前缀 + bincode 编解码器 |
