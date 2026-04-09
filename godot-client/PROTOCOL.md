# Godot客户端通信协议

WebSocket服务器地址: `ws://host:port`

## 客户端发送的消息

### 加入房间
```json
{
  "type": "join_room",
  "room_id": "room1",
  "player_name": "Alice"
}
```

### 提交卡组
```json
{
  "type": "submit_deck",
  "cards": ["S000-C-001", "S000-C-002", "S000-S-001"]
}
```

### 发送行动
```json
{
  "type": "action",
  "action": {
    "action_type": "pass"
  }
}
```

```json
{
  "type": "action",
  "action": {
    "action_type": "surrender"
  }
}
```

```json
{
  "type": "action",
  "action": {
    "action_type": "play_card",
    "instance_id": 123,
    "target_zone": {
      "zone_type": "front",
      "slot": 0
    }
  }
}
```

```json
{
  "type": "action",
  "action": {
    "action_type": "declare_attack",
    "attacker_id": 123,
    "target": {
      "target_type": "direct"
    }
  }
}
```

```json
{
  "type": "action",
  "action": {
    "action_type": "declare_attack",
    "attacker_id": 123,
    "target": {
      "target_type": "slot",
      "slot_index": 2
    }
  }
}
```

### 恢复选卡
```json
{
  "type": "recovery",
  "cards": [1, 2, 3]
}
```

## 服务器发送的消息

### 加入成功
```json
{
  "type": "joined",
  "player_id": "Player1",
  "room_state": "Waiting"
}
```

### 对手加入
```json
{
  "type": "opponent_joined",
  "player_name": "Bob"
}
```

### 等待卡组提交
```json
{
  "type": "waiting_for_deck"
}
```

### 游戏开始
```json
{
  "type": "game_started",
  "state": {
    "turn_number": 1,
    "current_phase": "TurnStart",
    "current_player": "Player1",
    "your_state": {
      "hp": 5,
      "real_point": 0,
      "deck_count": 3,
      "hand": [],
      "front": [null, null, null, null, null],
      "back": [null, null, null, null, null],
      "cost_zone": [],
      "grave": []
    },
    "opponent_state": {
      "hp": 5,
      "real_point": 0,
      "deck_count": 3,
      "hand_count": 0,
      "front": [null, null, null, null, null],
      "back": [null, null, null, null, null],
      "cost_zone": [],
      "grave": []
    }
  }
}
```

### 状态更新
```json
{
  "type": "state_update",
  "state": { ... }
}
```

### 请求行动
```json
{
  "type": "action_request",
  "available_actions": ["Pass", "Surrender", "PlayCard"],
  "timeout_secs": 60
}
```

### 请求恢复选卡
```json
{
  "type": "recovery_request",
  "count": 2,
  "options": ["card1", "card2"]
}
```

### 对手断线
```json
{
  "type": "player_disconnected",
  "player_name": "Alice"
}
```

### 游戏结束
```json
{
  "type": "game_over",
  "winner": "Player1",
  "reason": "Player Player2 disconnected"
}
```

### 错误
```json
{
  "type": "error",
  "message": "Room not found"
}
```

## 游戏流程

1. 连接WebSocket
2. 发送 `join_room` 加入房间
3. 等待 `opponent_joined` （对手加入）
4. 收到 `waiting_for_deck` 后发送 `submit_deck`
5. 双方提交卡组后收到 `game_started`
6. 收到 `action_request` 时发送 `action`
7. 游戏结束收到 `game_over`

## 断线处理

- 如果对手断线，会收到 `player_disconnected` 和 `game_over`
- 如果自己断线，需要重新连接并重新加入房间
