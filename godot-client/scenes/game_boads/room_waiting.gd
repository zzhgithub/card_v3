## 房间等待场景
## 显示房间ID、玩家列表、准备按钮、卡组选择

extends Control

@onready var room_id_label: Label = $VBoxContainer/TopBar/RoomIdLabel
@onready var player_list: VBoxContainer = $VBoxContainer/MiddleSection/PlayerListPanel/VBoxContainer/ScrollContainer/PlayerList
@onready var ready_btn: Button = $VBoxContainer/TopBar/ReadyBtn
@onready var leave_btn: Button = $VBoxContainer/TopBar/LeaveBtn
@onready var deck_dropdown: OptionButton = $VBoxContainer/MiddleSection/RightPanel/DeckDropdown
@onready var deck_preview: TextureRect = $VBoxContainer/MiddleSection/RightPanel/DeckPreview

var room_id: String = ""
var players: Array = []
var is_ready: bool = false
var current_player_name: String = ""
var deck_cards: Dictionary = {}  ## 缓存卡组名称到卡片列表的映射

var network_manager: Node
var scene_manager: Node

func _ready() -> void:
	# 获取管理器
	network_manager = get_node_or_null("/root/NetworkManager")
	scene_manager = get_node_or_null("/root/SceneManager")

	# 获取当前用户名
	if network_manager:
		current_player_name = network_manager.current_username

	# 连接信号
	ready_btn.pressed.connect(_on_ready_pressed)
	leave_btn.pressed.connect(_on_leave_pressed)
	deck_dropdown.item_selected.connect(_on_deck_selected)

	if network_manager:
		network_manager.player_joined.connect(_on_player_joined)
		network_manager.player_left.connect(_on_player_left)
		network_manager.player_ready.connect(_on_player_ready)
		network_manager.player_unready.connect(_on_player_unready)
		network_manager.room_state_updated.connect(_on_room_state_updated)
		network_manager.game_starting.connect(_on_game_starting)
		network_manager.game_started.connect(_on_game_started)
		network_manager.disconnected.connect(_on_disconnected)

	# 初始化UI
	_update_room_id()
	_load_deck_list()
	_refresh_player_list()

func _update_room_id() -> void:
	room_id_label.text = "房间ID: %s" % room_id

func _refresh_player_list() -> void:
	if player_list == null:
		return
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

		var is_player_ready = player.get("is_ready", false)
		var deck_id = player.get("deck_id", "")
		var status_text = "已准备" if is_player_ready else "未准备"
		if is_player_ready and deck_id != "":
			status_text += " (%s)" % deck_id

		var status_label = Label.new()
		status_label.text = status_text
		status_label.add_theme_color_override("font_color", Color(0, 1, 0) if is_player_ready else Color(1, 1, 1))

		hbox.add_child(name_label)
		hbox.add_child(status_label)
		player_list.add_child(hbox)

func _load_deck_list() -> void:
	if deck_dropdown == null:
		return
	# 加载卡组列表（从 desks 文件夹）
	deck_dropdown.clear()
	deck_dropdown.add_item("选择卡组")

	var desks_dir = DirAccess.open("res://desks")
	if desks_dir:
		desks_dir.list_dir_begin()
		var file_name = desks_dir.get_next()
		while file_name != "":
			if not desks_dir.current_is_dir() and file_name.ends_with(".json"):
				# 去掉 .json 后缀显示
				var deck_name = file_name.get_basename()
				deck_dropdown.add_item(deck_name)
				# 加载卡组卡片
				_load_deck_cards(deck_name)
			file_name = desks_dir.get_next()
		desks_dir.list_dir_end()

	# 如果没有找到任何卡组，添加默认选项
	if deck_dropdown.item_count <= 1:
		deck_dropdown.add_item("默认卡组")

func _load_deck_cards(deck_name: String) -> void:
	var file_path = "res://desks/%s.json" % deck_name
	if FileAccess.file_exists(file_path):
		var file = FileAccess.open(file_path, FileAccess.READ)
		if file:
			var content = file.get_as_text()
			file.close()
			var json = JSON.new()
			var err = json.parse(content)
			if err == OK:
				var data = json.get_data()
				if data is Dictionary and data.has("cards"):
					deck_cards[deck_name] = data["cards"]
					_print("Loaded deck '%s' with %d cards" % [deck_name, data["cards"].size()])

func _on_ready_pressed() -> void:
	if is_ready:
		# 取消准备
		is_ready = false
		ready_btn.text = "准备"
		if network_manager:
			network_manager.set_unready()
	else:
		# 准备
		var deck_id = deck_dropdown.get_item_text(deck_dropdown.selected) if deck_dropdown.selected > 0 else ""
		if deck_id == "" or deck_id == "选择卡组":
			_print("Please select a deck first")
			return

		var cards = deck_cards.get(deck_id, [])
		if cards.is_empty():
			_print("Deck '%s' is empty or not loaded" % deck_id)
			return

		is_ready = true
		ready_btn.text = "取消准备"
		if network_manager:
			network_manager.submit_deck(deck_id, cards)

func _on_leave_pressed() -> void:
	if network_manager:
		network_manager.leave_room()
		network_manager.disconnect_from_server()

	if scene_manager:
		scene_manager.change_scene("lobby", {})

func _on_deck_selected(index: int) -> void:
	if index > 0:
		var deck_name = deck_dropdown.get_item_text(index)
		_print("Selected deck: %s" % deck_name)

func _on_player_joined(player_name: String) -> void:
	_print("Player joined signal: %s" % player_name)
	# 触发轮询获取最新房间状态
	if network_manager:
		_print("Triggering room state query after player joined")
		network_manager.query_room_state()

func _on_player_left(player_name: String) -> void:
	_print("Player left signal: %s" % player_name)
	# 触发轮询获取最新房间状态
	if network_manager:
		_print("Triggering room state query after player left")
		network_manager.query_room_state()

func _on_player_ready(player_name: String, deck_id: String) -> void:
	_print("Player ready signal: %s with deck %s" % [player_name, deck_id])
	# 触发轮询获取最新房间状态
	if network_manager:
		_print("Triggering room state query after player ready")
		network_manager.query_room_state()

func _on_player_unready(player_name: String) -> void:
	_print("Player unready signal: %s" % player_name)
	# 触发轮询获取最新房间状态
	if network_manager:
		_print("Triggering room state query after player unready")
		network_manager.query_room_state()

func _on_room_state_updated(players_info: Array, all_ready: bool) -> void:
	_print("Room state updated, all_ready: %s" % all_ready)
	_print("Players info from server: %s" % str(players_info))

	# 完全替换玩家列表（基于服务器返回的数据）
	players.clear()
	for player_info in players_info:
		if player_info is Dictionary:
			var player_name = player_info.get("name", "")
			var is_player_ready = player_info.get("is_ready", false)
			var deck_id = player_info.get("deck_id", "")
			players.append({
				"name": player_name,
				"is_ready": is_player_ready,
				"deck_id": deck_id if deck_id else ""
			})
			_print("Added player: %s (ready=%s, deck=%s)" % [player_name, is_player_ready, deck_id])

	_print("Total players in room: %d" % players.size())
	_refresh_player_list()

func _on_game_starting() -> void:
	_print("Game is starting...")

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
