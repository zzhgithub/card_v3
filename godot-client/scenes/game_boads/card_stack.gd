## 卡组/墓地堆叠容器
## 继承自 Pile，添加剩余数量显示

class_name CardStack
extends Pile

enum StackType {
	DECK,   ## 卡组
	GRAVE   ## 墓地
}

@export var stack_type: StackType = StackType.DECK
@export var show_count_label: bool = true

var count_label: Label
var stack_name_label: Label

const STACK_NAME = {
	StackType.DECK: "卡组",
	StackType.GRAVE: "墓地"
}

func _ready() -> void:
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

	# 更新标签位置
	_update_label_positions()

	# 设置默认堆叠方向（向下）
	layout = PileDirection.DOWN

	# 设置默认堆叠间距（体现厚度）
	stack_display_gap = 3

	super._ready()


## 重写：更新目标位置
func _update_target_positions() -> void:
	super._update_target_positions()
	_update_label_positions()
	_update_count_display()


## 更新标签位置
func _update_label_positions() -> void:
	if count_label == null or stack_name_label == null:
		return

	# 数量标签显示在堆叠右下角
	count_label.position = Vector2(size.x - 30, size.y + 5)

	# 名称标签显示在堆叠下方
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


## 当尺寸变化时更新标签
func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		_update_label_positions()
