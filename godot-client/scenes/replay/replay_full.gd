## 完整棋盘回放 — 使用真实 game_board + AnimationManager 动画
extends Control

@onready var game_board: Control = $GameBoard

# 控制栏
@onready var file_selector: OptionButton = $ControlBar/HBox/FileSelector
@onready var play_pause_btn: Button = $ControlBar/HBox/PlayPauseBtn
@onready var step_btn: Button = $ControlBar/HBox/StepBtn
@onready var speed_btn: Button = $ControlBar/HBox/SpeedBtn
@onready var progress_slider: HSlider = $ControlBar/HBox/ProgressSlider
@onready var message_label: Label = $ControlBar/HBox/MessageLabel
@onready var jump_input: LineEdit = $ControlBar/HBox/JumpInput
@onready var jump_btn: Button = $ControlBar/HBox/JumpBtn

var _messages: Array = []
var _current_index: int = 0
var _is_playing: bool = false
var _speed_multiplier: float = 1.0
var _pending_delay: float = 0.0

var _replay_files: Array = [
	{"name": "--- 场景测试 ---", "path": "", "disabled": true},
	{"name": "1. 初始状态", "path": "res://assets/replay/test_scenarios/scenario_01.json"},
	{"name": "2. 抽卡", "path": "res://assets/replay/test_scenarios/scenario_02.json"},
	{"name": "3. 登场(前场)+费用", "path": "res://assets/replay/test_scenarios/scenario_03.json"},
	{"name": "4. 登场(后场)+费用", "path": "res://assets/replay/test_scenarios/scenario_04.json"},
	{"name": "5. 对手前场登场", "path": "res://assets/replay/test_scenarios/scenario_05.json"},
	{"name": "6. 攻击+破坏+HP", "path": "res://assets/replay/test_scenarios/scenario_06.json"},
	{"name": "7. 自己被攻击破坏", "path": "res://assets/replay/test_scenarios/scenario_07.json"},
	{"name": "8. 回合切换+P2抽卡", "path": "res://assets/replay/test_scenarios/scenario_08.json"},
	{"name": "9. RP变化", "path": "res://assets/replay/test_scenarios/scenario_09.json"},
	{"name": "10. GameOver", "path": "res://assets/replay/test_scenarios/scenario_10.json"},
	{"name": "11. 多事件混合", "path": "res://assets/replay/test_scenarios/scenario_11.json"},
	{"name": "12. 费用区多卡", "path": "res://assets/replay/test_scenarios/scenario_12.json"},
		{"name": "--- 完整流程 ---", "path": "", "disabled": true},
		{"name": "简单流程 (14步)", "path": "res://assets/replay/e2e_replay_simple.json"},
	]
const SPEEDS: Array[float] = [1.0, 5.0, 10.0]


func _ready():
	for f in _replay_files:
		if f.get("disabled", false):
			file_selector.add_separator(f["name"])
		else:
			file_selector.add_item(f["name"])
	file_selector.select(1)

	play_pause_btn.pressed.connect(_on_play_pause)
	step_btn.pressed.connect(_on_step)
	speed_btn.pressed.connect(_on_speed_toggle)
	progress_slider.value_changed.connect(_on_slider_changed)
	progress_slider.drag_ended.connect(_on_slider_drag_ended)
	jump_btn.pressed.connect(_on_jump)
	file_selector.item_selected.connect(_on_file_selected)

	_load_replay(_replay_files[1]["path"])


func _load_replay(path: String):
	var file = FileAccess.open(path, FileAccess.READ)
	if not file:
		push_error("Cannot open: %s" % path)
		return
	var data = JSON.parse_string(file.get_as_text())
	file.close()
	if data == null:
		push_error("Parse failed: %s" % path)
		return

	_messages = data.get("messages", [])
	_is_playing = false
	play_pause_btn.text = "▶"
	_current_index = 0
	progress_slider.max_value = max(0, _messages.size() - 1)
	progress_slider.value = 0

	# 延迟一帧：等容器布局完成后再加载状态，否则卡组 global_position 是 (0,0)
	await get_tree().process_frame
	_seek_to_first_state()
	_update_ui()
	print("[ReplayFull] Loaded %d messages: %s" % [_messages.size(), data.get("scenario", path)])


func _process(delta: float):
	if not _is_playing:
		return
	_pending_delay -= delta * _speed_multiplier
	while _pending_delay <= 0.0 and _current_index < _messages.size():
		_advance_one()
		if _pending_delay > 0.0:
			break


func _advance_one():
	if _current_index >= _messages.size():
		return

	var msg: Dictionary = _messages[_current_index]
	var msg_type: String = msg.get("type", "")
	var msg_data = msg.get("data", {})

	match msg_type:
		"state_update":
			if game_board.has_method("_on_state_update"):
				game_board._on_state_update(msg_data)
		"game_over":
			_is_playing = false
			play_pause_btn.text = "▶"
			_current_index = 0
			progress_slider.value = 0
			_seek_to_first_state()
			_update_ui()
			return
		_:
			pass  # 跳过 action_request / chain 等

	_current_index += 1
	_update_ui()

	if _current_index < _messages.size():
		_pending_delay = float(_messages[_current_index].get("delay_ms", 0)) / 1000.0


func _seek_to_first_state():
	for i in range(0, _messages.size()):
		var m: Dictionary = _messages[i]
		if m.get("type") == "state_update":
			if game_board.has_method("_on_state_update"):
				game_board._on_state_update(m.get("data", {}))
			_current_index = i
			return


func _update_ui():
	if _messages.is_empty():
		return
	var idx = min(_current_index, _messages.size() - 1)
	var msg: Dictionary = _messages[idx]
	message_label.text = "[%d/%d] %s" % [idx + 1, _messages.size(), msg.get("type", "?")]
	progress_slider.set_value_no_signal(idx)


# ── 按钮事件 ──

func _on_play_pause():
	_is_playing = not _is_playing
	play_pause_btn.text = "⏸" if _is_playing else "▶"

func _on_step():
	_is_playing = false
	play_pause_btn.text = "▶"
	if _current_index < _messages.size():
		_advance_one()
	else:
		_current_index = 0
		_seek_to_first_state()
		_update_ui()

func _on_speed_toggle():
	var idx = SPEEDS.find(_speed_multiplier)
	_speed_multiplier = SPEEDS[(idx + 1) % SPEEDS.size()]
	speed_btn.text = "%gx" % _speed_multiplier

func _on_slider_changed(_v: float):
	message_label.text = "[%d/%d] ..." % [int(progress_slider.value) + 1, _messages.size()]

func _on_slider_drag_ended(changed: bool):
	if not changed:
		return
	var target = int(progress_slider.value)
	for i in range(target, -1, -1):
		if _messages[i].get("type") == "state_update":
			game_board._on_state_update(_messages[i].get("data", {}))
			_current_index = i
			break
	_update_ui()

func _on_jump():
	var t = jump_input.text.strip_edges()
	if t.is_valid_int():
		var idx = clampi(t.to_int() - 1, 0, _messages.size() - 1)
		for i in range(idx, -1, -1):
			if _messages[i].get("type") == "state_update":
				game_board._on_state_update(_messages[i].get("data", {}))
				_current_index = i
				break
		_update_ui()

func _on_file_selected(idx: int):
	var f = _replay_files[idx]
	if not f.get("disabled", false):
		_load_replay(f["path"])
