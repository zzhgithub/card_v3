## 战场格子容器
## 继承自 CardContainer，用于放置单张卡片
## 支持前场(淡蓝色)和后场(淡橙色)两种背景色

class_name BattleFieldSlot
extends CardContainer

enum FieldType {
	FRONT,  ## 前场 - 淡蓝色背景
	BACK    ## 后场 - 淡橙色背景
}

@export var field_type: FieldType = FieldType.FRONT
@export var slot_index: int = 0  ## 格子索引 (0-4)

var background: ColorRect

const FRONT_COLOR = Color(0.7, 0.85, 1.0, 0.6)  ## 淡蓝色
const BACK_COLOR = Color(1.0, 0.85, 0.7, 0.6)   ## 淡橙色
const BORDER_COLOR = Color(0.5, 0.5, 0.5, 0.8)

func _ready() -> void:
	# 创建背景
	background = ColorRect.new()
	background.name = "Background"
	background.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(background)
	move_child(background, 0)

	# 设置背景色
	_update_background()

	# 确保 drop zone 接受卡片
	enable_drop_zone = true

	super._ready()


func _update_background() -> void:
	if background == null:
		return

	var color = FRONT_COLOR if field_type == FieldType.FRONT else BACK_COLOR
	background.color = color

	# 添加边框效果
	background.queue_redraw()


func _draw() -> void:
	# 绘制边框
	var rect = Rect2(Vector2.ZERO, size)
	draw_rect(rect, BORDER_COLOR, false, 2.0)


## 重写：只能放置一张卡
func _card_can_be_added(_cards: Array) -> bool:
	# 如果已经有卡片，不允许添加
	if _held_cards.size() >= 1:
		return false
	return true


## 重写：更新位置（居中显示）
func _update_target_positions() -> void:
	if _held_cards.is_empty():
		return

	var card = _held_cards[0]
	var card_size = card_manager.card_size if card_manager else Vector2(180, 252)

	# 计算居中位置
	var center_x = (size.x - card_size.x) / 2.0
	var center_y = (size.y - card_size.y) / 2.0

	var target_pos = Vector2(center_x, center_y)
	card.move(target_pos, 0)
	card.show_front = true
	card.can_be_interacted_with = true


## 重写：更新 z-index
func _update_target_z_index() -> void:
	for i in range(_held_cards.size()):
		_held_cards[i].stored_z_index = i


## 获取当前卡片
func get_card() -> Card:
	if _held_cards.is_empty():
		return null
	return _held_cards[0]


## 是否有卡片
func has_card(card: Card = null) -> bool:
	if card == null:
		return not _held_cards.is_empty()
	return card in _held_cards


## 当尺寸变化时更新背景
func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		if background:
			background.size = size
