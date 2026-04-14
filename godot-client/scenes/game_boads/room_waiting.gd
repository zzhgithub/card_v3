## 房间等待场景
## 显示房间ID、玩家列表、准备按钮、卡组选择

extends Control

@onready var room_id_label: Label = $VBoxContainer/TopBar/RoomIdLabel
@onready var player_list: VBoxContainer = $VBoxContainer/MiddleSection/PlayerListPanel/ScrollContainer/PlayerList
@onready var ready_btn: Button = $VBoxContainer/TopBar/ReadyBtn
@onready var leave_btn: Button = $VBoxContainer/TopBar/LeaveBtn
@onready var deck_dropdown: OptionButton = $VBoxContainer/MiddleSection/RightPanel/DeckDropdown
@onready var deck_preview: TextureRect = $VBoxContainer/MiddleSection/RightPanel/DeckPreview

var room_id: String = ""
var players: Array = []
var is_ready: bool = false

var network_manager: Node
var scene_manager: Node

func _ready() -> void:
	# 获取管理器
	network_manager = get_node_or_null("/root/NetworkManager")
	scene_manager = get_node_or_null("/root/SceneManager")

	# 连接信号
	ready_btn.pressed.connect(_on_ready_pressed)
	leave_btn.pressed.connect(_on_leave_pressed)
	deck_dropdown.item_selected.connect(_on_deck_selected)

	if network_manager:
		network_manager.player_joined.connect(_on_player_joined)
		network_manager.player_left.connect(_on_player_left)
		network_manager.player_ready_changed.connect(_on_player_ready_changed)
		network_manager.game_started.connect(_on_game_started)
		network_manager.disconnected.connect(_on_disconnected)

	# 初始化UI
	_update_room_id()
	_refresh_player_list()
	_load_deck_list()

func _update_room_id() -> void:
	room_id_label.text = "房间ID: %s" % room_id

func _refresh_player_list() -> void:
	# 清空列表
	for child in player_list.get_children():
		child.queue_free()

	# 添加玩家
	for player in players:
		var hbox = HBoxContainer.new()
		hbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL

		var name_label = Label.new()
		name_label.text = player.get("name", "未知")
		name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL

		var status_label = Label.new()
		status_label.text = "已准备" if player.get("is_ready", false) else "未准备"
		status_label.add_theme_color_override("font_color", Color(0, 1, 0) if player.get("is_ready", false) else Color(1, 1, 1))

		hbox.add_child(name_label)
		hbox.add_child(status_label)
		player_list.add_child(hbox)

func _load_deck_list() -> void:
	# 加载卡组列表（从本地存储或配置）
	deck_dropdown.clear()
	deck_dropdown.add_item("选择卡组")
	deck_dropdown.add_item("默认卡组")
	deck_dropdown.add_item("自定义卡组1")
	deck_dropdown.add_item("自定义卡组2")

func _on_ready_pressed() -> void:
	is_ready = not is_ready
	ready_btn.text = "取消准备" if is_ready else "准备"

	var deck_id = deck_dropdown.get_item_text(deck_dropdown.selected) if deck_dropdown.selected > 0 else ""

	if network_manager:
		network_manager.set_ready(is_ready, deck_id)

func _on_leave_pressed() -> void:
	if network_manager:
		network_manager.leave_room()
		network_manager.disconnect_from_server()

	if scene_manager:
		scene_manager.change_scene("lobby", {})

func _on_deck_selected(index: int) -> void:
	if index > 0:
		# 加载卡组预览
		_print("Selected deck: %s" % deck_dropdown.get_item_text(index))

func _on_player_joined(player_name: String) -> void:
	_print("Player joined: %s" % player_name)
	players.append({"name": player_name, "is_ready": false})
	_refresh_player_list()

func _on_player_left(player_name: String) -> void:
	_print("Player left: %s" % player_name)
	players = players.filter(func(p): return p.get("name", "") != player_name)
	_refresh_player_list()

func _on_player_ready_changed(player_name: String, ready: bool) -> void:
	_print("Player %s ready: %s" % [player_name, ready])
	for player in players:
		if player.get("name", "") == player_name:
			player["is_ready"] = ready
			break
	_refresh_player_list()

func _on_game_started(game_data: Dictionary) -> void:
	_print("Game started!")
	if scene_manager:
		scene_manager.change_scene("game_board", game_data)

func _on_disconnected() -> void:
	_print("Disconnected from server")
	if scene_manager:
		scene_manager.change_scene("lobby", {})

func _print(msg: String) -> void:
	print("[RoomWaiting] %s" % msg)
