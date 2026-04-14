## 房间等待场景
## 显示玩家列表、准备状态、卡组选择

extends Control

signal back_pressed
signal ready_requested(deck_name: String)
signal cancel_ready_requested

const DECKS_DIR = "res://desks"

var _is_ready: bool = false
var _selected_deck: String = ""
var _players: Array = []

@onready var room_id_label: Label = $VBoxContainer/RoomIdLabel
@onready var player_list: VBoxContainer = $VBoxContainer/PlayerListScroll/PlayerList
@onready var deck_option: OptionButton = $VBoxContainer/DeckOption
@onready var ready_button: Button = $VBoxContainer/ButtonContainer/ReadyButton
@onready var back_button: Button = $VBoxContainer/ButtonContainer/BackButton

func _ready():
	ready_button.pressed.connect(_on_ready_pressed)
	back_button.pressed.connect(_on_back_pressed)
	deck_option.item_selected.connect(_on_deck_selected)

	# 加载卡组列表
	_load_decks()

	# 初始状态
	_update_ready_button_state()


## 设置房间ID
func set_room_id(room_id: String) -> void:
	room_id_label.text = "房间号: %s" % room_id


## 加载卡组列表
func _load_decks() -> void:
	deck_option.clear()
	deck_option.add_item("请选择卡组...")

	var dir = DirAccess.open(DECKS_DIR)
	if dir == null:
		push_error("[RoomWaiting] Cannot open decks directory: %s" % DECKS_DIR)
		return

	dir.list_dir_begin()
	var file_name = dir.get_next()
	while file_name != "":
		if file_name.ends_with(".json"):
			var deck_name = file_name.get_basename()
			deck_option.add_item(deck_name)
		file_name = dir.get_next()
	dir.list_dir_end()


## 更新玩家列表
func update_player_list(players: Array) -> void:
	_players = players

	# 清除旧列表
	for child in player_list.get_children():
		child.queue_free()

	# 创建玩家项
	for player in players:
		var item = _create_player_item(player)
		player_list.add_child(item)


## 创建玩家列表项
func _create_player_item(player: Dictionary) -> HBoxContainer:
	var container = HBoxContainer.new()
	container.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var name_label = Label.new()
	name_label.text = player.get("name", "Unknown")
	name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	container.add_child(name_label)

	var status_label = Label.new()
	var is_ready = player.get("ready", false)
	status_label.text = "已准备" if is_ready else "未准备"
	status_label.add_theme_color_override("font_color", Color(0.2, 1.0, 0.2) if is_ready else Color(1.0, 0.5, 0.2))
	container.add_child(status_label)

	return container


## 卡组选择
func _on_deck_selected(index: int) -> void:
	if index <= 0:
		_selected_deck = ""
	else:
		_selected_deck = deck_option.get_item_text(index)
	_update_ready_button_state()


## 更新准备按钮状态
func _update_ready_button_state() -> void:
	if _is_ready:
		ready_button.text = "取消准备"
		ready_button.add_theme_color_override("font_color", Color(1.0, 0.5, 0.2))
		deck_option.disabled = true
	else:
		ready_button.text = "准备"
		ready_button.add_theme_color_override("font_color", Color(0.2, 1.0, 0.2))
		deck_option.disabled = false

	# 只有选择了卡组才能准备
	ready_button.disabled = not _is_ready and _selected_deck == ""


## 准备/取消准备按钮点击
func _on_ready_pressed() -> void:
	if _is_ready:
		# 取消准备
		_is_ready = false
		emit_signal("cancel_ready_requested")
	else:
		# 准备
		if _selected_deck != "":
			_is_ready = true
			emit_signal("ready_requested", _selected_deck)

	_update_ready_button_state()


## 返回按钮点击
func _on_back_pressed() -> void:
	emit_signal("back_pressed")


## 被其他玩家触发准备状态改变（外部调用）
func set_ready_state(is_ready: bool) -> void:
	_is_ready = is_ready
	_update_ready_button_state()
