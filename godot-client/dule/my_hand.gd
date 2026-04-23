class_name MyHand
extends CardContainer

@export_group("hand_meta_info")
@export var max_hand_size := CardFrameworkSettings.LAYOUT_MAX_HAND_SIZE
@export var card_face_up := true
@export var card_hover_distance := CardFrameworkSettings.PHYSICS_CARD_HOVER_DISTANCE

@export_group("hand_layout")
@export var card_gap := 10.0
@export var align_drop_zone_size_with_current_hand_size := true
@export var swap_only_on_reorder := false


func _ready() -> void:
	super._ready()
	resized.connect(_on_my_hand_resized)
	# 确保 drop_zone 不拦截鼠标事件
	if drop_zone != null:
		drop_zone.visible = false


func _on_my_hand_resized() -> void:
	if cards_node != null:
		cards_node.size = size


func _card_can_be_added(_cards: Array) -> bool:
	var is_all_cards_contained = true
	for card in _cards:
		if !_held_cards.has(card):
			is_all_cards_contained = false
	if is_all_cards_contained:
		return true
	return _held_cards.size() + _cards.size() <= max_hand_size


func _update_target_z_index() -> void:
	for i in range(_held_cards.size()):
		var card = _held_cards[i]
		card.stored_z_index = i


func _update_target_positions() -> void:
	var card_size = card_manager.card_size
	var _w = card_size.x
	var _h = card_size.y

	var total_cards = _held_cards.size()
	if total_cards == 0:
		if align_drop_zone_size_with_current_hand_size and enable_drop_zone and drop_zone != null:
			drop_zone.return_sensor_size()
		return

	var available_width = size.x
	var available_height = size.y

	# 计算间距：优先使用 card_gap，如果放不下则压缩
	var spacing = _w + card_gap
	var total_width = total_cards * _w + (total_cards - 1) * card_gap
	if total_width > available_width and total_cards > 1:
		spacing = (available_width - _w) / float(total_cards - 1)

	var actual_total_width = total_cards * _w + (total_cards - 1) * (spacing - _w)
	var start_x = (available_width - actual_total_width) / 2.0
	var center_y = available_height / 2.0 - _h / 2.0

	var x_min = INF
	var x_max = -INF
	var y_min = INF
	var y_max = -INF

	var card_mids: Array = []

	for i in range(total_cards):
		var card = _held_cards[i]
		var local_pos = Vector2.ZERO

		if total_cards == 1:
			local_pos.x = available_width / 2.0 - _w / 2.0
		else:
			local_pos.x = start_x + i * spacing

		local_pos.y = center_y

		var global_pos = global_position + local_pos

		card_mids.append(global_pos.x + _w / 2.0)

		x_min = min(x_min, local_pos.x)
		x_max = max(x_max, local_pos.x + _w)
		y_min = min(y_min, local_pos.y)
		y_max = max(y_max, local_pos.y + _h)

		card.move(global_pos, 0)
		card.show_front = card_face_up
		card.can_be_interacted_with = true

	# 更新 drop zone
	if align_drop_zone_size_with_current_hand_size and enable_drop_zone and drop_zone != null:
		var _size = Vector2(x_max - x_min, y_max - y_min)
		drop_zone.set_sensor_size_flexibly(_size, Vector2(x_min, y_min))

		# 设置 vertical partitions 用于精确拖放排序
		var vertical_partitions: Array = []
		for j in range(card_mids.size() - 1):
			var mid = (card_mids[j] + card_mids[j + 1]) / 2.0
			vertical_partitions.append(mid)
		drop_zone.set_vertical_partitions(vertical_partitions)


func move_cards(cards: Array, index: int = -1, with_history: bool = true) -> bool:
	if cards.size() == 1 and _held_cards.has(cards[0]) and index >= 0 and index < _held_cards.size():
		var current_index = _held_cards.find(cards[0])
		if swap_only_on_reorder:
			swap_card(cards[0], index)
			return true
		if current_index == index:
			update_card_ui()
			_restore_mouse_interaction(cards)
			return true
		_reorder_card_in_hand(cards[0], current_index, index, with_history)
		_restore_mouse_interaction(cards)
		return true
	return super.move_cards(cards, index, with_history)


func swap_card(card: Card, index: int) -> void:
	var current_index = _held_cards.find(card)
	if current_index == index:
		return
	var temp = _held_cards[current_index]
	_held_cards[current_index] = _held_cards[index]
	_held_cards[index] = temp
	update_card_ui()


func _restore_mouse_interaction(cards: Array) -> void:
	for card in cards:
		card.mouse_filter = Control.MOUSE_FILTER_STOP


func _reorder_card_in_hand(card: Card, from_index: int, to_index: int, with_history: bool) -> void:
	if with_history:
		card_manager._add_history(self, [card])
	_held_cards.remove_at(from_index)
	_held_cards.insert(to_index, card)
	update_card_ui()


func hold_card(card: Card) -> void:
	if _held_cards.has(card):
		super.hold_card(card)
