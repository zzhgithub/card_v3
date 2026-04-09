extends Control

@onready var status_label: Label = $VBoxContainer/StatusLabel
@onready var room_input: LineEdit = $VBoxContainer/RoomInput
@onready var name_input: LineEdit = $VBoxContainer/NameInput
@onready var connect_button: Button = $VBoxContainer/ConnectButton
@onready var log_text: TextEdit = $VBoxContainer/LogText

var is_in_room: bool = false

func _ready():
	# Connect network signals
	Network.connected.connect(_on_connected)
	Network.disconnected.connect(_on_disconnected)
	Network.error_occurred.connect(_on_error)
	Network.player_joined.connect(_on_player_joined)
	Network.opponent_joined.connect(_on_opponent_joined)
	Network.game_started.connect(_on_game_started)
	Network.state_update.connect(_on_state_update)
	Network.action_requested.connect(_on_action_requested)
	Network.game_over.connect(_on_game_over)

	# Default values
	room_input.text = "room1"
	name_input.text = "Player" + str(randi() % 1000)

	_log("[Main] Ready. Click 'Connect' to start.")

func _on_connect_button_pressed():
	if not is_in_room:
		var url = Network.server_url
		_log("[Main] Connecting to " + url)
		connect_button.disabled = true
		Network.connect_to_server()
	else:
		_log("[Main] Disconnecting...")
		Network.disconnect_from_server()

func _on_connected():
	_log("[Main] Connected! Joining room...")
	var room_id = room_input.text
	var player_name = name_input.text
	Network.join_room(room_id, player_name)

func _on_disconnected():
	_log("[Main] Disconnected from server")
	status_label.text = "Status: Disconnected"
	connect_button.text = "Connect"
	connect_button.disabled = false
	is_in_room = false

func _on_error(error_message: String):
	_log("[Main] Error: " + error_message)
	connect_button.disabled = false

func _on_player_joined(player_name: String):
	_log("[Main] Joined room as: " + player_name)
	status_label.text = "Status: In Room (Waiting for opponent)"
	is_in_room = true
	connect_button.text = "Disconnect"

func _on_opponent_joined(opponent_name: String):
	_log("[Main] Opponent joined: " + opponent_name)
	status_label.text = "Status: Opponent Joined"

func _on_game_started(game_state: Dictionary):
	_log("[Main] Game started!")
	_log("[Main] Turn: " + str(game_state.get("turn_number")))
	_log("[Main] Phase: " + game_state.get("current_phase", ""))

	GameState.update_from_server(game_state)

	status_label.text = "Status: Game Started"

	# Submit deck if we haven't (demo deck)
	if GameState.get_my_deck_count() == 0:
		var demo_deck: Array[String] = ["S000-C-001", "S000-C-002", "S000-S-001"]
		Network.submit_deck(demo_deck)
		_log("[Main] Submitted demo deck")

func _on_state_update(game_state: Dictionary):
	_log("[Main] State update received")
	GameState.update_from_server(game_state)

func _on_action_requested(available_actions: Array, timeout_secs: int):
	_log("[Main] Action requested! Timeout: " + str(timeout_secs) + "s")
	_log("[Main] Available actions: " + str(available_actions))

	# Auto pass for demo
	await get_tree().create_timer(1.0).timeout
	Network.send_action_pass()
	_log("[Main] Sent Pass action")

func _on_game_over(winner: String, reason: String):
	_log("[Main] Game Over!")
	_log("[Main] Winner: " + winner)
	_log("[Main] Reason: " + reason)
	status_label.text = "Status: Game Over"

func _log(message: String):
	print(message)
	log_text.text += message + "\n"
	log_text.scroll_vertical = INF
