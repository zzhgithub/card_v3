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
var attack_label: Label

const FRONT_COLOR = Color(0.7, 0.85, 1.0, 0.6)  ## 淡蓝色
const BACK_COLOR = Color(1.0, 0.85, 0.7, 0.6)   ## 淡橙色
const BORDER_COLOR = Color(0.5, 0.5, 0.5, 0.8)

## 标签在卡片上方的垂直偏移
const ATTACK_LABEL_OFFSET_Y = -22

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

	# 创建攻击力标签（使用 top_level 确保独立绘制在最高层）
	attack_label = Label.new()
	attack_label.name = "AttackLabel"
	attack_label.add_theme_font_size_override("font_size", 18)
	attack_label.add_theme_color_override("font_color", Color.WHITE)
	attack_label.add_theme_color_override("font_outline_color", Color.BLACK)
	attack_label.add_theme_constant_override("outline_size", 3)
	attack_label.visible = false
	# top_level 让标签脱离 slot 的变换与层级，使用全局坐标绘制在最上层
	attack_label.top_level = true
	add_child(attack_label)

	super._ready()


func _update_background() -> void:
	if background == null:
		return

	var color = FRONT_COLOR if field_type == FieldType.FRONT else BACK_COLOR
	background.color = color
	background.queue_redraw()


func _draw() -> void:
	var rect = Rect2(Vector2.ZERO, size)
	draw_rect(rect, BORDER_COLOR, false, 2.0)


## 重写：只能放置一张卡
func _card_can_be_added(_cards: Array) -> bool:
	if _held_cards.size() >= 1:
		return false
	return true


## 重写：更新位置（居中显示）
func _update_target_positions() -> void:
	if _held_cards.is_empty():
		attack_label.visible = false
		return

	var card = _held_cards[0]
	var card_size = card_manager.card_size if card_manager else Vector2(180, 252)

	# 计算居中位置
	var center_x = (size.x - card_size.x) / 2.0
	var center_y = (size.y - card_size.y) / 2.0

	var target_pos = global_position + Vector2(center_x, center_y)
	card.move(target_pos, 0)
	card.show_front = true
	card.can_be_interacted_with = true

	# 更新攻击力标签全局位置：卡片左上角 + 向上偏移，显示在卡片上方
	if attack_label:
		attack_label.global_position = global_position + Vector2(center_x + 4, center_y + ATTACK_LABEL_OFFSET_Y)

	_update_attack_display()


## 重写：更新 z-index
func _update_target_z_index() -> void:
	for i in range(_held_cards.size()):
		_held_cards[i].stored_z_index = i


## 重写：清除卡片时隐藏标签
func clear_cards() -> void:
	if attack_label:
		attack_label.visible = false
	super.clear_cards()


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


## 更新攻击力显示
func _update_attack_display() -> void:
	if attack_label == null:
		return

	if _held_cards.is_empty():
		attack_label.visible = false
		return

	var card = _held_cards[0]
	var attack_value = ""
	if card is DuleCard:
		var dule_card = card as DuleCard
		if not dule_card.card_data.is_empty():
			# 优先读取 current_attack（状态同步值），回退到 attack（卡面定义值）
			var attack = dule_card.card_data.get("current_attack", null)
			if attack == null:
				attack = dule_card.card_data.get("attack", 0)
			attack_value = "ACK: %d" % int(attack)

	attack_label.text = attack_value
	attack_label.visible = attack_value != ""


## 当尺寸变化时更新背景
func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		if background:
			background.size = size
