## 对局主场景
## 集成所有游戏区域：手牌、场地、卡组、墓地、费用区

extends Control

signal return_to_lobby

## 玩家数据
var player_hp: int = 20
var player_real_points: int = 0
var opponent_hp: int = 20
var opponent_real_points: int = 0

## 场景引用 - 对手区域
@onready var opponent_hand: Hand = $OpponentArea/OpponentHand
@onready var opponent_deck: CardStack = $OpponentArea/OpponentFieldArea/OpponentStacks/OpponentDeck
@onready var opponent_grave: CardStack = $OpponentArea/OpponentFieldArea/OpponentStacks/OpponentGrave
@onready var opponent_front_field: HBoxContainer = $OpponentArea/OpponentFieldArea/OpponentBattleField/OpponentFrontField
@onready var opponent_back_field: HBoxContainer = $OpponentArea/OpponentFieldArea/OpponentBattleField/OpponentBackField
@onready var opponent_cost: CostZone = $OpponentArea/OpponentFieldArea/OpponentStatus/OpponentCost
@onready var opponent_status: VBoxContainer = $OpponentArea/OpponentFieldArea/OpponentStatus/OpponentStatusPanel

## 场景引用 - 自己区域
@onready var player_hand: Hand = $PlayerArea/PlayerHand
@onready var player_deck: CardStack = $PlayerArea/PlayerFieldArea/PlayerStacks/PlayerDeck
@onready var player_grave: CardStack = $PlayerArea/PlayerFieldArea/PlayerStacks/PlayerGrave
@onready var player_front_field: HBoxContainer = $PlayerArea/PlayerFieldArea/PlayerBattleField/PlayerFrontField
@onready var player_back_field: HBoxContainer = $PlayerArea/PlayerFieldArea/PlayerBattleField/PlayerBackField
@onready var player_cost: CostZone = $PlayerArea/PlayerFieldArea/PlayerStatus/PlayerCost
@onready var player_status: VBoxContainer = $PlayerArea/PlayerFieldArea/PlayerStatus/PlayerStatusPanel

## UI 引用
@onready var menu_button: Button = $UIOverlay/MenuButton

func _ready():
	menu_button.pressed.connect(_on_menu_pressed)
	_initialize_fields()
	_initialize_card_stacks()


## 初始化战场格子
func _initialize_fields() -> void:
	# 设置对手前场格子
	for i in range(5):
		var slot = opponent_front_field.get_child(i)
		if slot is BattleFieldSlot:
			slot.field_type = BattleFieldSlot.FieldType.FRONT
			slot.slot_index = i

	# 设置对手后场格子
	for i in range(5):
		var slot = opponent_back_field.get_child(i)
		if slot is BattleFieldSlot:
			slot.field_type = BattleFieldSlot.FieldType.BACK
			slot.slot_index = i

	# 设置自己前场格子
	for i in range(5):
		var slot = player_front_field.get_child(i)
		if slot is BattleFieldSlot:
			slot.field_type = BattleFieldSlot.FieldType.FRONT
			slot.slot_index = i

	# 设置自己后场格子
	for i in range(5):
		var slot = player_back_field.get_child(i)
		if slot is BattleFieldSlot:
			slot.field_type = BattleFieldSlot.FieldType.BACK
			slot.slot_index = i


## 初始化卡组/墓地
func _initialize_card_stacks() -> void:
	# 设置卡组类型
	opponent_deck.stack_type = CardStack.StackType.DECK
	player_deck.stack_type = CardStack.StackType.DECK
	opponent_grave.stack_type = CardStack.StackType.GRAVE
	player_grave.stack_type = CardStack.StackType.GRAVE

	# 设置卡组为卡背朝上
	opponent_deck.card_face_up = false
	player_deck.card_face_up = false
	opponent_grave.card_face_up = true
	player_grave.card_face_up = true


## 设置手牌（外部调用）
func setup_hands(player_cards: Array, opponent_cards: Array) -> void:
	# 清空现有手牌
	player_hand.clear_cards()
	opponent_hand.clear_cards()

	# 添加玩家手牌（显示卡图）
	for card in player_cards:
		card.show_front = true
		player_hand.add_card(card)

	# 添加对手手牌（显示卡背）
	for card in opponent_cards:
		card.show_front = false
		opponent_hand.add_card(card)


## 更新状态显示
func update_status(is_player: bool, hp: int, real_points: int) -> void:
	if is_player:
		player_hp = hp
		player_real_points = real_points
		_update_status_panel(player_status, hp, real_points)
	else:
		opponent_hp = hp
		opponent_real_points = real_points
		_update_status_panel(opponent_status, hp, real_points)


func _update_status_panel(panel: VBoxContainer, hp: int, real_points: int) -> void:
	var hp_label = panel.get_node_or_null("HPLabel")
	var rp_label = panel.get_node_or_null("RealPointsLabel")

	if hp_label:
		hp_label.text = "HP: %d" % hp
	if rp_label:
		rp_label.text = "RP: %d" % real_points


## 菜单按钮点击
func _on_menu_pressed() -> void:
	# 显示菜单弹窗
	var popup = PopupPanel.new()
	popup.size = Vector2(200, 150)

	var vbox = VBoxContainer.new()
	vbox.alignment = BoxContainer.ALIGNMENT_CENTER

	var surrender_btn = Button.new()
	surrender_btn.text = "投降"
	surrender_btn.pressed.connect(func(): _on_surrender(popup))

	var return_btn = Button.new()
	return_btn.text = "返回大厅"
	return_btn.pressed.connect(func(): _on_return_to_lobby(popup))

	var cancel_btn = Button.new()
	cancel_btn.text = "取消"
	cancel_btn.pressed.connect(func(): popup.hide())

	vbox.add_child(surrender_btn)
	vbox.add_child(return_btn)
	vbox.add_child(cancel_btn)

	popup.add_child(vbox)
	add_child(popup)
	popup.popup_centered()


func _on_surrender(popup: PopupPanel) -> void:
	popup.hide()
	print("[GameBoard] Player surrendered")
	# TODO: 发送投降消息


func _on_return_to_lobby(popup: PopupPanel) -> void:
	popup.hide()
	emit_signal("return_to_lobby")
