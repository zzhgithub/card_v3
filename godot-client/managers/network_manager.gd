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
signal player_ready(player_name: String, deck_id: String)
signal player_unready(player_name: String)
signal room_state_updated(players: Array, all_ready: bool)
signal game_starting
signal game_started(game_data: Dictionary)
signal state_update(state: Dictionary)
signal action_request(available_actions: Array, timeout_secs: int)
signal recovery_request(count: int, options: Array)

const DEFAULT_SERVER_URL = "ws://localhost:8080/ws"
const CONNECTION_TIMEOUT = 30.0
const ROOM_STATE_POLL_INTERVAL = 2.0  ## 轮询房间状态间隔（秒）

var websocket: WebSocketPeer
var server_url: String = DEFAULT_SERVER_URL
var current_room_id: String = ""
var current_username: String = ""
var ws_connected: bool = false
var connection_timer: Timer
var room_state_timer: Timer  ## 房间状态轮询定时器

func _ready() -> void:
	websocket = WebSocketPeer.new()
	_create_connection_timer()
	_create_room_state_timer()

func _create_connection_timer() -> void:
	connection_timer = Timer.new()
	connection_timer.one_shot = true
	connection_timer.wait_time = CONNECTION_TIMEOUT
	connection_timer.timeout.connect(_on_connection_timeout)
	add_child(connection_timer)

func _create_room_state_timer() -> void:
	room_state_timer = Timer.new()
	room_state_timer.wait_time = ROOM_STATE_POLL_INTERVAL
	room_state_timer.timeout.connect(_on_room_state_poll)
	add_child(room_state_timer)

func _process(_delta: float) -> void:
	if not ws_connected:
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
			ws_connected = false
			room_state_timer.stop()
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
			ws_connected = true
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
	# 先发送离开房间消息
	if ws_connected and current_room_id != "":
		leave_room()

	if ws_connected:
		websocket.close(1000, "Client disconnect")
		ws_connected = false
		room_state_timer.stop()
		_print("Disconnected from server")

## 发送消息 (直接展开 data 字段以匹配服务器格式)
func send_message(type: String, data: Dictionary = {}) -> bool:
	if not ws_connected:
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
			_print("Joined room, player_id: %s" % data.get("player_id", ""))
			player_joined.emit(current_username)
			room_joined.emit(current_room_id, [])
			# 开始轮询房间状态
			room_state_timer.start()

		"left":
			_print("Left room")
			room_state_timer.stop()
			current_room_id = ""

		"opponent_joined":
			player_joined.emit(data.get("player_name", ""))

		"player_disconnected":
			player_left.emit(data.get("player_name", ""))

		"player_ready":
			player_ready.emit(data.get("player_name", ""), data.get("deck_id", ""))

		"player_unready":
			player_unready.emit(data.get("player_name", ""))

		"room_state":
			var players = data.get("players", [])
			var all_ready = data.get("all_ready", false)
			_print("Room state response received: %d players, all_ready=%s" % [players.size(), all_ready])
			for p in players:
				_print("  - Player: %s, ready=%s, deck=%s" % [p.get("name", "?"), p.get("is_ready", false), p.get("deck_id", "none")])
			room_state_updated.emit(players, all_ready)

		"waiting_for_deck":
			_print("Waiting for deck submission")

		"game_starting":
			_print("Game is starting...")
			game_starting.emit()

		"game_started":
			_print("Game started!")
			room_state_timer.stop()
			# GameStarted 是纯通知，不再携带 state（由后续的 state_update 推送）
			game_started.emit({})

		"state_update":
			_print("State update received")
			# 服务器发送的是 {"state": {...}}，提取 state 字段
			var state = data.get("state", {})
			state_update.emit(state)

		"action_request":
			var actions = data.get("available_actions", [])
			var timeout = data.get("timeout_secs", 60)
			action_request.emit(actions, timeout)

		"recovery_request":
			var count = data.get("count", 0)
			var options = data.get("options", [])
			recovery_request.emit(count, options)

		"game_over":
			_print("Game over, winner: %s" % data.get("winner", "none"))

		"error":
			_print("Server error: %s" % data.get("message", "Unknown error"))

## 连接超时处理
func _on_connection_timeout() -> void:
	if not ws_connected:
		websocket.close(1000, "Connection timeout")
		connection_failed.emit("Connection timeout after %.0f seconds" % CONNECTION_TIMEOUT)

## 房间状态轮询
func _on_room_state_poll() -> void:
	_print("Room state poll triggered")
	if ws_connected and current_room_id != "":
		query_room_state()
	else:
		_print("Skipping room state poll - not connected or not in room (connected=%s, room=%s)" % [ws_connected, current_room_id])

## 加入房间
func join_room(room_id: String, username: String) -> bool:
	current_room_id = room_id
	current_username = username
	return send_message("join_room", {
		"room_id": room_id,
		"player_name": username
	})

## 准备（提交卡组）
func submit_deck(deck_id: String, cards: Array) -> bool:
	_print("Submitting deck '%s' with %d cards" % [deck_id, cards.size()])
	return send_message("submit_deck", {
		"deck_id": deck_id,
		"cards": cards
	})

## 取消准备
func set_unready() -> bool:
	_print("Setting unready")
	return send_message("unready", {})

## 离开房间
func leave_room() -> bool:
	_print("Leaving room")
	room_state_timer.stop()
	return send_message("leave_room", {})

## 查询房间状态
func query_room_state() -> bool:
	_print("Querying room state for room: %s" % current_room_id)
	return send_message("query_room_state", {})

## 发送操作: Pass
func send_action_pass() -> bool:
	return send_message("action", {"action": {"action_type": "pass"}})

## 发送操作: Surrender
func send_action_surrender() -> bool:
	return send_message("action", {"action": {"action_type": "surrender"}})

## 发送操作: PlayCard
func send_action_play_card(instance_id: int, zone_type: String, slot: int) -> bool:
	return send_message("action", {
		"action": {
			"action_type": "play_card",
			"instance_id": instance_id,
			"target_zone": {
				"zone_type": zone_type,
				"slot": slot
			}
		}
	})

## 发送操作: DeclareAttack
func send_action_declare_attack(attacker_id: int, target_type: String, slot_index: int = -1) -> bool:
	var action = {
		"action_type": "declare_attack",
		"attacker_id": attacker_id,
		"target": {
			"target_type": target_type
		}
	}
	if target_type == "slot" and slot_index >= 0:
		action["target"]["slot_index"] = slot_index
	return send_message("action", {"action": action})

## 发送回收选择
func send_recovery_selection(cards: Array[int]) -> bool:
	return send_message("recovery", {"cards": cards})

## 打印日志
func _print(msg: String) -> void:
	print("[NetworkManager] %s" % msg)
