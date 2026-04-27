## 游戏板测试场景
## 模拟服务器数据，无需连接服务器即可测试游戏板
## 提供可视化测试面板，可发送各种正常/边界/异常状态数据

extends Control

## 测试场景引用
@onready var game_board: Control = $GameBoard

## UI 引用
@onready var test_panel: Control = $TestPanel
@onready var test_button: Button = $TestButton
@onready var test_list: VBoxContainer = $TestPanel/ScrollContainer/VBoxContainer
@onready var log_label: RichTextLabel = $TestPanel/LogLabel

## 当前状态缓存（用于增量修改）
var _current_state: Dictionary = {}

## 测试用例数组（在 _ready 中填充）
var _test_cases: Array = []

## 基础状态模板
func _base_state() -> Dictionary:
	return {
		"turn_number": 1,
		"current_phase": "TurnStart",
		"current_player": "Player1",
		"your_state": {
			"hp": 5,
			"real_point": 0,
			"deck_count": 35,
			"hand": [
				{"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 1},
				{"current_attack": 800,  "definition_id": "S000-C-002", "instance_id": 2},
				{"current_attack": 1200, "definition_id": "S000-C-003", "instance_id": 3},
				{"current_attack": 1500, "definition_id": "S000-C-004", "instance_id": 4},
				{"current_attack": null,  "definition_id": "S000-S-001", "instance_id": 5}
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


func _ready():
	print("[Test] 游戏板测试场景已加载")

	# 延迟一帧确保 game_board 完全初始化
	await get_tree().process_frame

	# 填充测试用例（在 _ready 中填充避免类初始化时调用方法的潜在问题）
	_fill_test_cases()

	# 构建测试面板
	_build_test_panel()

	# 发送初始状态
	_simulate_state_update(_test_cases[0]["data"])

	print("[Test] 测试面板已就绪，共 %d 个测试用例" % _test_cases.size())


## 填充测试用例数据
func _fill_test_cases() -> void:
	_test_cases = [
		{
			"name": "1. 初始状态 (TurnStart)",
			"desc": "游戏开始时的初始状态，回合数为1，阶段为 TurnStart",
			"warn": "期望：双方手牌显示正确，HP/RP 显示为5/0，信息栏显示回合1",
			"data": _base_state()
		},
		{
			"name": "2. 抽卡阶段 (Draw)",
			"desc": "阶段变为 Draw，回合数不变",
			"warn": "期望：信息栏阶段显示为 Draw，其他区域不变",
			"data": _make_state_phase("Draw")
		},
		{
			"name": "3. 回收阶段 (Recovery)",
			"desc": "阶段变为 Recovery",
			"warn": "期望：信息栏显示 Recovery 阶段",
			"data": _make_state_phase("Recovery")
		},
		{
			"name": "4. 主要阶段1 (Main1)",
			"desc": "阶段变为 Main1，当前玩家为 Player1",
			"warn": "期望：信息栏显示 Main1，提示当前玩家行动",
			"data": _make_state_phase("Main1")
		},
		{
			"name": "5. 战斗阶段 (Battle)",
			"desc": "阶段变为 Battle",
			"warn": "期望：信息栏显示 Battle 阶段",
			"data": _make_state_phase("Battle")
		},
		{
			"name": "6. 主要阶段2 (Main2)",
			"desc": "阶段变为 Main2",
			"warn": "期望：信息栏显示 Main2 阶段",
			"data": _make_state_phase("Main2")
		},
		{
			"name": "7. 回合结束 (TurnEnd)",
			"desc": "阶段变为 TurnEnd",
			"warn": "期望：信息栏显示 TurnEnd 阶段",
			"data": _make_state_phase("TurnEnd")
		},
		{
			"name": "8. 第5回合 + 换玩家",
			"desc": "回合数增加到5，当前玩家切换为 Player2",
			"warn": "期望：信息栏显示回合5，当前玩家为 Player2",
			"data": _make_state_turn(5, "Player2", "Main1")
		},
		{
			"name": "9. 对手前场有卡",
			"desc": "对手在前场 slot 0 和 slot 2 放置了卡牌",
			"warn": "期望：对手前场区域显示2张卡，slot 0 和 slot 2 位置正确",
			"data": _make_state_opponent_front()
		},
		{
			"name": "10. 对手后场有卡",
			"desc": "对手在后场 slot 1 和 slot 3 放置了卡牌",
			"warn": "期望：对手后场区域显示2张卡",
			"data": _make_state_opponent_back()
		},
		{
			"name": "11. 自己前场满员 (5张)",
			"desc": "自己前场5个格子全部有卡",
			"warn": "期望：自己前场显示5张卡，排列整齐，无重叠或错位",
			"data": _make_state_player_front_full()
		},
		{
			"name": "12. 自己后场满员 (5张)",
			"desc": "自己后场5个格子全部有卡",
			"warn": "期望：自己后场显示5张卡",
			"data": _make_state_player_back_full()
		},
		{
			"name": "13. 费用区有卡 (3张)",
			"desc": "自己费用区放置了3张卡",
			"warn": "期望：费用区显示3张卡，卡背朝上，数量标签正确",
			"data": _make_state_cost_zone()
		},
		{
			"name": "14. 墓地有卡 (4张)",
			"desc": "自己墓地有4张卡，正面朝上显示",
			"warn": "期望：墓地显示4张卡（最多显示5张），最新卡在最上面，数量标签为4",
			"data": _make_state_grave()
		},
		{
			"name": "15. 手牌为空",
			"desc": "自己手牌数组为空",
			"warn": "期望：手牌区域为空，不显示任何卡牌。注意：是否出现 null 引用错误？",
			"data": _make_state_empty_hand()
		},
		{
			"name": "16. 手牌满 (20张)",
			"desc": "自己手牌有20张卡（上限）",
			"warn": "期望：手牌区域显示20张卡，排列不超出屏幕边界，可以滚动或缩放",
			"data": _make_state_full_hand()
		},
		{
			"name": "17. HP = 1 (濒死)",
			"desc": "自己 HP 降至1",
			"warn": "期望：HP 显示为1，UI 可能有红色警告或闪烁效果",
			"data": _make_state_hp(1)
		},
		{
			"name": "18. HP = 0 (死亡)",
			"desc": "自己 HP 降至0",
			"warn": "期望：HP 显示为0，可能触发游戏结束 UI。注意：是否出现负数？",
			"data": _make_state_hp(0)
		},
		{
			"name": "19. RP = 6 (满值)",
			"desc": "自己 RealPoint 为6（最大值）",
			"warn": "期望：RP 显示为6，UI 可能有满值高亮效果",
			"data": _make_state_rp(6)
		},
		{
			"name": "20. 卡组剩1张",
			"desc": "自己卡组只剩1张卡",
			"warn": "期望：卡组堆叠显示1张卡背，数量标签为1",
			"data": _make_state_deck_count(1)
		},
		{
			"name": "21. 卡组为0 (DeckOut)",
			"desc": "自己卡组为0张",
			"warn": "期望：卡组区域为空或显示0，可能触发 DeckOut 提示",
			"data": _make_state_deck_count(0)
		},
		{
			"name": "22. 对手手牌0张",
			"desc": "对手手牌数量为0",
			"warn": "期望：对手手牌区域为空，不显示卡背",
			"data": _make_state_opponent_hand(0)
		},
		{
			"name": "23. 对手手牌7张 (上限显示)",
			"desc": "对手手牌数量为7张",
			"warn": "期望：对手手牌显示7张卡背（代码上限为7），超出部分不显示但 hand_count 正确",
			"data": _make_state_opponent_hand(7)
		},
		{
			"name": "24. 缺少 your_state",
			"desc": "状态数据中完全缺少 your_state 字段",
			"warn": "异常数据测试！期望：游戏板不崩溃，自己区域保持上次状态或显示默认值。检查是否有报错",
			"data": _make_state_missing_your_state()
		},
		{
			"name": "25. 缺少 opponent_state",
			"desc": "状态数据中完全缺少 opponent_state 字段",
			"warn": "异常数据测试！期望：游戏板不崩溃，对手区域保持上次状态或显示默认值",
			"data": _make_state_missing_opponent_state()
		},
		{
			"name": "26. hand 为 null",
			"desc": "your_state.hand 为 null 而非数组",
			"warn": "异常数据测试！期望：游戏板不崩溃，手牌区域为空。检查 _update_hand 是否处理 null",
			"data": _make_state_hand_null()
		},
		{
			"name": "27. front 包含错误类型",
			"desc": "front 数组中包含字符串而非字典/null",
			"warn": "异常数据测试！期望：游戏板不崩溃，错误位置不显示卡牌。检查类型判断",
			"data": _make_state_front_invalid()
		},
		{
			"name": "28. 空状态 {}",
			"desc": "发送完全空的状态字典",
			"warn": "异常数据测试！期望：游戏板不崩溃，所有区域保持上次状态或清空",
			"data": {}
		},
		{
			"name": "29. 所有区域为空",
			"desc": "正常结构但所有数组为空，数值为0",
			"warn": "期望：游戏板显示空场地，HP/RP/卡组数量均为0或默认值",
			"data": _make_state_all_empty()
		},
		{
			"name": "30. 综合复杂状态",
			"desc": "包含所有区域都有卡的复杂状态",
			"warn": "期望：所有区域正确显示，无错位、无重叠、无遗漏",
			"data": _make_state_complex()
		}
	]


# ============================================================================
# 测试数据生成辅助函数
# ============================================================================

func _make_state_phase(phase: String) -> Dictionary:
	var s = _base_state()
	s["current_phase"] = phase
	return s

func _make_state_turn(turn: int, player: String, phase: String) -> Dictionary:
	var s = _base_state()
	s["turn_number"] = turn
	s["current_player"] = player
	s["current_phase"] = phase
	return s

func _make_state_opponent_front() -> Dictionary:
	var s = _base_state()
	s["opponent_state"]["front"][0] = {"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 100}
	s["opponent_state"]["front"][2] = {"current_attack": 1200, "definition_id": "S000-C-003", "instance_id": 101}
	return s

func _make_state_opponent_back() -> Dictionary:
	var s = _base_state()
	s["opponent_state"]["back"][1] = {"current_attack": 800, "definition_id": "S000-C-002", "instance_id": 102}
	s["opponent_state"]["back"][3] = {"current_attack": 1000, "definition_id": "S000-I-001", "instance_id": 103}
	return s

func _make_state_player_front_full() -> Dictionary:
	var s = _base_state()
	for i in range(5):
		s["your_state"]["front"][i] = {"current_attack": 1500 + i * 100, "definition_id": "S000-C-00%d" % (i+1), "instance_id": 200 + i}
	return s

func _make_state_player_back_full() -> Dictionary:
	var s = _base_state()
	for i in range(5):
		s["your_state"]["back"][i] = {"current_attack": 800 + i * 50, "definition_id": "S000-S-00%d" % (i+1), "instance_id": 210 + i}
	return s

func _make_state_cost_zone() -> Dictionary:
	var s = _base_state()
	s["your_state"]["cost_zone"] = [
		{"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 300},
		{"current_attack": 800,  "definition_id": "S000-C-002", "instance_id": 301},
		{"current_attack": 1200, "definition_id": "S000-C-003", "instance_id": 302}
	]
	return s

func _make_state_grave() -> Dictionary:
	var s = _base_state()
	s["your_state"]["grave"] = [
		{"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 400},
		{"current_attack": 800,  "definition_id": "S000-C-002", "instance_id": 401},
		{"current_attack": 1200, "definition_id": "S000-C-003", "instance_id": 402},
		{"current_attack": 1500, "definition_id": "S000-C-004", "instance_id": 403}
	]
	return s

func _make_state_empty_hand() -> Dictionary:
	var s = _base_state()
	s["your_state"]["hand"] = []
	return s

func _make_state_full_hand() -> Dictionary:
	var s = _base_state()
	var hand = []
	for i in range(20):
		hand.append({"current_attack": 1000 + i * 50, "definition_id": "S000-C-001", "instance_id": 500 + i})
	s["your_state"]["hand"] = hand
	return s

func _make_state_hp(hp: int) -> Dictionary:
	var s = _base_state()
	s["your_state"]["hp"] = hp
	return s

func _make_state_rp(rp: int) -> Dictionary:
	var s = _base_state()
	s["your_state"]["real_point"] = rp
	return s

func _make_state_deck_count(count: int) -> Dictionary:
	var s = _base_state()
	s["your_state"]["deck_count"] = count
	return s

func _make_state_opponent_hand(count: int) -> Dictionary:
	var s = _base_state()
	s["opponent_state"]["hand_count"] = count
	return s

func _make_state_missing_your_state() -> Dictionary:
	var s = _base_state()
	s.erase("your_state")
	return s

func _make_state_missing_opponent_state() -> Dictionary:
	var s = _base_state()
	s.erase("opponent_state")
	return s

func _make_state_hand_null() -> Dictionary:
	var s = _base_state()
	s["your_state"]["hand"] = null
	return s

func _make_state_front_invalid() -> Dictionary:
	var s = _base_state()
	s["your_state"]["front"][0] = "invalid_card"
	s["your_state"]["front"][1] = 12345
	return s

func _make_state_all_empty() -> Dictionary:
	return {
		"turn_number": 1,
		"current_phase": "TurnStart",
		"current_player": "Player1",
		"your_state": {
			"hp": 0,
			"real_point": 0,
			"deck_count": 0,
			"hand": [],
			"front": [null, null, null, null, null],
			"back": [null, null, null, null, null],
			"cost_zone": [],
			"grave": []
		},
		"opponent_state": {
			"hp": 0,
			"real_point": 0,
			"deck_count": 0,
			"hand_count": 0,
			"front": [null, null, null, null, null],
			"back": [null, null, null, null, null],
			"cost_zone": [],
			"grave": []
		}
	}

func _make_state_complex() -> Dictionary:
	var s = _base_state()
	s["turn_number"] = 3
	s["current_phase"] = "Main1"
	# 自己前场2张
	s["your_state"]["front"][0] = {"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 600}
	s["your_state"]["front"][3] = {"current_attack": 1200, "definition_id": "S000-C-003", "instance_id": 601}
	# 自己后场1张
	s["your_state"]["back"][2] = {"current_attack": null, "definition_id": "S000-S-001", "instance_id": 602}
	# 费用区2张
	s["your_state"]["cost_zone"] = [
		{"current_attack": 800, "definition_id": "S000-C-002", "instance_id": 603},
		{"current_attack": 1500, "definition_id": "S000-C-004", "instance_id": 604}
	]
	# 墓地3张
	s["your_state"]["grave"] = [
		{"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 605},
		{"current_attack": 800,  "definition_id": "S000-C-002", "instance_id": 606},
		{"current_attack": 1200, "definition_id": "S000-C-003", "instance_id": 607}
	]
	# 对手前场3张
	s["opponent_state"]["front"][0] = {"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 700}
	s["opponent_state"]["front"][2] = {"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 701}
	s["opponent_state"]["front"][4] = {"current_attack": 1500, "definition_id": "S000-C-001", "instance_id": 702}
	# 对手后场2张
	s["opponent_state"]["back"][1] = {"current_attack": null, "definition_id": "S000-S-001", "instance_id": 703}
	s["opponent_state"]["back"][4] = {"current_attack": null, "definition_id": "S000-S-002", "instance_id": 704}
	# 对手手牌10张
	s["opponent_state"]["hand_count"] = 10
	return s


# ============================================================================
# UI 构建与交互
# ============================================================================

func _build_test_panel():
	# 清除旧按钮
	for child in test_list.get_children():
		child.queue_free()

	for case in _test_cases:
		var btn := Button.new()
		btn.text = case["name"]
		btn.alignment = HORIZONTAL_ALIGNMENT_LEFT
		btn.custom_minimum_size = Vector2(0, 36)

		var desc := Label.new()
		var warn := Label.new()

		var case_data = case["data"]
		var case_name = case["name"]
		var case_warn = case["warn"]
		var case_desc = case["desc"]

		btn.pressed.connect(func():
			var state: Dictionary = case_data.duplicate(true) if case_data is Dictionary else case_data
			_simulate_state_update(state)
			_log("[发送] %s" % case_name)
			_log("  描述: %s" % case_desc)
			_log("  提示: %s" % case_warn)
		)

		test_list.add_child(btn)

		# 描述标签（默认隐藏，hover 时显示）
		desc.text = "  %s" % case_desc
		desc.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		desc.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
		desc.visible = false
		test_list.add_child(desc)

		# 提示标签
		warn.text = "  ⚠ %s" % case_warn
		warn.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		warn.add_theme_color_override("font_color", Color(1.0, 0.8, 0.4))
		warn.visible = false
		test_list.add_child(warn)

		# 鼠标悬停显示/隐藏描述
		btn.mouse_entered.connect(func():
			desc.visible = true
			warn.visible = true
		)
		btn.mouse_exited.connect(func():
			desc.visible = false
			warn.visible = false
		)

		# 分隔线
		var sep := HSeparator.new()
		sep.modulate = Color(0.3, 0.3, 0.3)
		test_list.add_child(sep)

	# 连接测试面板开关
	test_button.pressed.connect(_toggle_test_panel)


func _toggle_test_panel():
	test_panel.visible = not test_panel.visible
	test_button.text = "关闭测试" if test_panel.visible else "打开测试"


func _simulate_state_update(state: Dictionary):
	_current_state = state.duplicate(true)
	if game_board.has_method("_on_state_update"):
		game_board._on_state_update(state)
	else:
		push_error("[Test] GameBoard 没有 _on_state_update 方法")


func _log(text: String):
	log_label.text += text + "\n"
	# 自动滚动到底部
	log_label.scroll_to_line(log_label.get_line_count() - 1)
	print(text)


# ============================================================================
# 键盘快捷键（保留原有功能）
# ============================================================================

func _input(event):
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_1:
				_send_preset_state("TurnStart")
			KEY_2:
				_send_preset_state("Draw")
			KEY_3:
				_send_preset_state("Main1")
			KEY_4:
				_send_preset_state("Battle")
			KEY_5:
				_send_preset_state("Main2")
			KEY_6:
				_send_preset_state("TurnEnd")
			KEY_R:
				_send_preset_state("TurnStart")
			KEY_T:
				_toggle_test_panel()


func _send_preset_state(phase: String):
	var s = _base_state()
	s["current_phase"] = phase
	_simulate_state_update(s)
	_log("[快捷键] 阶段 -> %s" % phase)
