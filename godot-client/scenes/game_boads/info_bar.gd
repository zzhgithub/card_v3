## 信息分割条
## 显示回合信息：回合数、当前玩家、当前阶段
## 预留倒计时位置

class_name InfoBar
extends Control

@onready var turn_label: Label = $TurnLabel
@onready var player_label: Label = $PlayerLabel
@onready var phase_label: Label = $PhaseLabel
@onready var timer_label: Label = $TimerLabel

var current_turn: int = 1
var current_player: String = ""
var current_phase: String = ""
var countdown_time: int = 0

func _ready() -> void:
	_update_display()


## 更新回合数
func set_turn(turn: int) -> void:
	current_turn = turn
	_update_display()


## 更新当前玩家
func set_player(player_name: String) -> void:
	current_player = player_name
	_update_display()


## 更新当前阶段
func set_phase(phase: String) -> void:
	current_phase = phase
	_update_display()


## 更新倒计时
func set_countdown(seconds: int) -> void:
	countdown_time = seconds
	_update_display()


## 更新显示
func _update_display() -> void:
	if turn_label:
		turn_label.text = "回合: %d" % current_turn

	if player_label:
		player_label.text = "当前玩家: %s" % (current_player if current_player else "-")

	if phase_label:
		phase_label.text = "阶段: %s" % (current_phase if current_phase else "-")

	if timer_label:
		if countdown_time > 0:
			timer_label.text = "%02d:%02d" % [countdown_time / 60, countdown_time % 60]
		else:
			timer_label.text = "--:--"


## 从服务器状态更新
func update_from_state(state: Dictionary) -> void:
	if state.has("turn_number"):
		set_turn(state["turn_number"])

	if state.has("current_player"):
		var player_id = state["current_player"]
		set_player("Player1" if player_id == "Player1" else "Player2")

	if state.has("current_phase"):
		var phase = state["current_phase"]
		set_phase(_format_phase(phase))


## 格式化阶段名称
func _format_phase(phase: String) -> String:
	match phase:
		"TurnStart": return "回合开始"
		"Draw": return "抽卡阶段"
		"Recovery": return "复苏阶段"
		"Main1": return "主要阶段1"
		"Battle": return "战斗阶段"
		"Main2": return "主要阶段2"
		"TurnEnd": return "回合结束"
		_: return phase
