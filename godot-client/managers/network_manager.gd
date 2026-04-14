## 网络管理器
## 处理 WebSocket 连接、消息收发和场景切换

extends Node

signal connected
signal disconnected
signal connection_failed(error: String)
signal message_received(type: String, data: Dictionary)
signal room_joined(room_id: String, players: Array)
signal player_joined(player_name: String)
signal player_left(player_name: String)
signal player_ready_changed(player_name: String, is_ready: bool)
signal game_started(game_data: Dictionary)

const DEFAULT_SERVER_URL = "ws://localhost:8080/ws"
const CONNECTION_TIMEOUT = 30.0

var websocket: WebSocketPeer
var server_url: String = DEFAULT_SERVER_URL
var current_room_id: String = ""
var current_username: String = ""
var is_connected: bool = false
var connection_timer: Timer

func _ready() -> void:
	websocket = WebSocketPeer.new()
	_create_connection_timer()

func _create_connection_timer() -> void:
	connection_timer = Timer.new()
	connection_timer.one_shot = true
	connection_timer.wait_time = CONNECTION_TIMEOUT
	connection_timer.timeout.connect(_on_connection_timeout)
	add_child(connection_timer)

func _process(_delta: float) -> void:
	if not is_connected:
		return

	websocket.poll()
	var state = websocket.get_ready_state()

	match state:
		WebSocketPeer.STATE_OPEN:
			while websocket.get_available_packet_count() > 0:
				var packet = websocket.get_packet()
				var message = packet.get_string_from_utf8()
				_handle_message(message)

		WebSocketPeer.STATE_CLOSED:
			var code = websocket.get_close_code()
			var reason = websocket.get_close_reason()
			_print("WebSocket closed: %d - %s" % [code, reason])
			is_connected = false
			disconnected.emit()

## 连接到服务器
func connect_to_server(url: String = "") -> bool:
	if url:
		server_url = url

	_print("Connecting to: %s" % server_url)

	var err = websocket.connect_to_url(server_url)
	if err != OK:
		connection_failed.emit("Failed to initiate connection: %d" % err)
		return false

	connection_timer.start()

	# 等待连接建立
	var attempts = 0
	while attempts < 100:
		websocket.poll()
		var state = websocket.get_ready_state()

		if state == WebSocketPeer.STATE_OPEN:
			is_connected = true
			connection_timer.stop()
			_print("Connected to server")
			connected.emit()
			return true
		elif state == WebSocketPeer.STATE_CLOSED:
			connection_failed.emit("Connection closed immediately")
			return false

		await get_tree().create_timer(0.05).timeout
		attempts += 1

	return false

## 断开连接
func disconnect_from_server() -> void:
	if is_connected:
		websocket.close(1000, "Client disconnect")
		is_connected = false
		_print("Disconnected from server")

## 发送消息 (直接展开 data 字段以匹配服务器格式)
func send_message(type: String, data: Dictionary = {}) -> bool:
	if not is_connected:
		_print("Cannot send message: not connected")
		return false

	# 服务器期望直接字段，不是嵌套在 data 中
	var message = {"type": type}
	for key in data:
		message[key] = data[key]

	var json = JSON.stringify(message)
	_print("Sending JSON: %s" % json)
	var err = websocket.send_text(json)

	if err == OK:
		_print("Sent: %s" % type)
		return true
	else:
		_print("Failed to send message: %d" % err)
		return false

## 处理接收到的消息 (服务器直接返回字段，不是嵌套在 data 中)
func _handle_message(message: String) -> void:
	_print("Received: %s" % message)

	var json = JSON.new()
	var err = json.parse(message)
	if err != OK:
		_print("Failed to parse message: %s" % message)
		return

	var parsed = json.get_data()
	if not parsed is Dictionary:
		return

	var msg_type = parsed.get("type", "")
	# 服务器返回的消息直接包含字段，不需要从 data 中提取
	var data = parsed.duplicate()
	data.erase("type")

	message_received.emit(msg_type, data)
	_route_message(msg_type, data)

## 路由消息到具体处理函数
func _route_message(type: String, data: Dictionary) -> void:
	match type:
		"joined":
			# 服务器返回 joined 消息，player_id 是字符串
			_print("Joined room, player_id: %s" % data.get("player_id", ""))
			# joined 消息没有 room_id，使用 current_room_id
			room_joined.emit(current_room_id, [])

		"opponent_joined":
			player_joined.emit(data.get("player_name", ""))

		"player_disconnected":
			player_left.emit(data.get("player_name", ""))

		"waiting_for_deck":
			_print("Waiting for deck submission")

		"game_started":
			game_started.emit(data)

		"state_update":
			_print("State update received")

		"action_request":
			_print("Action requested")

		"recovery_request":
			_print("Recovery requested")

		"game_over":
			_print("Game over, winner: %s" % data.get("winner", "none"))

		"error":
			_print("Server error: %s" % data.get("message", "Unknown error"))

## 连接超时处理
func _on_connection_timeout() -> void:
	if not is_connected:
		websocket.close(1000, "Connection timeout")
		connection_failed.emit("Connection timeout after %.0f seconds" % CONNECTION_TIMEOUT)

## 房间相关操作
func join_room(room_id: String, username: String) -> bool:
	current_room_id = room_id
	current_username = username
	return send_message("join_room", {
		"room_id": room_id,
		"player_name": username
	})

func set_ready(is_ready: bool, deck_id: String = "") -> bool:
	return send_message("ready", {
		"is_ready": is_ready,
		"deck_id": deck_id
	})

func leave_room() -> bool:
	return send_message("leave_room", {})

## 打印日志
func _print(msg: String) -> void:
	print("[NetworkManager] %s" % msg)
