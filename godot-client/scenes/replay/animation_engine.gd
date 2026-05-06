extends RefCounted
const Types = preload("res://scenes/replay/replay_types.gd")

const _RT = preload("res://scenes/replay/replay_types.gd")


var _board  # ReplayBoard


func _init(board):  # ReplayBoard
	_board = board


## 处理 recent_events 数组
func play_events(events: Array):
	if events.is_empty():
		return
	for event in events:
		if typeof(event) != TYPE_DICTIONARY:
			continue
		for event_type in event.keys():
			var data = event[event_type]
			_play_event(event_type, data)


func _play_event(event_type: String, data):
	match event_type:
		"DrawCard":
			var iid = data.get("instance_id", 0) if typeof(data) == TYPE_DICTIONARY else 0
			var card = _find_label_by_iid(iid)
			if card:
				card.modulate = Color(1, 1, 1, 0.0)
				var tween = _board.create_tween()
				tween.tween_property(card, "modulate", Color.WHITE, 0.3)
				Types.VerifyLogger.log_animation("DrawCard", iid, "fade-in")
			else:
				Types.VerifyLogger.log_skip("DrawCard", "card iid=%d not found" % iid)

		"CardSummoned":
			var iid = data.get("instance_id", 0) if typeof(data) == TYPE_DICTIONARY else 0
			var card = _find_label_by_iid(iid)
			if card:
				card.scale = Vector2(0.6, 0.6)
				card.modulate = Color(1, 1, 1, 0.2)
				var tween = _board.create_tween()
				tween.set_parallel()
				tween.tween_property(card, "scale", Vector2(1.0, 1.0), 0.3).set_ease(Tween.EASE_OUT).set_trans(Tween.TRANS_BACK)
				tween.tween_property(card, "modulate", Color.WHITE, 0.2)
				Types.VerifyLogger.log_animation("CardSummoned", iid, "pop-in")
			else:
				Types.VerifyLogger.log_skip("CardSummoned", "card iid=%d not found" % iid)

		"CardExposed":
			var iid = data.get("instance_id", 0) if typeof(data) == TYPE_DICTIONARY else 0
			var card = _find_label_by_iid(iid)
			if card:
				card.rotation = deg_to_rad(90)
				card.scale = Vector2(0.6, 0.6)
				var tween = _board.create_tween()
				tween.set_parallel()
				tween.tween_property(card, "rotation", 0.0, 0.25).set_ease(Tween.EASE_OUT)
				tween.tween_property(card, "scale", Vector2(1.0, 1.0), 0.25)
				Types.VerifyLogger.log_animation("CardExposed", iid, "flip-in")
			else:
				Types.VerifyLogger.log_skip("CardExposed", "card iid=%d not found" % iid)

		"CardDestroyed":
			var iid = data.get("instance_id", 0) if typeof(data) == TYPE_DICTIONARY else 0
			var card = _find_label_by_iid(iid)
			if card:
				var tween = _board.create_tween()
				tween.set_parallel()
				tween.tween_property(card, "scale", Vector2(0.3, 0.3), 0.3).set_ease(Tween.EASE_IN)
				tween.tween_property(card, "modulate", Color(1, 0.3, 0.3, 0.0), 0.3)
				Types.VerifyLogger.log_animation("CardDestroyed", iid, "shrink-out")
			else:
				Types.VerifyLogger.log_skip("CardDestroyed", "card iid=%d not found" % iid)

		"HpChanged":
			Types.VerifyLogger.log_animation("HpChanged", 0,
				"%s %d→%d" % [data.get("player", ""), data.get("old_hp", 0), data.get("new_hp", 0)])

		"PhaseChanged":
			Types.VerifyLogger.log_animation("PhaseChanged", 0,
				"→ %s" % data.get("new_phase", ""))

		"TurnChanged":
			Types.VerifyLogger.log_animation("TurnChanged", 0,
				"→ player=%s turn=%d" % [data.get("new_active_player", ""), data.get("turn_number", 0)])

		"AttackDeclared":
			var attacker = data.get("attacker", 0) if typeof(data) == TYPE_DICTIONARY else 0
			Types.VerifyLogger.log_animation("AttackDeclared", attacker, "shake")


## 在所有棋盘区域中搜索 instance_id 对应的 Label
func _find_label_by_iid(instance_id: int) -> Control:
	var zones: Array[Control] = [
		_board.self_hand, _board.opp_hand,
		_board.self_front, _board.opp_front,
		_board.self_back, _board.opp_back,
		_board.self_cost, _board.opp_cost,
		_board.self_grave, _board.opp_grave,
	]
	for zone in zones:
		if zone == null:
			continue
		for child in zone.get_children():
			if child is Label and child.get_meta("iid", 0) == instance_id:
				return child
	return null
