## 连接中场景
## 显示连接进度，30秒超时返回

extends Control

const TIMEOUT_SECONDS = 30.0

@onready var status_label: Label = $VBoxContainer/StatusLabel
@onready var timer_label: Label = $VBoxContainer/TimerLabel
@onready var cancel_btn: Button = $VBoxContainer/CancelBtn

var room_id: String = ""
var username: String = ""
var server_url: String = ""
var elapsed_time: float = 0.0
var is_connecting: bool = false

var network_manager: Node
var scene_manager: Node

func _ready() -> void:
	# 获取管理器
	network_manager = get_node_or_null("/root/NetworkManager")
	scene_manager = get_node_or_null("/root/SceneManager")

	# 连接信号
	cancel_btn.pressed.connect(_on_cancel_pressed)

	if network_manager:
		network_manager.connected.connect(_on_connected)
		network_manager.connection_failed.connect(_on_connection_failed)
		network_manager.room_joined.connect(_on_room_joined)

	# 开始连接
	_start_connection()

func _start_connection() -> void:
	is_connecting = true
	elapsed_time = 0.0
	status_label.text = "正在连接服务器..."

	if network_manager:
		var success = network_manager.connect_to_server(server_url)
		if not success:
			status_label.text = "连接失败"
			_return_to_lobby(1.0)
	else:
		status_label.text = "网络管理器未找到"
		_return_to_lobby(1.0)

func _process(delta: float) -> void:
	if is_connecting:
		elapsed_time += delta
		var remaining = max(0, TIMEOUT_SECONDS - elapsed_time)
		timer_label.text = "%.0f秒" % remaining

		if elapsed_time >= TIMEOUT_SECONDS:
			_on_connection_failed("连接超时")

func _on_connected() -> void:
	status_label.text = "已连接，正在加入房间..."
	if network_manager:
		network_manager.join_room(room_id, username)

func _on_connection_failed(error: String) -> void:
	is_connecting = false
	status_label.text = "连接失败: %s" % error
	_return_to_lobby(2.0)

func _on_room_joined(room_id: String, players: Array) -> void:
	is_connecting = false
	status_label.text = "已加入房间 %s" % room_id

	# 切换到房间等待场景
	if scene_manager:
		scene_manager.change_scene("room_waiting", {
			"room_id": room_id,
			"players": players
		})

func _on_cancel_pressed() -> void:
	is_connecting = false
	if network_manager:
		network_manager.disconnect_from_server()

	_return_to_lobby(0.0)

func _return_to_lobby(delay: float) -> void:
	if delay > 0:
		await get_tree().create_timer(delay).timeout

	if scene_manager:
		scene_manager.change_scene("lobby", {})

func _print(msg: String) -> void:
	print("[Connecting] %s" % msg)
