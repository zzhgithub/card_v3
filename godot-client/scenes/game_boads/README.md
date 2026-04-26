# 游戏界面设计

## 场景流程

1. 点击首页进入到联机游戏后场景，要输入房间ID和用户名，还有服务器地址ws://开头。有:Join按钮和返回按钮。
2. 输入全后进入到连接中页面。（转圈圈，超时30s后弹窗报错返回）
3. 连接成功后，进入房间等待场景。场景内显示显示玩家列表(可以看待其他玩家和自己是否准备)。有准备和取消按钮（一个按钮两个功能）。有一个卡组选择框，只能勾选一个卡组。勾线成功后，才能点击准备。准备时向ws发送确认卡组的消息。
4. 双方都准备后，进入到对局场景。
5. 对局场景：下方是自己的场地，上方是对手的场地。右侧显示：自己和对手的状态：Hp. RealPoint个数
6. 每个玩家的战场处显示，5个前场，5个后场。用全色背景。方形，有间距。前场是淡蓝色，后场是淡橙色。左侧还有卡组和墓地。在右侧（状态栏的左侧。）有竖直显示的费用区域。
7. 注意每个区域都是可以放置卡片对象的堆。要实现插件card_framework中的卡片容器的接口。并且每种类型创建单独的场景。要实现解耦。
8. 在下方的部分，显示的是手卡。可以复用card_framework中的手卡区域。最上方是对手的手卡。
9. 自己的手卡是卡图，对手的手卡是卡背。卡背适应images中的back.png图片，缺省情况下白色，有卡背字样。

---

# WebSocket 接口文档

## 连接信息

- **协议**: WebSocket (ws:// 或 wss://)
- **默认地址**: `ws://localhost:8080/ws`
- **消息格式**: JSON

## 客户端 → 服务器消息

所有消息都包含 `type` 字段用于区分消息类型。

### 1. join_room - 加入房间

玩家加入指定房间。如果房间不存在则自动创建。

```json
{
  "type": "join_room",
  "room_id": "房间ID",
  "player_name": "玩家名称"
}
```

### 2. leave_room - 离开房间

玩家主动离开当前房间。

```json
{
  "type": "leave_room"
}
```

### 3. submit_deck - 提交卡组（准备）

玩家选择卡组并准备。包含卡组ID和卡片列表。

```json
{
  "type": "submit_deck",
  "deck_id": "卡组名称",
  "cards": ["卡片ID1", "卡片ID2", ...]
}
```

**说明**:
- `deck_id`: 卡组名称（如 "Example", "NewDeck1"）
- `cards`: 卡片ID字符串数组（如 `["S000-C-001", "S000-C-002"]`）
- 服务器收到后会将该玩家标记为"已准备"
- 当两名玩家都准备后，游戏自动开始

### 4. unready - 取消准备

玩家取消准备状态。

```json
{
  "type": "unready"
}
```

### 5. query_room_state - 查询房间状态

查询当前房间内所有玩家的状态（用于轮询更新）。

```json
{
  "type": "query_room_state"
}
```

### 6. action - 游戏动作

游戏进行中的动作（游戏开始后才可用）。

```json
{
  "type": "action",
  "action": {
	"action_type": "play_card|declare_attack|pass|surrender",
	...
  }
}
```

### 7. recovery - 复苏选择

选择复苏的卡片。

```json
{
  "type": "recovery",
  "cards": [1, 2, 3]
}
```

---

## 服务器 → 客户端消息

### 1. joined - 成功加入房间

```json
{
  "type": "joined",
  "player_id": "Player1",
  "room_state": "Waiting|DeckSubmission|Playing|Finished"
}
```

### 2. left - 成功离开房间

```json
{
  "type": "left"
}
```

### 3. opponent_joined - 其他玩家加入

```json
{
  "type": "opponent_joined",
  "player_name": "玩家名称"
}
```

### 4. player_disconnected - 玩家断开连接

```json
{
  "type": "player_disconnected",
  "player_name": "玩家名称"
}
```

### 5. player_ready - 玩家已准备

```json
{
  "type": "player_ready",
  "player_name": "玩家名称",
  "deck_id": "卡组名称"
}
```

### 6. player_unready - 玩家取消准备

```json
{
  "type": "player_unready",
  "player_name": "玩家名称"
}
```

### 7. room_state - 房间状态（查询响应）

```json
{
  "type": "room_state",
  "players": [
	{
	  "name": "玩家1",
	  "is_ready": true,
	  "deck_id": "Example"
	},
	{
	  "name": "玩家2",
	  "is_ready": false,
	  "deck_id": null
	}
  ],
  "all_ready": false
}
```

### 8. waiting_for_deck - 等待提交卡组

```json
{
  "type": "waiting_for_deck"
}
```

### 9. game_starting - 游戏即将开始

两名玩家都准备后发送，表示游戏即将开始。

```json
{
  "type": "game_starting"
}
```

### 10. game_started - 游戏开始

游戏正式开始通知（纯通知，不包含状态数据）。

```json
{
  "type": "game_started"
}
```

> 客户端收到此通知后，应等待紧随其后的 `state_update` 获取初始游戏状态。

### 11. state_update - 状态更新

游戏状态变化时推送。

```json
{
  "type": "state_update",
  "state": { ... }
}
```

### 12. action_request - 请求动作

轮到当前玩家行动时发送。

```json
{
  "type": "action_request",
  "available_actions": ["play_card", "declare_attack", "pass"],
  "timeout_secs": 60
}
```

### 13. recovery_request - 请求选择复苏卡片

```json
{
  "type": "recovery_request",
  "count": 2,
  "options": ["card1", "card2", "card3"]
}
```

### 14. game_over - 游戏结束

```json
{
  "type": "game_over",
  "winner": "Player1",
  "reason": "Player2 disconnected"
}
```

### 15. error - 错误消息

```json
{
  "type": "error",
  "message": "错误描述"
}
```

---

## 游戏流程示例

### 正常对局流程

```
玩家A                              服务器                              玩家B
  |                                  |                                   |
  |---- join_room (room:1, name:A) --->|                                   |
  |                                  |---- joined (player_id: Player1) --->|
  |                                  |                                   |
  |                                  |<--- join_room (room:1, name:B) -----|
  |                                  |<--- joined (player_id: Player2) ----|
  |                                  |                                   |
  |<---------------------- opponent_joined (name: B) ---------------------|
  |                                  |                                   |
  |---- submit_deck (deck_id: X) --->|                                   |
  |                                  |---- player_ready (name: A, deck: X) ->|
  |                                  |                                   |
  |                                  |<--- submit_deck (deck_id: Y) --------|
  |                                  |<--- player_ready (name: B, deck: Y) -|
  |                                  |                                   |
  |<------------------------ game_starting -------------------------------|
  |                                  |                                   |
  |<------------------------ game_started -------------------------------|
```

### 房间状态轮询

客户端应每 2 秒发送一次 `query_room_state` 消息，获取最新房间状态。

---

## 断开连接处理

当客户端断开连接（关闭游戏或网络异常）时：

1. **正常离开**: 客户端应先发送 `leave_room` 消息，再断开 WebSocket 连接
2. **异常断开**: 服务器会检测到连接断开，自动将玩家标记为断开状态
3. **游戏进行中断开**: 另一名玩家获胜，游戏结束

---

## 卡组文件格式

卡组文件存储在 `desks/` 目录下，JSON 格式：

```json
{
  "name": "卡组名称",
  "cards": [
	"S000-C-001",
	"S000-C-001",
	"S000-C-002",
	...
  ]
}
```

- `name`: 卡组显示名称
- `cards`: 卡片ID数组，可以包含重复卡片（表示多张同名卡）
