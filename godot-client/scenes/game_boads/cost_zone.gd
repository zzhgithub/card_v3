class_name CostZone
extends CardContainer

@export_group("cost_meta_info")
@export var max_cost_size := 6
@export var card_face_up := true
@export var card_hover_distance := CardFrameworkSettings.PHYSICS_CARD_HOVER_DISTANCE

@export_group("cost_layout")
@export var card_gap := 10.0
## 每张卡片的额外旋转偏移（弧度）
@export var card_rotation_offset := 0.0
## 容器中的卡片是否可以被拖拽
@export var cards_draggable := true
## 不超过此数量时，费用卡在容器高度内正常展开；超过后在此高度内均匀压缩
@export var max_cards_without_stack := 6
## 超过 max_cards_without_stack 后的最大展开高度
@export var cost_spread_height := 400.0

const BG_COLOR = Color(0.15, 0.15, 0.2, 0.8)
const BORDER_COLOR = Color(0.4, 0.4, 0.5, 1.0)

var background: ColorRect

func _ready() -> void:
	# 创建背景
	background = ColorRect.new()
	background.name = "Background"
	background.color = BG_COLOR
	background.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(background)
	move_child(background, 0)

	super._ready()
	resized.connect(_on_cost_zone_resized)
	if drop_zone != null:
		drop_zone.visible = false


func _on_cost_zone_resized() -> void:
	if cards_node != null:
		cards_node.size = size
	if background != null:
		background.size = size


func _draw() -> void:
	var rect = Rect2(Vector2.ZERO, size)
	draw_rect(rect, BORDER_COLOR, false, 2.0)


func _card_can_be_added(_cards: Array) -> bool:
	var is_all_cards_contained = true
	for card in _cards:
		if !_held_cards.has(card):
			is_all_cards_contained = false
	if is_all_cards_contained:
		return true
	return _held_cards.size() + _cards.size() <= max_cost_size


func _update_target_z_index() -> void:
	for i in range(_held_cards.size()):
		_held_cards[i].stored_z_index = i


func _update_target_positions() -> void:
	var card_size = card_manager.card_size
	var _w = card_size.x
	var _h = card_size.y
	var total_cards = _held_cards.size()

	if total_cards == 0:
		return

	var container_width = size.x
	var container_height = size.y
	var center_x = container_width / 2.0 - _w / 2.0

	# 不超过 max_cards_without_stack 时用容器高度；超过后用 cost_spread_height
	var spread_height: float
	if total_cards <= max_cards_without_stack:
		spread_height = container_height
	else:
		spread_height = min(cost_spread_height, container_height)

	# 计算正常等距排列的总高度
	var normal_height = total_cards * _h + (total_cards - 1) * card_gap
	var use_stacked_mode = normal_height > spread_height

	if not use_stacked_mode:
		# === 正常模式：在 spread_height 内居中排列 ===
		var content_start = (spread_height - normal_height) / 2.0

		for i in range(total_cards):
			var card = _held_cards[i]
			var local_pos = Vector2.ZERO
			local_pos.x = center_x
			local_pos.y = content_start + i * (_h + card_gap)

			var global_pos = global_position + local_pos
			card.move(global_pos, card_rotation_offset)
			card.show_front = card_face_up
			card.can_be_interacted_with = cards_draggable
	else:
		# === 堆叠模式：在容器高度内均匀压缩 ===
		var spacing: float
		if total_cards > 1:
			spacing = (spread_height - _h) / float(total_cards - 1)
		else:
			spacing = 0.0

		var total_visual_height = (total_cards - 1) * spacing + _h
		var stack_offset_y = (container_height - total_visual_height) / 2.0

		for i in range(total_cards):
			var card = _held_cards[i]

			var local_pos = Vector2.ZERO
			local_pos.x = center_x
			local_pos.y = stack_offset_y + i * spacing

			var global_pos = global_position + local_pos
			card.move(global_pos, card_rotation_offset)
			card.show_front = card_face_up
			card.can_be_interacted_with = cards_draggable


## 添加费用卡（添加到末尾）
func add_cost_card(card: Card) -> bool:
	if _held_cards.size() >= max_cost_size:
		return false
	add_card(card, -1)
	return true


## 清空费用区（返回所有卡片）
func clear_cost_zone() -> Array:
	var cards = _held_cards.duplicate()
	clear_cards()
	return cards


## 获取费用数量
func get_cost_count() -> int:
	return _held_cards.size()
