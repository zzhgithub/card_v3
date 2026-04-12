## 卡片大图预览弹窗
## 左右结构：左侧大图(90%屏幕高度)，右侧详情

class_name CardPreviewPopup
extends Control

signal closed

const PREVIEW_CARD_SCENE = preload("res://scenes/preview_card_display.tscn")

var card_data: Dictionary = {}

@onready var overlay: ColorRect = $Overlay
@onready var content: HBoxContainer = $Content
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

	# 清除旧内容
	for child in content.get_children():
		child.queue_free()

	# 创建左侧卡片显示区域（占90%屏幕高度）
	var left_container = CenterContainer.new()
	left_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	left_container.size_flags_vertical = Control.SIZE_EXPAND_FILL
	left_container.mouse_filter = Control.MOUSE_FILTER_STOP
	content.add_child(left_container)

	# 使用专门的预览卡片显示（无悬停效果，大图模板模式）
	var preview_display = PREVIEW_CARD_SCENE.instantiate()
	# 计算高度为屏幕高度的90%
	var screen_height = get_viewport_rect().size.y
	var card_height = screen_height * 0.9
	var card_width = card_height * 1500.0 / 2100.0  # 保持卡片比例（基于大图尺寸）
	preview_display.custom_minimum_size = Vector2(card_width, card_height)
	preview_display.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	preview_display.size_flags_vertical = Control.SIZE_SHRINK_CENTER

	# 使用模板渲染（不传入图片）
	preview_display.setup(data)
	left_container.add_child(preview_display)

	# 创建右侧详情面板（半透明背景）
	var right_panel = ColorRect.new()
	right_panel.custom_minimum_size = Vector2(450, 0)
	right_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	right_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
	right_panel.color = Color(0.1, 0.1, 0.15, 0.85)
	right_panel.mouse_filter = Control.MOUSE_FILTER_STOP
	content.add_child(right_panel)

	# 右侧内容容器
	var right_container = VBoxContainer.new()
	right_container.layout_mode = 1
	right_container.anchors_preset = Control.PRESET_FULL_RECT
	right_container.offset_left = 20
	right_container.offset_top = 18
	right_container.offset_right = -20
	right_container.offset_bottom = -20
	right_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	right_container.size_flags_vertical = Control.SIZE_EXPAND_FILL
	right_container.add_theme_constant_override("separation", 15)
	right_panel.add_child(right_container)

	# 卡片名称
	var name_label = Label.new()
	name_label.text = data.get("name", card_id)
	name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT
	name_label.add_theme_font_size_override("font_size", 28)
	name_label.add_theme_color_override("font_color", Color.WHITE)
	right_container.add_child(name_label)

	# 分隔线
	var separator = ColorRect.new()
	separator.custom_minimum_size = Vector2(0, 2)
	separator.color = Color(0.5, 0.5, 0.5, 0.5)
	right_container.add_child(separator)

	# 详细属性
	var detail_text = _format_card_detail(data)
	var detail_label = RichTextLabel.new()
	detail_label.bbcode_enabled = true
	detail_label.text = detail_text
	detail_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	detail_label.size_flags_vertical = Control.SIZE_EXPAND_FILL
	detail_label.fit_content = true
	detail_label.scroll_active = false
	detail_label.add_theme_color_override("default_color", Color.WHITE)
	detail_label.add_theme_font_size_override("normal_font_size", 16)
	right_container.add_child(detail_label)


## 加载卡片图片
func _load_card_image(card_id: String) -> Texture2D:
	var image_path = "res://images/cards/%s.png" % card_id
	if ResourceLoader.exists(image_path):
		return load(image_path) as Texture2D
	return null


## 格式化卡片详情文本
func _format_card_detail(data: Dictionary) -> String:
	var result = ""

	result += "[b]编号 (ID):[/b] %s\n" % data.get("id", "Unknown")
	result += "[b]类型 (Type):[/b] %s\n" % _get_card_type_name(data.get("card_type", "Unknown"))

	match data.get("card_type"):
		"Character":
			result += "[b]属性 (Property):[/b] %s\n" % _get_property_name(data.get("property", "-"))
			result += "[b]领域 (Category):[/b] %s\n" % _get_category_name(data.get("category", "-"))
			result += "[b]费用 (Cost):[/b] %d\n" % data.get("cost", 0)
			result += "[b]攻击力 (Attack):[/b] %d\n" % data.get("attack", 0)
		"Item":
			result += "[b]种类 (Kind):[/b] %s\n" % _get_item_kind_name(data.get("item_kind", "-"))
			result += "[b]属性 (Property):[/b] %s\n" % _get_property_name(data.get("property", "-"))
			result += "[b]领域 (Category):[/b] %s\n" % _get_category_name(data.get("category", "-"))
			result += "[b]费用 (Cost):[/b] %d\n" % data.get("cost", 0)
		"Strategy":
			result += "[b]种类 (Kind):[/b] %s\n" % _get_strategy_kind_name(data.get("strategy_kind", "-"))
			result += "[b]属性 (Property):[/b] %s\n" % _get_property_name(data.get("property", "-"))
			result += "[b]领域 (Category):[/b] %s\n" % _get_category_name(data.get("category", "-"))
			result += "[b]费用 (Cost):[/b] %d\n" % data.get("cost", 0)
		"Legend":
			result += "[b]属性 (Property):[/b] %s\n" % _get_property_name(data.get("property", "-"))
			result += "[b]费用 (Cost):[/b] %d\n" % data.get("cost", 0)

	var effects = data.get("effects", {})
	if not effects.is_empty():
		result += "\n[b]效果 (Effects):[/b]"
		# 调用外部工具转换效果为中文
		var effect_texts = _get_effects_text(effects)
		for text in effect_texts:
			result += "\n• %s" % text

	return result


## 获取卡片类型中文名称
func _get_card_type_name(type: String) -> String:
	match type:
		"Character":
			return "角色 (Character)"
		"Item":
			return "道具 (Item)"
		"Strategy":
			return "策略 (Strategy)"
		"Legend":
			return "传说 (Legend)"
		_:
			return type


## 获取属性中文名称
func _get_property_name(property: String) -> String:
	match property:
		"Rational":
			return "理性 (Rational)"
		"Divine":
			return "神圣 (Divine)"
		"Spiritual":
			return "灵性 (Spiritual)"
		_:
			return property


## 获取领域中文名称
func _get_category_name(category: String) -> String:
	match category:
		"Math":
			return "数学 (Math)"
		"Science":
			return "科学 (Science)"
		"Literature":
			return "文学 (Literature)"
		"Philosophy":
			return "哲学 (Philosophy)"
		"Mystery":
			return "神秘 (Mystery)"
		_:
			return category


## 获取策略种类中文名称
func _get_strategy_kind_name(kind: String) -> String:
	match kind:
		"Normal":
			return "普通 (Normal)"
		"Trick":
			return "计谋 (Trick)"
		"Instant":
			return "瞬间 (Instant)"
		_:
			return kind


## 获取道具种类中文名称
func _get_item_kind_name(kind: String) -> String:
	match kind:
		"Normal":
			return "普通 (Normal)"
		"Persistent":
			return "持续 (Persistent)"
		_:
			return kind


## 调用外部工具获取效果文本
func _get_effects_text(effects: Dictionary) -> Array[String]:
	var result: Array[String] = []

	# 获取工具路径
	var tool_path = ProjectSettings.globalize_path("res://tools/card-effect-tool")

	# 检查工具是否存在
	if not FileAccess.file_exists("res://tools/card-effect-tool"):
		push_error("[CardPreviewPopup] Effect tool not found: " + tool_path)
		# 返回简单的效果描述作为 fallback
		for effect_id in effects:
			var effect = effects[effect_id]
			var trigger = effect.get("trigger", "未知")
			result.append(trigger)
		return result

	# 将效果字典转换为 JSON
	var json_string = JSON.stringify(effects)
	print("[CardPreviewPopup] Effects JSON input: " + json_string)

	# 写入临时文件
	var temp_path = OS.get_user_data_dir() + "/temp_effects.json"
	var file = FileAccess.open(temp_path, FileAccess.WRITE)
	if file:
		file.store_string(json_string)
		file.close()
	else:
		push_error("[CardPreviewPopup] Failed to write temp file")
		return result

	# 执行外部工具，从文件读取
	var output = []
	var exit_code = OS.execute(tool_path, [temp_path], output, true, true)

	print("[CardPreviewPopup] Tool exit code: " + str(exit_code))

	if exit_code != 0:
		push_error("[CardPreviewPopup] Effect tool failed with exit code: " + str(exit_code))
		# 返回简单的效果描述作为 fallback
		for effect_id in effects:
			var effect = effects[effect_id]
			var trigger = effect.get("trigger", "未知")
			result.append(trigger)
		return result

	# 解析输出
	print("[CardPreviewPopup] Tool output size: " + str(output.size()))
	if output.size() > 0:
		var output_text = output[0]
		print("[CardPreviewPopup] Tool output: " + output_text)
		# 按行分割
		var lines = output_text.split("\n", false)
		for line in lines:
			line = line.strip_edges()
			if not line.is_empty():
				result.append(line)

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


## 静态方法：便捷创建并显示预览弹窗
static func show_preview(parent: Node, data: Dictionary) -> CardPreviewPopup:
	var popup = preload("res://scenes/card_preview_popup.tscn").instantiate()
	parent.add_child(popup)
	popup.setup(data)
	return popup
