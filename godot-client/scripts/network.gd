extends Node

signal connected
signal disconnected
signal message_received(message: Dictionary)
signal error_occurred(error_message: String)
signal player_joined(player_name: String)
signal opponent_joined(player_name: String)
signal game_started(game_state: Dictionary)
signal state_update(game_state: Dictionary)
signal action_requested(available_actions: Array, timeout_secs: int)
signal recovery_requested(count: int, options: Array)
signal player_disconnected(player_name: String)
signal game_over(winner: String, reason: String)

@export var server_url: String = "ws://127.0.0.1:7878"
@export var auto_reconnect: bool = true
@export var reconnect_delay: float = 3.0

var socket: WebSocketPeer = WebSocketPeer.new()
var ws_connected: bool = false
var current_room_id: String = ""
var current_player_name: String = ""
var my_player_id: String = ""

func _ready():
	set_process(false)

func _process(_delta):
	socket.poll()
	var state = socket.get_ready_state()

	match state:
		WebSocketPeer.STATE_OPEN:
			if not ws_connected:
				ws_connected = true
				emit_signal("connected")
				print("[Network] Connected to server")

			while socket.get_available_packets() > 0:
				var packet = socket.get_packet()
				var message = packet.get_string_from_utf8()
				_handle_message(message)

		WebSocketPeer.STATE_CLOSED:
			if ws_connected:
				ws_connected = false
				emit_signal("disconnected")
				print("[Network] Disconnected from server")
				set_process(false)

				if auto_reconnect:
					await get_tree().create_timer(reconnect_delay).timeout
					connect_to_server()

func connect_to_server(url: String = "") -> bool:
	if url != "":
		server_url = url

	print("[Network] Connecting to ", server_url)
	var err = socket.connect_to_url(server_url)
	if err != OK:
		emit_signal("error_occurred", "Failed to connect: " + str(err))
		return false

	set_process(true)
	return true

func disconnect_from_server():
	auto_reconnect = false
	socket.close()
	set_process(false)
	ws_connected = false

func _handle_message(message: String):
	print("[Network] Received: ", message)

	var json = JSON.new()
	var error = json.parse(message)
	if error != OK:
		print("[Network] Failed to parse message: ", message)
		return

	var data = json.get_data()
	if typeof(data) != TYPE_DICTIONARY:
		print("[Network] Invalid message format")
		return

	emit_signal("message_received", data)
	_dispatch_message(data)

func _dispatch_message(data: Dictionary):
	var msg_type = data.get("type", "")

	match msg_type:
		"joined":
			my_player_id = data.get("player_id", "")
			emit_signal("player_joined", current_player_name)
			print("[Network] Joined as ", my_player_id)

		"opponent_joined":
			var opponent_name = data.get("player_name", "Unknown")
			emit_signal("opponent_joined", opponent_name)
			print("[Network] Opponent joined: ", opponent_name)

		"waiting_for_deck":
			print("[Network] Waiting for deck submission")

		"game_started":
			var state = data.get("state", {})
			emit_signal("game_started", state)
			print("[Network] Game started!")

		"state_update":
			var state = data.get("state", {})
			if not state.is_empty():
				emit_signal("state_update", state)

		"action_request":
			var actions = data.get("available_actions", [])
			var timeout = data.get("timeout_secs", 60)
			emit_signal("action_requested", actions, timeout)
			print("[Network] Action requested, timeout: ", timeout)

		"recovery_request":
			var count = data.get("count", 0)
			var options = data.get("options", [])
			emit_signal("recovery_requested", count, options)

		"player_disconnected":
			var player_name = data.get("player_name", "Unknown")
			emit_signal("player_disconnected", player_name)
			print("[Network] Player disconnected: ", player_name)

		"game_over":
			var winner = data.get("winner", "")
			var reason = data.get("reason", "")
			emit_signal("game_over", winner, reason)
			print("[Network] Game over! Winner: ", winner, " Reason: ", reason)

		"error":
			var error_msg = data.get("message", "Unknown error")
			emit_signal("error_occurred", error_msg)
			print("[Network] Error: ", error_msg)

		_:
			print("[Network] Unknown message type: ", msg_type)

func _send_message(data: Dictionary) -> bool:
	if not ws_connected:
		print("[Network] Not connected, cannot send message")
		return false

	var json_string = JSON.stringify(data)
	var err = socket.send_text(json_string)
	if err != OK:
		emit_signal("error_occurred", "Failed to send message: " + str(err))
		return false

	print("[Network] Sent: ", json_string)
	return true

# Public API

func join_room(room_id: String, player_name: String) -> bool:
	current_room_id = room_id
	current_player_name = player_name
	return _send_message({
		"type": "join_room",
		"room_id": room_id,
		"player_name": player_name
	})

func submit_deck(cards: Array[String]) -> bool:
	return _send_message({
		"type": "submit_deck",
		"cards": cards
	})

func send_action_pass() -> bool:
	return _send_message({
		"type": "action",
		"action": {
			"action_type": "pass"
		}
	})

func send_action_surrender() -> bool:
	return _send_message({
		"type": "action",
		"action": {
			"action_type": "surrender"
		}
	})

func send_action_play_card(instance_id: int, zone_type: String, slot: int) -> bool:
	return _send_message({
		"type": "action",
		"action": {
			"action_type": "play_card",
			"instance_id": instance_id,
			"target_zone": {
				"zone_type": zone_type,
				"slot": slot
			}
		}
	})

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

	return _send_message({
		"type": "action",
		"action": action
	})

func send_recovery_selection(cards: Array[int]) -> bool:
	return _send_message({
		"type": "recovery",
		"cards": cards
	})
