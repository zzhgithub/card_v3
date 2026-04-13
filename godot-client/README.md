# Godot 客户端

卡片对战游戏的Godot客户端。

## 项目结构

```
godot-client/
├── project.godot         # Godot项目配置
├── PROTOCOL.md           # 通信协议文档
├── README.md             # 本文件
├── scripts/              # 脚本目录
│   ├── network.gd        # WebSocket网络管理
│   └── game_state.gd     # 游戏状态管理
└── scenes/               # 场景目录
	├── main.tscn         # 主场景
	└── main.gd           # 主场景脚本
```

## 快速开始

### 1. 启动服务器

在终端运行：
```bash
cd /path/to/card_v3
cargo run --package card-server
```

服务器默认监听 `ws://127.0.0.1:7878`

### 2. 打开Godot项目

1. 打开 Godot Engine (推荐 4.2+)
2. 点击"导入项目"
3. 选择 `card_v3/godot-client/` 目录
4. 点击"编辑"打开项目

### 3. 运行客户端

1. 按 F5 或点击"运行项目"
2. 点击"Connect"按钮连接服务器
3. 输入房间ID和玩家名称
4. 等待对手加入并开始游戏

## 核心脚本

### network.gd

WebSocket网络管理，提供以下功能：

**信号：**
- `connected` - 连接成功
- `disconnected` - 连接断开
- `message_received(message)` - 收到消息
- `error_occurred(error_message)` - 发生错误
- `game_started(game_state)` - 游戏开始
- `state_update(game_state)` - 状态更新
- `action_requested(actions, timeout)` - 请求行动
- `game_over(winner, reason)` - 游戏结束

**方法：**
```gdscript
# 连接服务器
Network.connect_to_server("ws://127.0.0.1:7878")

# 加入房间
Network.join_room("room1", "PlayerName")

# 提交卡组
var deck: Array[String] = ["S000-C-001", "S000-C-002"]
Network.submit_deck(deck)

# 发送行动
Network.send_action_pass()
Network.send_action_surrender()
Network.send_action_play_card(instance_id, "front", 0)
Network.send_action_declare_attack(attacker_id, "direct")
```

### game_state.gd

游戏状态管理单例，存储当前游戏状态：

**属性：**
- `turn_number` - 当前回合数
- `current_phase` - 当前阶段
- `current_player` - 当前行动玩家
- `my_state` - 自己的状态（HP、手牌、场地等）
- `opponent_state` - 对手状态

**方法：**
```gdscript
# 检查是否轮到自己
if GameState.is_my_turn():
	# 执行行动

# 获取状态信息
var my_hp = GameState.get_my_hp()
var opponent_hp = GameState.get_opponent_hp()
var hand = GameState.get_my_hand()
var hand_count = GameState.get_opponent_hand_count()

# 获取场地
var front_zone = GameState.get_front_zone("self")  # 或 "opponent"
var back_zone = GameState.get_back_zone()

# 获取可用行动
var actions = GameState.get_available_actions()
```

## 服务器连接

默认连接地址: `ws://127.0.0.1:7878`

修改方式：
1. 在Godot编辑器中选择 Network 节点
2. 在检查器中修改 `Server Url` 属性

或代码中修改：
```gdscript
Network.server_url = "ws://your-server:port"
```

## 协议文档

详见 [PROTOCOL.md](./PROTOCOL.md)

## 开发计划

### 已完成
- [x] WebSocket连接管理
- [x] 消息发送/接收
- [x] 游戏状态管理
- [x] 基础UI场景

### 待开发
- [ ] 卡片显示组件
- [ ] 场地格子系统
- [ ] 手牌拖动交互
- [ ] 攻击动画效果
- [ ] 更完善的UI界面
- [ ] 音效和音乐

## 调试技巧

### 查看网络日志
所有网络消息都会打印到Godot输出窗口。

### 测试卡组
使用以下卡片ID进行测试：
- `S000-C-001` - 角色卡
- `S000-C-002` - 角色卡
- `S000-S-001` - 策略卡

### 本地测试
1. 启动服务器
2. 运行两个Godot客户端实例
3. 两个客户端加入同一个房间
4. 双方提交卡组开始游戏
