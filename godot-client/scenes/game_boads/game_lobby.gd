## 游戏大厅场景
## 输入房间ID、用户名，选择服务器地址，连接到对战服务器

extends Control

const DEFAULT_SERVER = "ws://localhost:8080/ws"
const USERNAME_KEY = "player_username"

@onready var room_id_input: LineEdit = $VBoxContainer/RoomIdInput
@onready var username_input: LineEdit = $VBoxContainer/UsernameInput
@onready var server_input: LineEdit = $VBoxContainer/ServerInput
@onready var connect_btn: Button = $VBoxContainer/ConnectBtn
@onready var status_label: Label = $VBoxContainer/StatusLabel

var network_manager: Node
var scene_manager: Node

func _ready() -> void:
	# 获取管理器
	network_manager = get_node_or_null("/root/NetworkManager")
	scene_manager = get_node_or_null("/root/SceneManager")

	if not network_manager:
		_print("Warning: NetworkManager not found")
	if not scene_manager:
		_print("Warning: SceneManager not found")

	# 加载保存的用户名
	_load_username()

	# 设置默认值
	server_input.text = DEFAULT_SERVER

	# 连接信号
	connect_btn.pressed.connect(_on_connect_pressed)

	# 连接网络信号
	if network_manager:
		network_manager.connected.connect(_on_connected)
		network_manager.connection_failed.connect(_on_connection_failed)

	status_label.text = ""

func _load_username() -> void:
	var saved = _load_data(USERNAME_KEY, "")
	if saved:
		username_input.text = saved

func _save_username(username: String) -> void:
	_save_data(USERNAME_KEY, username)

func _on_connect_pressed() -> void:
	var room_id = room_id_input.text.strip_edges()
	var username = username_input.text.strip_edges()
	var server = server_input.text.strip_edges()

	if room_id.is_empty():
		status_label.text = "请输入房间ID"
		return

	if username.is_empty():
		status_label.text = "请输入用户名"
		return

	if server.is_empty():
		server = DEFAULT_SERVER

	# 保存用户名
	_save_username(username)

	# 切换到连接中场景
	if scene_manager:
		scene_manager.change_scene("connecting", {
			"room_id": room_id,
			"username": username,
			"server_url": server
		})
	else:
		status_label.text = "场景管理器未找到"

func _on_connected() -> void:
	_print("Connected to server")

func _on_connection_failed(error: String) -> void:
	_print("Connection failed: %s" % error)
	if scene_manager:
		scene_manager.change_scene("lobby", {})

# 简单的本地存储
func _save_data(key: String, value: String) -> void:
	var file = FileAccess.open("user://%s.save" % key, FileAccess.WRITE)
	if file:
		file.store_string(value)
		file.close()

func _load_data(key: String, default_value: String = "") -> String:
	if FileAccess.file_exists("user://%s.save" % key):
		var file = FileAccess.open("user://%s.save" % key, FileAccess.READ)
		if file:
			var value = file.get_as_text()
			file.close()
			return value
	return default_value

func _print(msg: String) -> void:
	print("[GameLobby] %s" % msg)
