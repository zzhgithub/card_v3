# 游戏启动流程详解

## 完整流程图

```
┌─────────┐         ┌─────────┐
│ 玩家A   │         │ 玩家B   │
└────┬────┘         └────┬────┘
     │                   │
     │ 1. JoinRoom       │
     │──────────────────►│
     │◄──────────────────│ 2. JoinRoom
     │                   │
     │◄── 3. Joined {P1} │
     │                   │◄── 4. Joined {P2}
     │                   │
     │◄── 5. OpponentJoined{B}
     │                   │
     │◄── 6. WaitingForDecks
     │◄──────────────────►│
     │                   │
     │ 7. SubmitDeck     │
     │──────────────────►│
     │◄──────────────────│ 8. SubmitDeck
     │                   │
     │     [Server]      │
     │  ┌─────────────┐  │
     │  │ 检查卡组    │  │
     │  │ 加载卡片    │  │
     │  │ 创建状态    │  │
     │  │ 启动引擎    │  │
     │  └─────────────┘  │
     │                   │
     │◄── 9. GameStarted │
     │◄──────────────────►│  (各自收到不同视角的状态)
     │                   │
```

## 代码实现关键点

### 1. 触发游戏启动

```rust
// crates/card-server/src/game_starter.rs
pub async fn start_game(
    room: Arc<Mutex<Room>>,
    rules: GameRules,
) -> Result<()> {
    // 1. 等待双方提交卡组
    while room_guard.decks[0].is_none() || room_guard.decks[1].is_none() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // 2. 加载卡片定义
    let loader = ScriptLoader::new(index);
    let registry = loader.load_for_game(&deck1, &deck2)?;

    // 3. 验证卡组合法性
    validate_deck(&deck1, &rules, &registry)?;
    validate_deck(&deck2, &rules, &registry)?;

    // 4. 创建游戏状态
    let mut state = GameState::new(rules, rand::random());
    populate_decks(&mut state, &deck1, &deck2, &registry);

    // 5. 发送初始状态（带信息隐藏）
    let visible_p1 = build_visible_state(&state, PlayerId::Player1);
    let visible_p2 = build_visible_state(&state, PlayerId::Player2);
    
    broadcast_tx.send(RoomMessage::GameStarted { state: visible_p1 }); // 给P1
    broadcast_tx.send(RoomMessage::GameStarted { state: visible_p2 }); // 给P2

    // 6. 启动游戏引擎
    let engine = GameEngine::new(state, registry, client1, client2);
    engine.run()
}
```

### 2. 数据保密性实现

```rust
fn build_visible_state(state: &GameState, for_player: PlayerId) -> VisibleGameState {
    let me = &state.players[my_idx];
    let opponent = &state.players[opp_idx];

    VisibleGameState {
        your_state: PlayerVisibleState {
            hand: me.zones.hand.iter().map(...).collect(), // 完整手牌
            // ...
        },
        opponent_state: OpponentVisibleState {
            hand_count: opponent.zones.hand.len(), // 只有数量！
            // 没有 hand 字段
        },
    }
}
```

### 3. 消息类型定义

```rust
pub enum RoomMessage {
    // 房间管理
    PlayerJoined { player_id: PlayerId, player_name: String },
    WaitingForDecks,
    
    // 游戏状态（关键：每个玩家收到不同内容）
    GameStarted { state: VisibleGameState },
    StateUpdate { for_player: PlayerId, state: VisibleGameState },
    
    // 动作请求
    ActionRequested { 
        player_id: PlayerId, 
        available_actions: Vec<String>,
        timeout_secs: u64 
    },
    
    // 游戏结束
    GameOver { winner: Option<PlayerId>, reason: String },
}
```

## 客户端接收到的数据示例

### 玩家1收到的 GameStarted:
```json
{
  "type": "GameStarted",
  "state": {
    "turn_number": 1,
    "current_phase": "TurnStart",
    "current_player": "Player1",
    "your_state": {
      "hp": 5,
      "real_point": 0,
      "deck_count": 27,
      "hand": [              // <-- 能看到自己的手牌详情
        {"instance_id": 1, "definition_id": "S000-C-001", "current_attack": 1500},
        {"instance_id": 2, "definition_id": "S000-C-002", "current_attack": 800}
      ],
      "front": [null, null, null, null, null],
      "back": [null, null, null, null, null],
      "cost_zone": [],
      "grave": []
    },
    "opponent_state": {
      "hp": 5,
      "real_point": 0,
      "deck_count": 27,
      "hand_count": 2,      // <-- 只能看到对手手牌数量！
      "front": [null, null, null, null, null],
      "back": [null, null, null, null, null],
      "cost_zone": [],
      "grave": []
    }
  }
}
```

### 玩家2收到的 GameStarted:
```json
{
  "type": "GameStarted",
  "state": {
    "your_state": {
      "hand": [              // 玩家2看到自己的手牌
        {"instance_id": 1001, "definition_id": "S000-S-001"},
        {"instance_id": 1002, "definition_id": "S000-C-003"}
      ]
    },
    "opponent_state": {
      "hand_count": 2       // 只能看到玩家1的手牌数量
    }
  }
}
```

## 启动时机

当前简化实现中，需要在 `RoomManager::join_room` 中检测到第二个玩家加入时启动游戏：

```rust
pub async fn join_room(...) -> Result<RoomJoinResult> {
    // ...
    
    if result.slot_filled == 1 {  // 第二个玩家加入
        let room_clone = room.clone();
        let rules = self.rules.clone();
        
        // 启动游戏任务
        tokio::spawn(async move {
            if let Err(e) = game_starter::start_game(room_clone, rules).await {
                error!("Game start failed: {}", e);
            }
        });
    }
    
    Ok(result)
}
```

## 注意事项

1. **卡组提交**: 当前实现需要双方提交卡组后才能开始游戏
2. **错误处理**: 如果卡组加载失败或验证失败，游戏不会启动
3. **状态同步**: 每个操作后都会广播新的状态给双方（各自看到不同的视角）
4. **游戏结束**: GameEngine::run() 返回时游戏结束，会发送 GameOver 消息
