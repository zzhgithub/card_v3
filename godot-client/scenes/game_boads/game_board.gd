## 对局主场景
## 集成所有游戏区域：手牌、场地、卡组、墓地、费用区

extends Control

## 玩家数据
var player_hp: int = 20
var player_real_points: int = 0
var opponent_hp: int = 20
var opponent_real_points: int = 0

## 网络管理器
var network_manager: Node
var scene_manager: Node

## CardManager 引用
@onready var card_manager: CardManager = $CardManager

## 场景引用 - 对手区域
@onready var opponent_hand: MyHand = $OpponentArea/OpponentTopRow/OpponentHand
@onready var opponent_deck: CardStack = $OpponentArea/OpponentMainRow/OpponentStacks/OpponentDeck
@onready var opponent_grave: CardStack = $OpponentArea/OpponentMainRow/OpponentStacks/OpponentGrave
@onready var opponent_front_field: HBoxContainer = $OpponentArea/OpponentMainRow/OpponentBattleField/OpponentFrontField
@onready var opponent_back_field: HBoxContainer = $OpponentArea/OpponentMainRow/OpponentBattleField/OpponentBackField
@onready var opponent_cost: CostZone = $OpponentArea/OpponentMainRow/OpponentCost
@onready var opponent_status: VBoxContainer = $OpponentArea/OpponentTopRow/OpponentStatus

## 场景引用 - 自己区域
@onready var player_hand: MyHand = $PlayerArea/PlayerBottomRow/PlayerHand
@onready var player_deck: CardStack = $PlayerArea/PlayerMainRow/PlayerStacks/PlayerDeck
@onready var player_grave: CardStack = $PlayerArea/PlayerMainRow/PlayerStacks/PlayerGrave
@onready var player_front_field: HBoxContainer = $PlayerArea/PlayerMainRow/PlayerBattleField/PlayerFrontField
@onready var player_back_field: HBoxContainer = $PlayerArea/PlayerMainRow/PlayerBattleField/PlayerBackField
@onready var player_cost: CostZone = $PlayerArea/PlayerMainRow/PlayerCost
@onready var player_status: VBoxContainer = $PlayerArea/PlayerBottomRow/PlayerStatus

## 中央信息条
@onready var info_bar: InfoBar = $InfoBar

## UI 引用
@onready var menu_button: Button = $MenuButton

## 卡牌数据缓存 (instance_id -> card_info)
var card_cache: Dictionary = {}
var back_card_texture: Texture2D
var default_card_scene: PackedScene

func _ready():
	# 获取管理器
	network_manager = get_node_or_null("/root/NetworkManager")
	scene_manager = get_node_or_null("/root/SceneManager")

	# 加载卡背纹理
	back_card_texture = load("res://images/framework/frame-c.png")
	default_card_scene = load("res://dule/dule_card.tscn")

	# 连接信号
	menu_button.pressed.connect(_on_menu_pressed)

	if network_manager:
		network_manager.game_started.connect(_on_game_started)
		network_manager.state_update.connect(_on_state_update)
		network_manager.action_request.connect(_on_action_requested)
		network_manager.recovery_request.connect(_on_recovery_requested)
		network_manager.game_over.connect(_on_game_over)
	network_manager.chain_action_request.connect(_on_chain_action_requested)

	# 初始化战场
	_initialize_fields()
	_initialize_card_stacks()

	_print("Game board ready")


## 初始化战场格子
func _initialize_fields() -> void:
	# 设置对手前场格子（上排，但场上是前场）
	for i in range(5):
		var slot = opponent_front_field.get_child(i)
		if slot is BattleFieldSlot:
			slot.field_type = BattleFieldSlot.FieldType.FRONT
			slot.slot_index = i

	# 设置对手后场格子（下排）
	for i in range(5):
		var slot = opponent_back_field.get_child(i)
		if slot is BattleFieldSlot:
			slot.field_type = BattleFieldSlot.FieldType.BACK
			slot.slot_index = i

	# 设置自己前场格子（上排）
	for i in range(5):
		var slot = player_front_field.get_child(i)
		if slot is BattleFieldSlot:
			slot.field_type = BattleFieldSlot.FieldType.FRONT
			slot.slot_index = i

	# 设置自己后场格子（下排）
	for i in range(5):
		var slot = player_back_field.get_child(i)
		if slot is BattleFieldSlot:
			slot.field_type = BattleFieldSlot.FieldType.BACK
			slot.slot_index = i


## 初始化卡组/墓地
func _initialize_card_stacks() -> void:
	# 设置卡组类型和朝向
	opponent_deck.stack_type = CardStack.StackType.DECK
	player_deck.stack_type = CardStack.StackType.DECK
	opponent_grave.stack_type = CardStack.StackType.GRAVE
	player_grave.stack_type = CardStack.StackType.GRAVE

	# 设置卡组为卡背朝上，墓地为正面朝上
	opponent_deck.card_face_up = false
	player_deck.card_face_up = false
	opponent_grave.card_face_up = true
	player_grave.card_face_up = true

	# 设置竖直布局
	opponent_deck.stack_layout = CardStack.StackLayoutDirection.VERTICAL
	player_deck.stack_layout = CardStack.StackLayoutDirection.VERTICAL
	opponent_grave.stack_layout = CardStack.StackLayoutDirection.VERTICAL
	player_grave.stack_layout = CardStack.StackLayoutDirection.VERTICAL


## 游戏开始
func _on_game_started(game_data: Dictionary) -> void:
	_print("Game started! Waiting for first state update...")
	# GameStarted 是纯通知，不再携带状态。
	# Board 的初始化和状态更新由 state_update 信号驱动。


## 游戏结束
func _on_game_over(winner: String, reason: String) -> void:
	_print("Game over! Winner: %s, Reason: %s" % [winner, reason])
	_clear_action_ui()

	var popup = AcceptDialog.new()
	popup.title = "游戏结束"
	popup.dialog_text = "胜者: %s\n原因: %s" % [winner, reason]
	popup.confirmed.connect(_return_to_lobby)
	add_child(popup)
	popup.popup_centered()


## 状态更新
func _on_state_update(state: Dictionary) -> void:
	_update_from_state(state)


## 从服务器状态更新界面
func _update_from_state(state: Dictionary) -> void:
	if state.is_empty():
		return

	# 更新信息条
	if info_bar:
		info_bar.update_from_state(state)

	# 获取玩家状态
	var my_state = state.get("your_state", {})
	var opponent_state = state.get("opponent_state", {})

	# 处理近期事件（用于动画驱动）
	var recent_events = _safe_array(state.get("recent_events", []))
	_process_recent_events(recent_events)

	# 更新自己状态
	_update_player_state(my_state)

	# 更新对手状态
	_update_opponent_state(opponent_state)


## 安全获取数组字段（处理 null 值）
func _safe_array(value) -> Array:
	if value == null:
		return []
	if typeof(value) == TYPE_ARRAY:
		return value
	return []


## 更新自己状态显示
func _update_player_state(state: Dictionary) -> void:
	if state.is_empty():
		return

	player_hp = state.get("hp", 20)
	player_real_points = state.get("real_point", 0)

	_update_status_panel(player_status, player_hp, player_real_points, "自己")

	# 更新手牌
	var hand_cards = _safe_array(state.get("hand", []))
	_update_hand(player_hand, hand_cards, true)

	# 更新卡组（优先用 deck 数组创建真实卡片背面，否则回退到 deck_count）
	var deck_cards = _safe_array(state.get("deck", []))
	var deck_count = state.get("deck_count", deck_cards.size())
	_update_deck_stack(player_deck, deck_cards, deck_count)

	# 更新墓地
	var grave_cards = _safe_array(state.get("grave", []))
	_update_grave_stack(player_grave, grave_cards)

	# 更新前场/后场
	var front_cards = _safe_array(state.get("front", []))
	var back_cards = _safe_array(state.get("back", []))
	_update_field_slots(player_front_field, front_cards, true)
	_update_field_slots(player_back_field, back_cards, true)

	# 更新费用区
	var cost_cards = _safe_array(state.get("cost_zone", []))
	_update_cost_zone(player_cost, cost_cards, true)

	# 缓存卡牌信息
	_cache_cards(hand_cards)
	_cache_cards(grave_cards)
	_cache_cards(front_cards)
	_cache_cards(back_cards)
	_cache_cards(cost_cards)


## 更新对手状态显示
func _update_opponent_state(state: Dictionary) -> void:
	if state.is_empty():
		return

	opponent_hp = state.get("hp", 20)
	opponent_real_points = state.get("real_point", 0)

	_update_status_panel(opponent_status, opponent_hp, opponent_real_points, "对手")

	# 更新对手手牌（只显示背面，数量通过 hand_count）
	var hand_count = state.get("hand_count", 0)
	_update_opponent_hand(opponent_hand, hand_count)

	# 更新对手卡组
	var deck_cards = _safe_array(state.get("deck", []))
	var deck_count = state.get("deck_count", deck_cards.size())
	_update_deck_stack(opponent_deck, deck_cards, deck_count)

	# 更新对手墓地
	var grave_cards = _safe_array(state.get("grave", []))
	_update_grave_stack(opponent_grave, grave_cards)

	# 更新对手前场/后场
	var front_cards = _safe_array(state.get("front", []))
	var back_cards = _safe_array(state.get("back", []))
	_update_field_slots(opponent_front_field, front_cards, true)
	_update_field_slots(opponent_back_field, back_cards, true)

	# 更新对手费用区
	var cost_cards = _safe_array(state.get("cost_zone", []))
	_update_cost_zone(opponent_cost, cost_cards, true)

	# 缓存卡牌信息
	_cache_cards(grave_cards)
	_cache_cards(front_cards)
	_cache_cards(back_cards)
	_cache_cards(cost_cards)


## 加载卡牌定义 JSON
func _load_card_definition(definition_id: String) -> Dictionary:
	var path = "res://scripts_json/%s/%s.json" % [definition_id.get_slice("-", 0), definition_id]
	if not FileAccess.file_exists(path):
		return {}
	var file = FileAccess.open(path, FileAccess.READ)
	var text = file.get_as_text()
	file.close()
	var json = JSON.new()
	var err = json.parse(text)
	if err != OK:
		push_error("Failed to parse card JSON: %s" % path)
		return {}
	return json.data


## 创建一张卡牌
func _create_card(definition_id: String, container: CardContainer, face_up: bool = true, extra_data: Dictionary = {}) -> Card:
	if default_card_scene == null:
		push_error("Default card scene not loaded")
		return null

	var card = default_card_scene.instantiate() as Card
	if card == null:
		push_error("Failed to instantiate card")
		return null

	# 设置卡牌尺寸
	card.custom_minimum_size = Vector2(79, 110)
	card.size = Vector2(79, 110)
	card.card_size = Vector2(79, 110)
	card.card_name = definition_id
	card.show_front = face_up

	# 加载并合并卡片定义数据
	var card_def = _load_card_definition(definition_id)
	if not card_def.is_empty():
		for key in extra_data:
			card_def[key] = extra_data[key]
		if card is DuleCard:
			card.setup(card_def)

	# 添加到容器
	container.add_card(card)

	return card


## 更新手牌显示
func _update_hand(hand: MyHand, cards: Array, face_up: bool) -> void:
	if hand == null:
		return

	# 清除现有手牌
	hand.clear_cards()

	# 创建新手牌
	for card_info in cards:
		if typeof(card_info) != TYPE_DICTIONARY:
			continue
		var definition_id = card_info.get("definition_id", "")
		if definition_id == "":
			continue

		var card = _create_card(definition_id, hand, face_up, card_info)
		if card != null:
			# 存储 instance_id 到卡牌 metadata 中
			card.set_meta("instance_id", card_info.get("instance_id", 0))
			card.set_meta("definition_id", definition_id)
			card.set_meta("current_attack", card_info.get("current_attack", 0))


## 更新对手手牌（只显示背面）
func _update_opponent_hand(hand: MyHand, count: int) -> void:
	if hand == null:
		return

	# 清除现有手牌
	hand.clear_cards()

	# 创建背面朝上的卡牌（最多显示7张）
	var display_count = min(count, 7)
	for i in range(display_count):
		_create_card("card_back", hand, false)


## 更新卡组堆叠
func _update_deck_stack(stack: CardStack, cards: Array, count: int) -> void:
	if stack == null:
		return

	# 清除现有卡牌
	stack.clear_cards()

	var display_count: int
	if cards.size() > 0:
		# 用真实卡片数据创建牌堆（背面显示）
		display_count = min(cards.size(), stack.max_stack_display)
		for i in range(display_count):
			var card_info = cards[cards.size() - display_count + i]
			var definition_id = card_info.get("definition_id", "card_back")
			var card = _create_card(definition_id, stack, false, card_info)
			if card != null:
				card.set_meta("instance_id", card_info.get("instance_id", 0))
	else:
		# 回退：只用卡背假卡片
		display_count = min(count, stack.max_stack_display)
		for i in range(display_count):
			_create_card("card_back", stack, false)

	# 更新数量显示标签
	if stack.count_label:
		stack.count_label.text = str(count)
		stack.count_label.visible = count > 0


## 更新墓地堆叠
func _update_grave_stack(stack: CardStack, cards: Array) -> void:
	if stack == null:
		return

	# 清除现有卡牌
	stack.clear_cards()

	# 创建墓地卡牌（正面朝上，最多显示5张）
	var display_count = min(cards.size(), 5)
	for i in range(display_count):
		var card_info = cards[cards.size() - display_count + i]  # 显示最新的卡牌
		if typeof(card_info) != TYPE_DICTIONARY:
			continue
		var definition_id = card_info.get("definition_id", "")
		if definition_id == "":
			continue

		var card = _create_card(definition_id, stack, true, card_info)
		if card != null:
			card.set_meta("instance_id", card_info.get("instance_id", 0))

	# 更新数量显示标签
	if stack.count_label:
		stack.count_label.text = str(cards.size())
		stack.count_label.visible = cards.size() > 0


## 更新战场格子（前场或后场）
func _update_field_slots(field: HBoxContainer, cards: Array, face_up: bool) -> void:
	if field == null:
		return

	for i in range(5):
		var slot = field.get_child(i)
		if not (slot is BattleFieldSlot):
			continue

		# 清除现有卡片
		slot.clear_cards()

		# 获取该 slot 的卡牌数据
		if i >= cards.size():
			continue

		var card_info = cards[i]
		if card_info == null or typeof(card_info) != TYPE_DICTIONARY:
			continue

		var definition_id = card_info.get("definition_id", "")
		if definition_id == "":
			continue

		var card = _create_card(definition_id, slot, face_up, card_info)
		if card != null:
			card.set_meta("instance_id", card_info.get("instance_id", 0))
			card.set_meta("definition_id", definition_id)
			card.set_meta("current_attack", card_info.get("current_attack", 0))


## 更新费用区
func _update_cost_zone(cost_zone: CostZone, cards: Array, face_up: bool) -> void:
	if cost_zone == null:
		return

	# 清除现有卡片
	cost_zone.clear_cards()

	# 创建新卡片
	for card_info in cards:
		if typeof(card_info) != TYPE_DICTIONARY:
			continue
		var definition_id = card_info.get("definition_id", "")
		if definition_id == "":
			continue

		var card = _create_card(definition_id, cost_zone, face_up, card_info)
		if card != null:
			card.set_meta("instance_id", card_info.get("instance_id", 0))
			card.set_meta("definition_id", definition_id)
			card.set_meta("current_attack", card_info.get("current_attack", 0))


## 缓存卡牌信息
func _cache_cards(cards: Array) -> void:
	for card_info in cards:
		if typeof(card_info) != TYPE_DICTIONARY:
			continue
		var instance_id = card_info.get("instance_id", 0)
		if instance_id != 0:
			card_cache[instance_id] = card_info


## 更新状态面板
func _update_status_panel(panel: VBoxContainer, hp: int, rp: int, label_prefix: String) -> void:
	var hp_label = panel.get_node_or_null("HPLabel")
	var rp_label = panel.get_node_or_null("RealPointsLabel")
	var title_label = panel.get_node_or_null("TitleLabel")

	if hp_label:
		hp_label.text = "HP: %d" % hp
	if rp_label:
		rp_label.text = "RP: %d" % rp
	if title_label:
		title_label.text = label_prefix


var current_popup: PopupPanel = null

## 菜单按钮点击
func _on_menu_pressed() -> void:
	# 关闭已打开的菜单
	if current_popup != null:
		current_popup.queue_free()
		current_popup = null
		return

	current_popup = PopupPanel.new()
	current_popup.size = Vector2(200, 150)
	current_popup.exclusive = false
	current_popup.unresizable = true

	# 确保弹窗可以接收输入
	current_popup.mouse_filter = Control.MOUSE_FILTER_STOP

	var vbox = VBoxContainer.new()
	vbox.alignment = BoxContainer.ALIGNMENT_CENTER
	vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	vbox.size_flags_vertical = Control.SIZE_EXPAND_FILL

	var surrender_btn = Button.new()
	surrender_btn.text = "投降"
	surrender_btn.pressed.connect(func(): _on_surrender(current_popup))

	var settings_btn = Button.new()
	settings_btn.text = "设置"
	settings_btn.pressed.connect(func(): _on_settings(current_popup))

	var cancel_btn = Button.new()
	cancel_btn.text = "取消"
	cancel_btn.pressed.connect(func():
		if current_popup != null:
			current_popup.hide()
			current_popup.queue_free()
			current_popup = null
	)

	vbox.add_child(surrender_btn)
	vbox.add_child(settings_btn)
	vbox.add_child(cancel_btn)

	current_popup.add_child(vbox)
	add_child(current_popup)

	# 使用 call_deferred 确保正确显示
	call_deferred("_show_popup", current_popup)

func _show_popup(popup: PopupPanel) -> void:
	if popup != null and is_instance_valid(popup):
		popup.popup_centered()

## 关闭菜单（点击外部时调用）
func _close_menu() -> void:
	if current_popup != null:
		current_popup.queue_free()
		current_popup = null


## 投降
func _on_surrender(popup: PopupPanel) -> void:
	if popup != null and is_instance_valid(popup):
		popup.hide()
		popup.queue_free()
	current_popup = null
	_print("Player surrendered")

	# 发送投降命令到服务器
	if network_manager:
		# 这里需要实现投降消息发送
		pass

	# 显示投降确认弹窗
	var confirm_popup = AcceptDialog.new()
	confirm_popup.title = "投降"
	confirm_popup.dialog_text = "你已投降，游戏结束。"
	confirm_popup.confirmed.connect(_return_to_lobby)
	add_child(confirm_popup)
	confirm_popup.popup_centered()


## 设置
func _on_settings(popup: PopupPanel) -> void:
	if popup != null and is_instance_valid(popup):
		popup.hide()
		popup.queue_free()
	current_popup = null
	_print("Settings opened (no action)")
	# 设置功能暂无操作


## 返回大厅
func _return_to_lobby() -> void:
	_print("Returning to lobby...")

	# 断开连接
	if network_manager:
		network_manager.disconnect_from_server()

	# 切换场景
	if scene_manager:
		scene_manager.change_scene("lobby", {})


# ============================================================================
# 事件处理 — 驱动动画
# ============================================================================

## 处理 StateUpdate 中携带的近期事件
func _process_recent_events(events: Array) -> void:
	if events.is_empty():
		return
	for event in events:
		if typeof(event) != TYPE_DICTIONARY:
			continue
		# 每个 event 是 serde 序列化的 CoreGameEvent JSON，包含单个 variant key
		for event_type in event.keys():
			var event_data = event[event_type]
			match event_type:
				"DrawCard":
					var player = event_data.get("player", "")
					var iid = event_data.get("instance_id", 0)
					_print("Event: %s draws card instance_id=%d" % [player, iid])
					# 动画触发点：根据 player 判断是自己还是对手
					# 自己抽卡 → 可以从 state.your_state.hand 找到该卡做动画
					# 对手抽卡 → 通用"对手抽卡"动画

				"CardSummoned":
					var iid = event_data.get("instance_id", 0)
					var to = event_data.get("to", {})
					_print("Event: CardSummoned instance_id=%d to=%s" % [iid, str(to)])

				"CardMoved":
					var iid = event_data.get("instance_id", 0)
					var from_zone = event_data.get("from", {})
					var to_zone = event_data.get("to", {})
					_print("Event: CardMoved instance_id=%d from=%s to=%s" % [iid, str(from_zone), str(to_zone)])

				"CardExposed":
					var iid = event_data.get("instance_id", 0)
					_print("Event: CardExposed instance_id=%d (card turned face-up, entering cost zone)" % iid)

				"CardDestroyed":
					var iid = event_data.get("instance_id", 0)
					_print("Event: CardDestroyed instance_id=%d" % iid)

				"PhaseChanged":
					var new_phase = event_data.get("new_phase", "")
					_print("Event: PhaseChanged → %s" % new_phase)

				"TurnChanged":
					var new_player = event_data.get("new_active_player", "")
					var turn = event_data.get("turn_number", 0)
					_print("Event: TurnChanged → player=%s turn=%d" % [new_player, turn])

				"HpChanged":
					var player = event_data.get("player", "")
					var old_hp = event_data.get("old_hp", 0)
					var new_hp = event_data.get("new_hp", 0)
					_print("Event: HpChanged player=%s %d→%d" % [player, old_hp, new_hp])

				"RealPointChanged":
					var player = event_data.get("player", "")
					var old_rp = event_data.get("old_rp", 0)
					var new_rp = event_data.get("new_rp", 0)
					_print("Event: RealPointChanged player=%s %d→%d" % [player, old_rp, new_rp])

				"EffectActivated":
					var iid = event_data.get("instance_id", 0)
					var ekey = event_data.get("effect_key", "")
					_print("Event: EffectActivated instance_id=%d effect_key=%s" % [iid, str(ekey)])

				"AttackDeclared":
					var attacker = event_data.get("attacker", 0)
					_print("Event: AttackDeclared attacker=%d" % attacker)

				_:
					_print("Event: %s data=%s" % [event_type, str(event_data)])


## 打印日志
func _print(msg: String) -> void:
	print("[GameBoard] %s" % msg)


# ============================================================================
# Action / Recovery 交互系统
# ============================================================================

## 交互状态
var _waiting_action: bool = false
var _waiting_recovery: bool = false
var _available_actions: Array = []
var _action_timeout: int = 60
var _recovery_count: int = 0
var _recovery_options: Array = []
var _selected_recovery_cards: Array[int] = []
var _highlighted_cards: Array = []
var _highlighted_slots: Array = []
var _action_panel: Panel = null


## action_request 处理
func _on_action_requested(available_actions: Array, timeout_secs: int) -> void:
	_print("Action requested: %d actions, timeout=%ds" % [available_actions.size(), timeout_secs])
	_clear_action_ui()
	_waiting_action = true
	_available_actions = available_actions
	_action_timeout = timeout_secs

	var has_pass := false
	var has_surrender := false
	var play_card_actions: Array = []
	var declare_attack_actions: Array = []
	var activate_effect_actions: Array = []

	for action in available_actions:
		if typeof(action) != TYPE_DICTIONARY:
			continue
		var action_type: String = action.get("action_type", "")
		match action_type:
			"pass":
				has_pass = true
			"surrender":
				has_surrender = true
			"play_card":
				play_card_actions.append(action)
			"declare_attack":
				declare_attack_actions.append(action)
			"activate_effect":
				activate_effect_actions.append(action)

	# 收集同一张卡的所有 play_card target_zones
	var card_play_options: Dictionary = {}
	for action in play_card_actions:
		var instance_id: int = action.get("instance_id", 0)
		if not card_play_options.has(instance_id):
			card_play_options[instance_id] = []
		card_play_options[instance_id].append(action)

	# 高亮可出的手牌（点击后弹出费用支付窗）
	for instance_id in card_play_options.keys():
		var card := _find_card_in_hand(instance_id)
		if card:
			_highlight_card(card)
			card.gui_input.connect(_on_play_card_clicked.bind(card, card_play_options[instance_id]))

	# 收集同一张卡的所有 declare_attack targets
	var card_attack_options: Dictionary = {}
	for action in declare_attack_actions:
		var attacker_id: int = action.get("attacker_id", 0)
		if not card_attack_options.has(attacker_id):
			card_attack_options[attacker_id] = []
		card_attack_options[attacker_id].append(action)

	# 高亮可攻击的前场卡
	for attacker_id in card_attack_options.keys():
		var card := _find_card_on_field(attacker_id)
		if card:
			_highlight_card(card)
			card.gui_input.connect(_on_declare_attack_clicked.bind(card, card_attack_options[attacker_id]))

	# 高亮有可发动效果的卡
	for action in activate_effect_actions:
		var instance_id: int = action.get("instance_id", 0)
		var effect_key: String = action.get("effect_key", "")
		var card := _find_card_on_field_or_hand(instance_id)
		if card:
			_highlight_card(card)
			card.gui_input.connect(_on_activate_effect_clicked.bind(card, instance_id, effect_key))

	# 显示操作面板
	_show_action_panel(has_pass, has_surrender)


## chain_action_request 处理
func _on_chain_action_requested(chain_size: int, available_effects: Array, timeout_secs: int) -> void:
	_print("Chain action requested: chain_size=%d, effects=%d" % [chain_size, available_effects.size()])
	_clear_action_ui()

	# Highlight chainable cards
	for effect in available_effects:
		if typeof(effect) != TYPE_DICTIONARY:
			continue
		var instance_id: int = effect.get("instance_id", 0)
		var effect_key: String = effect.get("effect_key", "")
		var card := _find_card_on_field_or_hand(instance_id)
		if card:
			_highlight_card(card)
			card.gui_input.connect(_on_chain_activate_clicked.bind(card, instance_id, effect_key))

	# Show chain panel
	_show_chain_panel(chain_size, timeout_secs)


## 玩家点击可连锁的效果卡
func _on_chain_activate_clicked(event: InputEvent, card: Card, instance_id: int, effect_key: String) -> void:
	if not (event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT):
		return
	_print("Sending chain_activate: instance_id=%d, effect_key=%s" % [instance_id, effect_key])
	if network_manager:
		network_manager.send_chain_activate(instance_id, effect_key)
	_clear_action_ui()


## 显示连锁面板
func _show_chain_panel(chain_size: int, timeout_secs: int) -> void:
	if _action_panel and is_instance_valid(_action_panel):
		_action_panel.queue_free()

	_action_panel = Panel.new()
	_action_panel.size = Vector2(400, 120)
	_action_panel.position = Vector2((size.x - _action_panel.size.x) / 2, size.y - _action_panel.size.y - 20)

	var vbox = VBoxContainer.new()
	vbox.alignment = BoxContainer.ALIGNMENT_CENTER
	vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	vbox.size_flags_vertical = Control.SIZE_EXPAND_FILL

	var label = Label.new()
	label.text = "连锁窗口 - 链栈: %d links (超时: %ds)" % [chain_size, timeout_secs]
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(label)

	var hbox = HBoxContainer.new()
	hbox.alignment = BoxContainer.ALIGNMENT_CENTER

	var pass_btn = Button.new()
	pass_btn.text = "跳过 (Pass)"
	pass_btn.pressed.connect(func():
		if network_manager:
			network_manager.send_chain_pass()
		_clear_action_ui()
	)
	hbox.add_child(pass_btn)

	vbox.add_child(hbox)
	_action_panel.add_child(vbox)
	add_child(_action_panel)


## recovery_request 处理
func _on_recovery_requested(count: int, options: Array) -> void:
	_print("Recovery requested: select %d from %d options" % [count, options.size()])
	_clear_action_ui()
	_waiting_recovery = true
	_recovery_count = count
	_recovery_options = options
	_selected_recovery_cards.clear()

	for option in options:
		if typeof(option) != TYPE_DICTIONARY:
			continue
		var instance_id: int = option.get("instance_id", 0)
		var card := _find_card_in_hand(instance_id)
		if card:
			_highlight_card(card)
			card.gui_input.connect(_on_recovery_card_clicked.bind(card, instance_id))

	_show_recovery_info(count)


## 玩家点击可出的手牌 → 弹出费用支付窗
var _pending_play_card_data: Dictionary = {}

func _on_play_card_clicked(event: InputEvent, card: Card, actions: Array) -> void:
	if not _waiting_action:
		return
	if not (event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT):
		return

	# 如果只有一个 target_zone 选项，弹出费用窗
	if actions.size() == 1:
		var action: Dictionary = actions[0]
		_show_cost_payment_popup(action, "play_card")
		return

	# 多个 target_zone 选项：先高亮格子，点击格子后弹费用窗
	_clear_slot_highlights()
	for action in actions:
		var target_zone: Dictionary = action.get("target_zone", {})
		var zone_type: String = target_zone.get("zone_type", "")
		var slot: int = target_zone.get("slot", 0)
		var container := _get_field_container(zone_type)
		if container and slot < container.get_child_count():
			var slot_node = container.get_child(slot)
			if slot_node is BattleFieldSlot:
				_highlight_slot(slot_node)
				var instance_id: int = actions[0].get("instance_id", 0)
				slot_node.gui_input.connect(_on_target_slot_clicked.bind(slot_node, target_zone, instance_id))


## 玩家点击目标格子 → 弹出费用支付窗
func _on_target_slot_clicked(event: InputEvent, slot: BattleFieldSlot, zone_data: Dictionary, instance_id: int) -> void:
	if not _waiting_action:
		return
	if not (event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT):
		return
	# Build action data and show cost popup
	var action_data = {
		"instance_id": instance_id,
		"target_zone": zone_data,
	}
	_show_cost_payment_popup(action_data, "play_card")


## 发送 PlayCard action（带费用）
func _send_play_card_with_cost(instance_id: int, zone_data: Dictionary, cost_hand: Array, cost_rp: int) -> void:
	var zone_type: String = zone_data.get("zone_type", "")
	var slot: int = zone_data.get("slot", 0)
	_print("Sending play_card: instance_id=%d, zone=%s, slot=%d, rp=%d, hand=%s" % [instance_id, zone_type, slot, cost_rp, str(cost_hand)])
	if network_manager:
		network_manager.send_action_play_card(instance_id, zone_type, slot, cost_hand, cost_rp)
	_clear_action_ui()


## 玩家点击可发动效果的卡
func _on_activate_effect_clicked(event: InputEvent, card: Card, instance_id: int, effect_key: String) -> void:
	if not _waiting_action:
		return
	if not (event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT):
		return
	_print("Sending activate_effect: instance_id=%d, effect_key=%s" % [instance_id, effect_key])
	if network_manager:
		network_manager.send_activate_effect(instance_id, effect_key)
	_clear_action_ui()


## 在手牌或场上查找卡牌
func _find_card_on_field_or_hand(instance_id: int) -> Card:
	var card := _find_card_in_hand(instance_id)
	if card:
		return card
	card = _find_card_on_field(instance_id)
	if card:
		return card
	# Also search cost zone (for cost zone activations)
	for child in player_cost.get_children():
		if child is Card and child.get_meta("instance_id", 0) == instance_id:
			return child
	for child in opponent_cost.get_children():
		if child is Card and child.get_meta("instance_id", 0) == instance_id:
			return child
	return null


# ============================================================================
# 费用支付弹窗
# ============================================================================

var _cost_popup: Panel = null
var _cost_popup_action_data: Dictionary = {}
var _cost_popup_action_type: String = ""

func _show_cost_payment_popup(action_data: Dictionary, action_type: String) -> void:
	# 先清除 slot 高亮
	_clear_slot_highlights()

	# 获取费用信息
	var instance_id: int = action_data.get("instance_id", 0)
	var card_cost: int = action_data.get("cost", 0)

	# 尝试从卡牌缓存计算费用
	if card_cost <= 0:
		var card_info = card_cache.get(instance_id, {})
		if card_info.is_empty():
			card_info = _get_card_info_from_board(instance_id)
		var def_id: String = card_info.get("definition_id", "")
		if def_id != "":
			var card_def = _load_card_definition(def_id)
			card_cost = card_def.get("cost", 0)
	# 安全兜底
	if typeof(card_cost) != TYPE_INT or card_cost < 0:
		card_cost = 0

	# 0 费卡直接发送，不弹窗
	if card_cost <= 0:
		if action_type == "play_card":
			_send_play_card_with_cost(instance_id, action_data.get("target_zone", {}), [], 0)
		elif action_type == "activate_effect":
			network_manager.send_activate_effect(instance_id, action_data.get("effect_key", ""))
		return

	_cost_popup_action_data = action_data
	_cost_popup_action_type = action_type

	var my_state = {}
	if action_type == "play_card":
		my_state = _get_my_state_dict()

	var available_rp: int = my_state.get("real_point", 0)

	# 可用来支付的手牌（排除正在登场的卡）
	var payable_hand: Array = []
	var hand_cards = _safe_array(my_state.get("hand", []))
	for card_info in hand_cards:
		if typeof(card_info) == TYPE_DICTIONARY:
			var iid = card_info.get("instance_id", 0)
			if iid != instance_id:
				payable_hand.append(card_info)

	# 移除旧弹窗
	if _cost_popup and is_instance_valid(_cost_popup):
		_cost_popup.queue_free()
	_cost_popup = null

	# 创建弹窗
	var popup = Panel.new()
	popup.size = Vector2(420, 320)
	popup.position = Vector2((size.x - popup.size.x) / 2, (size.y - popup.size.y) / 2)
	popup.mouse_filter = Control.MOUSE_FILTER_STOP

	var vbox = VBoxContainer.new()
	vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	vbox.size_flags_vertical = Control.SIZE_EXPAND_FILL
	vbox.add_theme_constant_override("separation", 10)

	# 标题
	var title = Label.new()
	title.text = "支付费用 — 卡费: %d" % card_cost
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(title)

	# RP 滑动条区域
	var rp_label = Label.new()
	rp_label.text = "使用 RealPoint: 0 (可用: %d)" % available_rp
	rp_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER

	var rp_slider = HSlider.new()
	rp_slider.min_value = 0
	rp_slider.max_value = mini(card_cost, available_rp)
	rp_slider.step = 1
	rp_slider.value = mini(card_cost, available_rp)  # 默认全部用 RP
	rp_slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	vbox.add_child(rp_label)
	vbox.add_child(rp_slider)

	# 手牌支付区域
	var hand_label = Label.new()
	hand_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER

	var hand_vbox = VBoxContainer.new()
	var checkboxes: Array = []
	var card_labels: Array = []

	for card_info in payable_hand:
		var hbox = HBoxContainer.new()
		var cb = CheckBox.new()
		var label = Label.new()
		var def_id: String = card_info.get("definition_id", "")
		label.text = "%s (Instance %d)" % [def_id, card_info.get("instance_id", 0)]
		hbox.add_child(cb)
		hbox.add_child(label)
		hand_vbox.add_child(hbox)
		checkboxes.append(cb)
		card_labels.append({"cb": cb, "info": card_info})

	vbox.add_child(hand_label)
	vbox.add_child(hand_vbox)

	# 按钮区域
	var btn_hbox = HBoxContainer.new()
	btn_hbox.alignment = BoxContainer.ALIGNMENT_CENTER

	var cancel_btn = Button.new()
	cancel_btn.text = "取消"
	cancel_btn.pressed.connect(func():
		if _cost_popup and is_instance_valid(_cost_popup):
			_cost_popup.queue_free()
		_cost_popup = null
		_clear_action_ui()
	)

	var confirm_btn = Button.new()
	confirm_btn.text = "确认支付"
	confirm_btn.disabled = true

	btn_hbox.add_child(cancel_btn)
	btn_hbox.add_child(confirm_btn)
	vbox.add_child(btn_hbox)

	popup.add_child(vbox)
	add_child(popup)
	_cost_popup = popup

	# 联动逻辑
	var update_ui = func():
		var rp_used: int = int(rp_slider.value)
		rp_label.text = "使用 RealPoint: %d (可用: %d)" % [rp_used, available_rp]
		var need_hand = card_cost - rp_used
		hand_label.text = "还需选择: %d 张手牌" % need_hand

		# 限制可勾选数量
		var checked_count = 0
		for i in range(checkboxes.size()):
			var cb: CheckBox = checkboxes[i]
			if cb.button_pressed:
				checked_count += 1
		for i in range(checkboxes.size()):
			var cb: CheckBox = checkboxes[i]
			if not cb.button_pressed and checked_count >= need_hand:
				cb.disabled = true
			else:
				cb.disabled = false

		# 确认按钮
		confirm_btn.disabled = (rp_used + checked_count) != card_cost

	# 连接信号
	rp_slider.value_changed.connect(func(_v): update_ui.call())
	for cb in checkboxes:
		cb.toggled.connect(func(_pressed): update_ui.call())

	confirm_btn.pressed.connect(func():
		var rp_used: int = int(rp_slider.value)
		var selected_hand: Array[int] = []
		for item in card_labels:
			if item["cb"].button_pressed:
				selected_hand.append(item["info"].get("instance_id", 0))

		if _cost_popup and is_instance_valid(_cost_popup):
			_cost_popup.queue_free()
		_cost_popup = null

		if _cost_popup_action_type == "play_card":
			var target_zone = _cost_popup_action_data.get("target_zone", {})
			_send_play_card_with_cost(instance_id, target_zone, selected_hand, rp_used)
	)

	update_ui.call()


## 从卡片缓存和手牌区域获取卡片信息
func _get_card_info_from_board(instance_id: int) -> Dictionary:
	# Check card_cache first
	if card_cache.has(instance_id):
		return card_cache.get(instance_id, {})
	# Search hand
	for card in player_hand._held_cards:
		if card.get_meta("instance_id", 0) == instance_id:
			return {
				"instance_id": instance_id,
				"definition_id": card.get_meta("definition_id", ""),
				"current_attack": card.get_meta("current_attack", 0),
			}
	return {}

func _get_my_state_dict() -> Dictionary:
	var state = {}
	if network_manager and network_manager.has_signal("state_update"):
		pass  # We don't store the last state directly; extract from UI
	# try from GameState autoload
	var gs = get_node_or_null("/root/GameState")
	if gs:
		state = gs.my_state
	return state


## 玩家点击可攻击的前场卡
func _on_declare_attack_clicked(event: InputEvent, card: Card, actions: Array) -> void:
	if not _waiting_action:
		return
	if not (event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT):
		return

	var attacker_id: int = actions[0].get("attacker_id", 0)

	# 收集所有 target 选项
	var has_direct := false
	var slot_targets: Array[int] = []
	for action in actions:
		var target: Dictionary = action.get("target", {})
		var target_type: String = target.get("target_type", "")
		if target_type == "direct":
			has_direct = true
		elif target_type == "slot":
			var slot_index: int = target.get("slot_index", -1)
			if slot_index >= 0:
				slot_targets.append(slot_index)

	_show_attack_target_panel(attacker_id, has_direct, slot_targets)


## 发送 DeclareAttack action
func _send_declare_attack(attacker_id: int, target_type: String, slot_index: int = -1) -> void:
	_print("Sending declare_attack: attacker_id=%d, target=%s" % [attacker_id, target_type])
	if network_manager:
		network_manager.send_action_declare_attack(attacker_id, target_type, slot_index)
	_clear_action_ui()


## 玩家点击可回收的手牌
func _on_recovery_card_clicked(event: InputEvent, card: Card, instance_id: int) -> void:
	if not _waiting_recovery:
		return
	if not (event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT):
		return

	if instance_id in _selected_recovery_cards:
		_selected_recovery_cards.erase(instance_id)
		card.modulate = Color(1.2, 1.2, 0.8)  # 恢复高亮
	else:
		_selected_recovery_cards.append(instance_id)
		card.modulate = Color(0.8, 1.2, 0.8)  # 选中绿色

	# 选够后自动发送
	if _selected_recovery_cards.size() >= _recovery_count:
		_print("Recovery selection complete: %s" % str(_selected_recovery_cards))
		if network_manager:
			network_manager.send_recovery_selection(_selected_recovery_cards)
		_clear_action_ui()


## 显示操作面板 (Pass / Surrender)
func _show_action_panel(has_pass: bool, has_surrender: bool) -> void:
	if _action_panel and is_instance_valid(_action_panel):
		_action_panel.queue_free()

	_action_panel = Panel.new()
	_action_panel.size = Vector2(300, 60)
	_action_panel.position = Vector2(
		(size.x - _action_panel.size.x) / 2,
		size.y - _action_panel.size.y - 20
	)

	var hbox := HBoxContainer.new()
	hbox.alignment = BoxContainer.ALIGNMENT_CENTER
	hbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hbox.size_flags_vertical = Control.SIZE_EXPAND_FILL

	if has_pass:
		var pass_btn := Button.new()
		pass_btn.text = "跳过 (Pass)"
		pass_btn.pressed.connect(func():
			if network_manager:
				network_manager.send_action_pass()
			_clear_action_ui()
		)
		hbox.add_child(pass_btn)

	if has_surrender:
		var surrender_btn := Button.new()
		surrender_btn.text = "投降 (Surrender)"
		surrender_btn.pressed.connect(func():
			if network_manager:
				network_manager.send_action_surrender()
			_clear_action_ui()
		)
		hbox.add_child(surrender_btn)

	_action_panel.add_child(hbox)
	add_child(_action_panel)


## 显示攻击目标选择面板
func _show_attack_target_panel(attacker_id: int, has_direct: bool, slot_targets: Array) -> void:
	if _action_panel and is_instance_valid(_action_panel):
		_action_panel.queue_free()

	_action_panel = Panel.new()
	_action_panel.size = Vector2(400, 120)
	_action_panel.position = Vector2(
		(size.x - _action_panel.size.x) / 2,
		size.y - _action_panel.size.y - 20
	)

	var vbox := VBoxContainer.new()
	vbox.alignment = BoxContainer.ALIGNMENT_CENTER
	vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	vbox.size_flags_vertical = Control.SIZE_EXPAND_FILL

	var label := Label.new()
	label.text = "选择攻击目标"
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(label)

	var hbox := HBoxContainer.new()
	hbox.alignment = BoxContainer.ALIGNMENT_CENTER

	if has_direct:
		var direct_btn := Button.new()
		direct_btn.text = "直接攻击"
		direct_btn.pressed.connect(func():
			_send_declare_attack(attacker_id, "direct")
		)
		hbox.add_child(direct_btn)

	for slot in slot_targets:
		var slot_btn := Button.new()
		slot_btn.text = "前场 %d" % slot
		var captured_slot: int = slot
		slot_btn.pressed.connect(func():
			_send_declare_attack(attacker_id, "slot", captured_slot)
		)
		hbox.add_child(slot_btn)

	vbox.add_child(hbox)
	_action_panel.add_child(vbox)
	add_child(_action_panel)


## 显示 Recovery 提示信息
func _show_recovery_info(count: int) -> void:
	if _action_panel and is_instance_valid(_action_panel):
		_action_panel.queue_free()

	_action_panel = Panel.new()
	_action_panel.size = Vector2(400, 60)
	_action_panel.position = Vector2(
		(size.x - _action_panel.size.x) / 2,
		size.y - _action_panel.size.y - 20
	)

	var label := Label.new()
	label.text = "回收阶段：请选择 %d 张卡 (点击手牌选中/取消)" % count
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	label.size = _action_panel.size
	_action_panel.add_child(label)
	add_child(_action_panel)


## 清除所有 action/recovery UI
func _clear_action_ui() -> void:
	_waiting_action = false
	_waiting_recovery = false
	_available_actions.clear()
	_recovery_options.clear()
	_selected_recovery_cards.clear()

	# 断开手牌高亮卡上的交互信号
	for card in _highlighted_cards:
		if is_instance_valid(card):
			card.modulate = Color.WHITE
			for conn in card.gui_input.get_connections():
				var method: String = conn.callable.get_method()
				if method in ["_on_play_card_clicked", "_on_declare_attack_clicked", "_on_recovery_card_clicked"]:
					card.gui_input.disconnect(conn.callable)
	_highlighted_cards.clear()

	# 清除格子高亮并断开信号
	_clear_slot_highlights()

	# 移除操作面板
	if _action_panel and is_instance_valid(_action_panel):
		_action_panel.queue_free()
	_action_panel = null


## 清除格子高亮并断开信号
func _clear_slot_highlights() -> void:
	for slot in _highlighted_slots:
		if is_instance_valid(slot):
			var bg = slot.get_node_or_null("Background")
			if bg:
				bg.modulate = Color.WHITE
			for conn in slot.gui_input.get_connections():
				if conn.callable.get_method() == "_on_target_slot_clicked":
					slot.gui_input.disconnect(conn.callable)
	_highlighted_slots.clear()


## 高亮卡牌
func _highlight_card(card: Card) -> void:
	if card not in _highlighted_cards:
		_highlighted_cards.append(card)
		card.modulate = Color(1.2, 1.2, 0.8)


## 高亮格子
func _highlight_slot(slot: BattleFieldSlot) -> void:
	if slot not in _highlighted_slots:
		_highlighted_slots.append(slot)
		var bg = slot.get_node_or_null("Background")
		if bg:
			bg.modulate = Color(1.5, 1.5, 1.0)


## 在手牌中查找卡牌
func _find_card_in_hand(instance_id: int) -> Card:
	for card in player_hand._held_cards:
		if card.get_meta("instance_id", 0) == instance_id:
			return card
	return null


## 在场地上查找卡牌（自己前场/后场）
func _find_card_on_field(instance_id: int) -> Card:
	for i in range(5):
		var slot = player_front_field.get_child(i)
		if slot is BattleFieldSlot:
			var card = slot.get_card()
			if card and card.get_meta("instance_id", 0) == instance_id:
				return card
	for i in range(5):
		var slot = player_back_field.get_child(i)
		if slot is BattleFieldSlot:
			var card = slot.get_card()
			if card and card.get_meta("instance_id", 0) == instance_id:
				return card
	return null


## 根据 zone_type 获取场地容器
func _get_field_container(zone_type: String) -> HBoxContainer:
	match zone_type:
		"front":
			return player_front_field
		"back":
			return player_back_field
		_:
			return null
