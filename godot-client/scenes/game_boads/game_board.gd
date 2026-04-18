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
@onready var opponent_hand: Hand = $OpponentArea/OpponentTopRow/OpponentHand
@onready var opponent_deck: CardStack = $OpponentArea/OpponentMainRow/OpponentStacks/OpponentDeck
@onready var opponent_grave: CardStack = $OpponentArea/OpponentMainRow/OpponentStacks/OpponentGrave
@onready var opponent_front_field: HBoxContainer = $OpponentArea/OpponentMainRow/OpponentBattleField/OpponentFrontField
@onready var opponent_back_field: HBoxContainer = $OpponentArea/OpponentMainRow/OpponentBattleField/OpponentBackField
@onready var opponent_cost: CostZone = $OpponentArea/OpponentMainRow/OpponentCost
@onready var opponent_status: VBoxContainer = $OpponentArea/OpponentTopRow/OpponentStatus

## 场景引用 - 自己区域
@onready var player_hand: Hand = $PlayerArea/PlayerBottomRow/PlayerHand
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
	_print("Game started! Updating board...")

	var state = game_data.get("state", {})
	_update_from_state(state)


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

	# 更新自己状态
	_update_player_state(my_state)

	# 更新对手状态
	_update_opponent_state(opponent_state)


## 更新自己状态显示
func _update_player_state(state: Dictionary) -> void:
	if state.is_empty():
		return

	player_hp = state.get("hp", 20)
	player_real_points = state.get("real_point", 0)

	_update_status_panel(player_status, player_hp, player_real_points, "自己")

	# 更新手牌
	var hand_cards = state.get("hand", [])
	_update_hand(player_hand, hand_cards, true)

	# 更新卡组数量（在 CardStack 中显示卡背）
	var deck_count = state.get("deck_count", 0)
	_update_deck_stack(player_deck, deck_count)

	# 更新墓地
	var grave_cards = state.get("grave", [])
	_update_grave_stack(player_grave, grave_cards)

	# 缓存卡牌信息
	_cache_cards(hand_cards)
	_cache_cards(grave_cards)
	_cache_cards(state.get("front", []))
	_cache_cards(state.get("back", []))
	_cache_cards(state.get("cost_zone", []))


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

	# 更新对手卡组数量
	var deck_count = state.get("deck_count", 0)
	_update_deck_stack(opponent_deck, deck_count)

	# 更新对手墓地
	var grave_cards = state.get("grave", [])
	_update_grave_stack(opponent_grave, grave_cards)

	# 缓存卡牌信息
	_cache_cards(grave_cards)
	_cache_cards(state.get("front", []))
	_cache_cards(state.get("back", []))
	_cache_cards(state.get("cost_zone", []))


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
func _update_hand(hand: Hand, cards: Array, face_up: bool) -> void:
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
func _update_opponent_hand(hand: Hand, count: int) -> void:
	if hand == null:
		return

	# 清除现有手牌
	hand.clear_cards()

	# 创建背面朝上的卡牌（最多显示7张）
	var display_count = min(count, 7)
	for i in range(display_count):
		_create_card("card_back", hand, false)


## 更新卡组堆叠
func _update_deck_stack(stack: CardStack, count: int) -> void:
	if stack == null:
		return

	# 清除现有卡牌
	stack.clear_cards()

	# 创建卡背卡牌堆叠
	# 为了性能和视觉效果，最多只创建5张卡作为视觉表示
	var display_count = min(count, 5)
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


## 打印日志
func _print(msg: String) -> void:
	print("[GameBoard] %s" % msg)
