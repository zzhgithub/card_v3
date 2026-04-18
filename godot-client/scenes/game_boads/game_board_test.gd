## 游戏板测试场景
## 模拟服务器数据，无需连接服务器即可测试游戏板

extends Control

## 测试数据 - 模拟服务器返回的状态
var test_game_state: Dictionary = {
	"turn_number": 1,
	"current_phase": "TurnStart",
	"current_player": "Player1",
	"your_state": {
		"hp": 5,
		"real_point": 0,
		"deck_count": 35,
		"hand": [
			{"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 1},
			{"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 2},
			{"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 3},
			{"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 4},
			{"current_attack": 1000, "definition_id": "S000-I-001", "instance_id": 5}
		],
		"front": [null, null, null, null, null],
		"back": [null, null, null, null, null],
		"cost_zone": [],
		"grave": []
	},
	"opponent_state": {
		"hp": 5,
		"real_point": 0,
		"deck_count": 35,
		"hand_count": 5,
		"front": [null, null, null, null, null],
		"back": [null, null, null, null, null],
		"cost_zone": [],
		"grave": []
	}
}

## 测试场景引用
@onready var game_board: Control = $GameBoard

func _ready():
	print("[Test] 游戏板测试场景已加载")

	# 延迟一帧确保 game_board 完全初始化
	await get_tree().process_frame

	# 发送模拟数据到游戏板
	_simulate_game_start()

	print("[Test] 按 '1' 键模拟状态更新（回合开始 -> 抽卡阶段）")
	print("[Test] 按 '2' 键模拟状态更新（抽卡阶段 -> 主要阶段）")
	print("[Test] 按 '3' 键模拟对手手牌数量变化")
	print("[Test] 按 'R' 键重新发送初始状态")

func _input(event):
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_1:
				_simulate_phase_change("Draw")
			KEY_2:
				_simulate_phase_change("Main")
			KEY_3:
				_simulate_opponent_card_play()
			KEY_R:
				_simulate_game_start()

## 模拟游戏开始
func _simulate_game_start():
	print("[Test] 模拟游戏开始...")
	if game_board.has_method("_on_game_started"):
		game_board._on_game_started({"state": test_game_state.duplicate(true)})
		print("[Test] 游戏开始数据已发送")
	else:
		push_error("[Test] GameBoard 没有 _on_game_started 方法")

## 模拟阶段变化
func _simulate_phase_change(new_phase: String):
	print("[Test] 模拟阶段变化到: %s" % new_phase)
	var new_state = test_game_state.duplicate(true)
	new_state["current_phase"] = new_phase
	if game_board.has_method("_on_state_update"):
		game_board._on_state_update(new_state)

## 模拟对手出牌（减少手牌数量）
func _simulate_opponent_card_play():
	print("[Test] 模拟对手出牌...")
	var new_state = test_game_state.duplicate(true)
	var current_count = new_state["opponent_state"]["hand_count"]
	if current_count > 0:
		new_state["opponent_state"]["hand_count"] = current_count - 1
		# 在前场放置一张卡
		new_state["opponent_state"]["front"][0] = {
			"current_attack": 1500,
			"definition_id": "S000-C-001",
			"instance_id": 100
		}
	if game_board.has_method("_on_state_update"):
		game_board._on_state_update(new_state)
		print("[Test] 对手手牌: %d -> %d" % [current_count, current_count - 1])
