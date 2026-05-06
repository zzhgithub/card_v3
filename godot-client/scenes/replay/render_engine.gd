extends RefCounted
const Types = preload("res://scenes/replay/replay_types.gd")



var _board  # ReplayBoard


func _init(board):  # ReplayBoard
	_board = board


## 应用一组 ZoneChange
func apply_changes(changes: Array):
	for ch in changes:
		_apply_zone_change(ch)


func _apply_zone_change(ch):  # ZoneChange
	var side = ch.player_side
	var is_self = (side == "self")

	match ch.zone_name:
		"hand":
			var container = _board.self_hand if is_self else _board.opp_hand
			_apply_adds(container, ch.adds, -1)
			_apply_removes(container, ch.removes)

		"front":
			var container = _board.self_front if is_self else _board.opp_front
			if ch.slot_index >= 0:
				for a in ch.adds:
					_board.add_card_to_zone(container, a, ch.slot_index)
			for r in ch.removes:
				_board.remove_card_from_zone(container, r)

		"back":
			var container = _board.self_back if is_self else _board.opp_back
			if ch.slot_index >= 0:
				for a in ch.adds:
					_board.add_card_to_zone(container, a, ch.slot_index)
			for r in ch.removes:
				_board.remove_card_from_zone(container, r)

		"cost_zone":
			var container = _board.self_cost if is_self else _board.opp_cost
			_apply_adds(container, ch.adds, -1)
			_apply_removes(container, ch.removes)

		"grave":
			var container = _board.self_grave if is_self else _board.opp_grave
			_apply_adds(container, ch.adds, -1)
			_apply_removes(container, ch.removes)

		"HP":
			var val = ch.adds[0].get("value", 0) if not ch.adds.is_empty() else 0
			if is_self: _board.update_self_hp(val)
			else: _board.update_opp_hp(val)

		"RP":
			var val = ch.adds[0].get("value", 0) if not ch.adds.is_empty() else 0
			if is_self: _board.update_self_rp(val)
			else: _board.update_opp_rp(val)

		"DECK":
			var val = ch.adds[0].get("value", 0) if not ch.adds.is_empty() else 0
			if is_self or side == "":  # 首次加载无 side
				if is_self or ch.zone_name == "DECK":
					if _board.has_method("update_self_deck") and is_self:
						_board.update_self_deck(val)
					elif side == "opponent":
						_board.update_opp_deck(val)
					else:
						# 尝试从 zone_name 推断
						if ch.player_side == "self":
							_board.update_self_deck(val)
						elif ch.player_side == "opponent":
							_board.update_opp_deck(val)
						else:
							_board.update_self_deck(val)
				else:
					_board.update_opp_deck(val)

		"HAND":
			# opponent hand_count
			var val = ch.adds[0].get("value", 0) if not ch.adds.is_empty() else 0
			_board.update_opp_hand_count(val)


func _apply_adds(container: Control, adds: Array, _slot_idx: int):
	for card_info in adds:
		_board.add_card_to_zone(container, card_info)


func _apply_removes(container: Control, removes: Array):
	for iid in removes:
		_board.remove_card_from_zone(container, iid)
