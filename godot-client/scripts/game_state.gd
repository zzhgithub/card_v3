extends Node

signal state_changed
signal turn_changed(turn_number: int, current_player: String)
signal phase_changed(phase: String)
signal hp_changed(player_id: String, new_hp: int)
signal hand_changed(cards: Array)
signal field_changed(zone: String, slot: int, card: Dictionary)

# Game state data
var turn_number: int = 0
var current_phase: String = ""
var current_player: String = ""
var my_player_id: String = ""

# Player state (self)
var my_state: Dictionary = {
	"hp": 0,
	"real_point": 0,
	"deck_count": 0,
	"hand": [],
	"front": [],
	"back": [],
	"cost_zone": [],
	"grave": []
}

# Opponent state
var opponent_state: Dictionary = {
	"hp": 0,
	"real_point": 0,
	"deck_count": 0,
	"hand_count": 0,
	"front": [],
	"back": [],
	"cost_zone": [],
	"grave": []
}

# Card cache (instance_id -> card info)
var card_cache: Dictionary = {}

func _ready():
	# Connect to network signals
	var network = get_node_or_null("/root/Network")
	if network:
		network.game_started.connect(_on_game_started)
		network.state_update.connect(_on_state_update)
	else:
		push_warning("[GameState] Network autoload not found")

func update_from_server(state_data: Dictionary):
	if state_data.is_empty():
		return

	var old_turn = turn_number
	var old_phase = current_phase
	var old_player = current_player

	# Update game info
	turn_number = state_data.get("turn_number", turn_number)
	current_phase = state_data.get("current_phase", current_phase)
	current_player = state_data.get("current_player", current_player)

	# Update my state
	if state_data.has("your_state"):
		var new_my_state = state_data["your_state"]
		_update_my_state(new_my_state)

	# Update opponent state
	if state_data.has("opponent_state"):
		var new_opp_state = state_data["opponent_state"]
		_update_opponent_state(new_opp_state)

	# Emit signals for changes
	if old_turn != turn_number or old_player != current_player:
		emit_signal("turn_changed", turn_number, current_player)

	if old_phase != current_phase:
		emit_signal("phase_changed", current_phase)

	emit_signal("state_changed")

func _update_my_state(new_state: Dictionary):
	var old_hp = my_state.get("hp", 0)
	var old_hand_size = my_state.get("hand", []).size()

	my_state = new_state.duplicate(true)

	# Cache card info
	_cache_cards(my_state.get("hand", []))
	_cache_cards(my_state.get("front", []))
	_cache_cards(my_state.get("back", []))

	# Emit signals for important changes
	if old_hp != my_state.get("hp", 0):
		emit_signal("hp_changed", my_player_id, my_state["hp"])

	var new_hand_size = my_state.get("hand", []).size()
	if old_hand_size != new_hand_size:
		emit_signal("hand_changed", my_state.get("hand", []))

func _update_opponent_state(new_state: Dictionary):
	var old_hp = opponent_state.get("hp", 0)

	opponent_state = new_state.duplicate(true)

	# Cache visible cards
	_cache_cards(opponent_state.get("front", []))
	_cache_cards(opponent_state.get("back", []))

	if old_hp != opponent_state.get("hp", 0):
		var opp_id = "Player2" if my_player_id == "Player1" else "Player1"
		emit_signal("hp_changed", opp_id, opponent_state["hp"])

func _cache_cards(cards: Array):
	for card in cards:
		if typeof(card) == TYPE_DICTIONARY and card.has("instance_id"):
			var id = card["instance_id"]
			card_cache[id] = card

func _on_game_started(state_data: Dictionary):
	my_player_id = Network.my_player_id
	update_from_server(state_data)
	print("[GameState] Game started, I am ", my_player_id)

func _on_state_update(state_data: Dictionary):
	update_from_server(state_data)
	print("[GameState] State updated, turn ", turn_number, ", phase: ", current_phase)

# Public API - Getters

func is_my_turn() -> bool:
	return current_player == my_player_id

func get_my_hp() -> int:
	return my_state.get("hp", 0)

func get_opponent_hp() -> int:
	return opponent_state.get("hp", 0)

func get_my_hand() -> Array:
	return my_state.get("hand", [])

func get_opponent_hand_count() -> int:
	return opponent_state.get("hand_count", 0)

func get_my_deck_count() -> int:
	return my_state.get("deck_count", 0)

func get_opponent_deck_count() -> int:
	return opponent_state.get("deck_count", 0)

func get_front_zone(player: String = "self") -> Array:
	if player == "self":
		return my_state.get("front", [])
	else:
		return opponent_state.get("front", [])

func get_back_zone(player: String = "self") -> Array:
	if player == "self":
		return my_state.get("back", [])
	else:
		return opponent_state.get("back", [])

func get_cost_zone(player: String = "self") -> Array:
	if player == "self":
		return my_state.get("cost_zone", [])
	else:
		return opponent_state.get("cost_zone", [])

func get_grave(player: String = "self") -> Array:
	if player == "self":
		return my_state.get("grave", [])
	else:
		return opponent_state.get("grave", [])

func get_card_info(instance_id: int) -> Dictionary:
	return card_cache.get(instance_id, {})

func can_play_card(card: Dictionary, zone_type: String, slot: int) -> bool:
	# Basic validation - can be extended with more rules
	if not card.has("instance_id"):
		return false

	var zone = get_front_zone() if zone_type == "front" else get_back_zone()
	if slot < 0 or slot >= zone.size():
		return false

	# Check if slot is empty
	if zone[slot] != null:
		return false

	return is_my_turn()

func can_attack(attacker_id: int) -> bool:
	if not is_my_turn():
		return false

	# Check if attacker exists on field
	var front = get_front_zone()
	for card in front:
		if typeof(card) == TYPE_DICTIONARY and card.get("instance_id") == attacker_id:
			return true

	return false

func get_available_actions() -> Array[String]:
	var actions: Array[String] = ["pass"]

	if not is_my_turn():
		return actions

	# Check if can play cards
	var hand = get_my_hand()
	if hand.size() > 0:
		actions.append("play_card")

	# Check if can attack
	var front = get_front_zone()
	for card in front:
		if card != null:
			actions.append("declare_attack")
			break

	actions.append("surrender")
	return actions

func get_empty_slots(zone_type: String) -> Array[int]:
	var zone = get_front_zone() if zone_type == "front" else get_back_zone()
	var empty_slots: Array[int] = []

	for i in range(zone.size()):
		if zone[i] == null:
			empty_slots.append(i)

	return empty_slots

func format_phase(phase: String) -> String:
	match phase:
		"TurnStart": return "回合开始"
		"Draw": return "抽卡阶段"
		"Main": return "主要阶段"
		"Battle": return "战斗阶段"
		"End": return "回合结束"
		_: return phase

func format_player_name(player_id: String) -> String:
	if player_id == my_player_id:
		return "你"
	return "对手"

func reset():
	turn_number = 0
	current_phase = ""
	current_player = ""
	my_player_id = ""
	my_state = {
		"hp": 0, "real_point": 0, "deck_count": 0,
		"hand": [], "front": [], "back": [],
		"cost_zone": [], "grave": []
	}
	opponent_state = {
		"hp": 0, "real_point": 0, "deck_count": 0, "hand_count": 0,
		"front": [], "back": [],
		"cost_zone": [], "grave": []
	}
	card_cache.clear()
	emit_signal("state_changed")
