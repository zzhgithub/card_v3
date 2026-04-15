## 费用区域容器
## 继承自 CardContainer，竖直排列最多6张卡
## 用于显示已支付的费用卡片

class_name CostZone
extends CardContainer

const MAX_COST_CARDS = 6
const COST_GAP = 40  ## 费用卡之间的叠放间距

@export var cost_label: String = "费用"

var background: ColorRect
var title_label: Label

const BG_COLOR = Color(0.15, 0.15, 0.2, 0.8)
const BORDER_COLOR = Color(0.4, 0.4, 0.5, 1.0)

func _ready() -> void:
	# 创建背景
	background = ColorRect.new()
	background.name = "Background"
	background.color = BG_COLOR
	background.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(background)
	move_child(background, 0)

	# 创建标题标签
	title_label = Label.new()
	title_label.name = "TitleLabel"
	title_label.text = cost_label
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	title_label.add_theme_font_size_override("font_size", 14)
	title_label.add_theme_color_override("font_color", Color(0.9, 0.9, 0.9))
	add_child(title_label)

	_update_background_size()

	# 启用 drop zone
	enable_drop_zone = true

	super._ready()


func _draw() -> void:
	# 绘制边框
	var rect = Rect2(Vector2.ZERO, size)
	draw_rect(rect, BORDER_COLOR, false, 2.0)


func _update_background_size() -> void:
	if background:
		background.size = size
	if title_label:
		# 标题显示在费用区上方
		title_label.position = Vector2(0, -20)
		title_label.size = Vector2(size.x, 20)


## 重写：最多6张卡
func _card_can_be_added(cards: Array) -> bool:
	var current_count = _held_cards.size()
	var adding_count = cards.size()
	return current_count + adding_count <= MAX_COST_CARDS


## 重写：竖直排列，叠放效果
func _update_target_positions() -> void:
	var card_size = card_manager.card_size if card_manager else Vector2(120, 170)

	for i in range(_held_cards.size()):
		var card = _held_cards[i]

		# 竖直排列，从顶部开始，有叠放效果
		var offset_y = i * COST_GAP
		var target_pos = Vector2((size.x - card_size.x) / 2.0, offset_y)

		card.move(target_pos, 0)
		card.show_front = true  ## 费用卡显示正面
		card.can_be_interacted_with = true


## 重写：更新 z-index（后面的在上面）
func _update_target_z_index() -> void:
	for i in range(_held_cards.size()):
		# 后面的卡片有更高的 z-index
		_held_cards[i].stored_z_index = i


## 添加费用卡（添加到末尾）
func add_cost_card(card: Card) -> bool:
	if _held_cards.size() >= MAX_COST_CARDS:
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


## 当尺寸变化时更新背景
func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		_update_background_size()
