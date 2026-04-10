## 卡片大图预览弹窗
## 居中显示卡片大图和详细信息

class_name CardPreviewPopup
extends Control

signal reserve_requested(card_data: Dictionary)
signal closed

const CARD_DISPLAY_SCENE = preload("res://scenes/card_display.tscn")

var card_data: Dictionary = {}

@onready var overlay: ColorRect = $Overlay
@onready var content: CenterContainer = $Content
@onready var close_button: Button = $CloseButton

func _ready():
	# 设置全屏
	set_anchors_preset(Control.PRESET_FULL_RECT)

	# 连接关闭按钮
	close_button.pressed.connect(_on_close_pressed)

	# 点击背景关闭
	overlay.gui_input.connect(_on_overlay_input)

	# 动画进入
	modulate = Color(1, 1, 1, 0)
	var tween = create_tween()
	tween.tween_property(self, "modulate", Color(1, 1, 1, 1), 0.2)


## 设置要显示的卡片数据
func setup(data: Dictionary) -> void:
	card_data = data
	var card_id = data.get("id", "Unknown")

	# 获取内容容器中的卡片容器
	var card_container = content.get_node("CardContainer")

	# 清除旧内容（如果存在）
	for child in card_container.get_children():
		child.queue_free()

	# 创建大图卡片显示
	var large_display = CARD_DISPLAY_SCENE.instantiate()
	large_display.custom_minimum_size = Vector2(300, 420)
	large_display.size_flags_horizontal = Control.SIZE_SHRINK_CENTER

	var front_image = _load_card_image(card_id)
	large_display.setup(data, front_image)
	card_container.add_child(large_display)

	# 卡片名称
	var name_label = Label.new()
	name_label.text = data.get("name", card_id)
	name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	name_label.add_theme_font_size_override("font_size", 24)
	name_label.add_theme_color_override("font_color", Color.WHITE)
	card_container.add_child(name_label)

	# 详细属性
	var detail_text = _format_card_detail(data)
	var detail_label = RichTextLabel.new()
	detail_label.bbcode_enabled = true
	detail_label.text = detail_text
	detail_label.custom_minimum_size = Vector2(350, 200)
	detail_label.fit_content = true
	detail_label.add_theme_color_override("default_color", Color.WHITE)
	card_container.add_child(detail_label)

	# 预留按钮
	var reserve_button = Button.new()
	reserve_button.text = "预留此卡"
	reserve_button.custom_minimum_size = Vector2(120, 40)
	reserve_button.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	reserve_button.pressed.connect(_on_reserve_pressed)
	card_container.add_child(reserve_button)


## 加载卡片图片
func _load_card_image(card_id: String) -> Texture2D:
	var image_path = "res://images/cards/%s.png" % card_id
	if ResourceLoader.exists(image_path):
		return load(image_path) as Texture2D
	return null


## 格式化卡片详情文本
func _format_card_detail(data: Dictionary) -> String:
	var result = ""

	result += "[b]编号:[/b] %s\n" % data.get("id", "Unknown")
	result += "[b]类型:[/b] %s\n" % data.get("card_type", "Unknown")

	match data.get("card_type"):
		"Character":
			result += "[b]属性:[/b] %s\n" % data.get("property", "-")
			result += "[b]领域:[/b] %s\n" % data.get("category", "-")
			result += "[b]费用:[/b] %d\n" % data.get("cost", 0)
			result += "[b]攻击力:[/b] %d\n" % data.get("attack", 0)
		"Item":
			result += "[b]种类:[/b] %s\n" % data.get("item_kind", "-")
			result += "[b]属性:[/b] %s\n" % data.get("property", "-")
			result += "[b]领域:[/b] %s\n" % data.get("category", "-")
			result += "[b]费用:[/b] %d\n" % data.get("cost", 0)
		"Strategy":
			result += "[b]种类:[/b] %s\n" % data.get("strategy_kind", "-")
			result += "[b]属性:[/b] %s\n" % data.get("property", "-")
			result += "[b]领域:[/b] %s\n" % data.get("category", "-")
			result += "[b]费用:[/b] %d\n" % data.get("cost", 0)
		"Legend":
			result += "[b]属性:[/b] %s\n" % data.get("property", "-")
			result += "[b]费用:[/b] %d\n" % data.get("cost", 0)

	var effects = data.get("effects", {})
	if not effects.is_empty():
		result += "\n[b]效果:[/b]"
		for effect_id in effects:
			var effect = effects[effect_id]
			var trigger = effect.get("trigger", "")
			result += "\n• %s" % trigger

	return result


## 点击背景关闭
func _on_overlay_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed:
		_on_close_pressed()


## 关闭弹窗
func _on_close_pressed() -> void:
	# 动画退出
	var tween = create_tween()
	tween.tween_property(self, "modulate", Color(1, 1, 1, 0), 0.15)
	tween.tween_callback(func():
		emit_signal("closed")
		queue_free()
	)


## 预留按钮点击
func _on_reserve_pressed() -> void:
	emit_signal("reserve_requested", card_data)
	_on_close_pressed()


## 静态方法：便捷创建并显示预览弹窗
static func show_preview(parent: Node, data: Dictionary) -> CardPreviewPopup:
	var popup = preload("res://scenes/card_preview_popup.tscn").instantiate()
	parent.add_child(popup)
	popup.setup(data)
	return popup
