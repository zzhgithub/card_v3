## 卡组/墓地堆叠容器
## 继承自 Pile，添加剩余数量显示

class_name CardStack
extends Pile

enum StackType {
	DECK,   ## 卡组
	GRAVE   ## 墓地
}

enum StackLayoutDirection {
	VERTICAL,   ## 竖直布局（从上到下）
	HORIZONTAL  ## 水平布局（从左到右）
}

@export var stack_type: StackType = StackType.DECK
@export var show_count_label: bool = true
@export var stack_layout: StackLayoutDirection = StackLayoutDirection.VERTICAL
## 每张卡片的额外旋转偏移（弧度）
@export var card_rotation_offset := 0.0
## 堆叠间距（浮点，支持小数偏移）
@export var stack_gap: float = 0.5

var count_label: Label
var stack_name_label: Label
var background: ColorRect

const STACK_NAME = {
	StackType.DECK: "卡组",
	StackType.GRAVE: "墓地"
}

const BACKGROUND_COLOR = Color(0.2, 0.2, 0.2, 0.5)
const BORDER_COLOR = Color(0.5, 0.5, 0.5, 0.8)

func _ready() -> void:
	# 确保标签不被裁剪
	clip_contents = false

	# 创建背景
	background = ColorRect.new()
	background.name = "Background"
	background.color = BACKGROUND_COLOR
	background.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(background)
	move_child(background, 0)

	# 创建数量标签
	count_label = Label.new()
	count_label.name = "CountLabel"
	count_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	count_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	count_label.add_theme_font_size_override("font_size", 16)
	count_label.add_theme_color_override("font_color", Color.WHITE)
	add_child(count_label)

	# 创建名称标签
	stack_name_label = Label.new()
	stack_name_label.name = "StackNameLabel"
	stack_name_label.text = STACK_NAME[stack_type]
	stack_name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	stack_name_label.add_theme_font_size_override("font_size", 14)
	stack_name_label.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	add_child(stack_name_label)

	# 根据布局方向设置堆叠方向（统一为右下）
	layout = PileDirection.DOWN

	# 设置默认堆叠间距（体现厚度）
	stack_gap = 0.5
	max_stack_display = 40

	# 更新标签位置
	_update_label_positions()

	super._ready()


## 重写：更新目标位置（控制卡片正反面、旋转）
func _update_target_positions() -> void:
	# Calculate top card position for drop zone alignment
	var last_index = _held_cards.size() - 1
	if last_index < 0:
		last_index = 0
	var last_offset = _calculate_offset(last_index)

	# Align drop zone with top card if enabled
	if enable_drop_zone and align_drop_zone_with_top_card:
		drop_zone.change_sensor_position_with_offset(last_offset)

	# Position each card and set interaction state
	for i in range(_held_cards.size()):
		var card = _held_cards[i]
		var offset = _calculate_offset(i)
		var target_pos = global_position + offset

		# Set card appearance, position, and rotation
		card.show_front = card_face_up
		card.move(target_pos, card_rotation_offset)

		# CardStack 中的卡片不可被拖拽
		card.can_be_interacted_with = false

	_update_label_positions()
	_update_count_display()


## 更新标签位置
func _update_label_positions() -> void:
	if count_label == null or stack_name_label == null or background == null:
		return

	# 更新背景大小
	background.size = size

	# 根据布局方向调整标签位置
	if stack_layout == StackLayoutDirection.VERTICAL:
		# 竖直布局：标签显示在堆叠右侧
		count_label.position = Vector2(size.x + 5, 5)
		count_label.size = Vector2(30, 20)

		stack_name_label.position = Vector2(size.x + 5, 30)
		stack_name_label.size = Vector2(60, 20)
	else:
		# 水平布局：标签显示在堆叠下方
		count_label.position = Vector2(size.x - 30, size.y + 5)
		count_label.size = Vector2(30, 20)

		stack_name_label.position = Vector2(0, size.y + 20)
		stack_name_label.size = Vector2(size.x, 20)


## 更新数量显示
func _update_count_display() -> void:
	if count_label == null:
		return

	var count = get_card_count()
	count_label.text = str(count)
	count_label.visible = count > 0 and show_count_label


## 获取顶部卡片
func peek_top_card() -> Card:
	var cards = get_top_cards(1)
	if cards.is_empty():
		return null
	return cards[0]


## 抽取顶部卡片（不通过拖拽）
func draw_top_card() -> Card:
	var card = peek_top_card()
	if card:
		remove_card(card)
	return card


## 向卡组添加卡片（放到顶部）
func add_card_to_top(card: Card) -> void:
	add_card(card, -1)


## 重写：卡片向右下方堆叠（同时向右和向下偏移）
func _calculate_offset(index: int) -> Vector2:
	var actual_index = min(index, max_stack_display - 1)
	var offset_value = actual_index * stack_gap
	return Vector2(offset_value, offset_value)


## 绘制边框
func _draw() -> void:
	var rect = Rect2(Vector2.ZERO, size)
	draw_rect(rect, BORDER_COLOR, false, 2.0)


## 当尺寸变化时更新标签
func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		if background:
			background.size = size
		_update_label_positions()
