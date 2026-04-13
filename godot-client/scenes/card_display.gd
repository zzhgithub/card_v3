## 通用卡片显示组件
## 支持模板化渲染（框架+文本）和图片渲染两种模式
## 可根据目标尺寸自动切换小图(150x210)和大图(1500x2100)模式

class_name CardDisplay
extends Control

signal preview_requested(card_data: Dictionary)
signal card_hovered(card_data: Dictionary)
signal card_unhovered
signal add_card(card_id: String)
signal remove_card(card_id: String)
signal add_to_deck(card_id: String)

## 显示模式
enum DisplayMode {
	TEMPLATE,  ## 模板化渲染（框架+文本）
	IMAGE      ## 图片模式（使用现成图片）
}

## 交互模式
enum InteractionMode {
	PREVIEW_ONLY,   ## 仅预览按钮
	ADD_MODE,       ## 添加按钮（+）
	REMOVE_MODE,    ## 移除按钮（-）
	DECK_MODE       ## 卡组模式（预览、+、- 三个按钮）
}

## 设计稿基准尺寸
const DESIGN_WIDTH := 1500.0
const DESIGN_HEIGHT := 2100.0

## 小图模式尺寸
const SMALL_WIDTH := 150.0
const SMALL_HEIGHT := 210.0

var card_data: Dictionary = {}
var card_id: String = ""
var _is_hovered: bool = false
var display_mode: DisplayMode = DisplayMode.TEMPLATE
var interaction_mode: InteractionMode = InteractionMode.PREVIEW_ONLY
var _add_disabled: bool = false  ## +按钮是否被禁用（用于卡组数量限制）

## 渲染目标尺寸（动态设置）
var target_width: float = SMALL_WIDTH
var target_height: float = SMALL_HEIGHT

## 缩放比例
var scale_factor: float = 1.0

## 中文翻译映射
const CATEGORY_NAMES := {
	"Math": "数学",
	"Science": "科学",
	"Literature": "文学",
	"Philosophy": "哲学",
	"Mystery": "神秘"
}

const STRATEGY_KIND_NAMES := {
	"Normal": "普通",
	"Trick": "诡计",
	"Instant": "瞬时"
}

const ITEM_KIND_NAMES := {
	"Normal": "普通",
	"Persistent": "存留"
}

@onready var artwork_layer: TextureRect = $ArtworkLayer
@onready var framework_layer: TextureRect = $FrameworkLayer
@onready var content_layer: Control = $ContentLayer

@onready var title_label: Label = $ContentLayer/TitleLabel
@onready var attribute_container: HBoxContainer = $ContentLayer/AttributeContainer
@onready var cost_icon: TextureRect = $ContentLayer/AttributeContainer/CostIcon
@onready var property_icon: TextureRect = $ContentLayer/AttributeContainer/PropertyIcon
@onready var category_label: Label = $ContentLayer/CategoryLabel
@onready var effect_container: VBoxContainer = $ContentLayer/EffectContainer
@onready var special_attr_label: Label = $ContentLayer/SpecialAttrLabel
@onready var card_id_label: Label = $ContentLayer/CardIdLabel

@onready var hover_overlay: ColorRect = $HoverOverlay
@onready var preview_button: Button = $PreviewButton
@onready var add_button: Button = $AddButton
@onready var remove_button: Button = $RemoveButton
@onready var click_overlay: Control = $ClickOverlay

func _ready():
	_update_hover_state(false)
	preview_button.pressed.connect(_on_preview_pressed)
	add_button.pressed.connect(_on_add_pressed)
	remove_button.pressed.connect(_on_remove_pressed)
	mouse_filter = Control.MOUSE_FILTER_STOP
	preview_button.mouse_filter = Control.MOUSE_FILTER_STOP
	add_button.mouse_filter = Control.MOUSE_FILTER_STOP
	remove_button.mouse_filter = Control.MOUSE_FILTER_STOP

func _process(_delta):
	# 确保尺寸一致（防止GridContainer拉伸导致蒙层不匹配）
	if size != custom_minimum_size:
		size = custom_minimum_size

	var mouse_pos = get_global_mouse_position()
	var rect = get_global_rect()
	var is_mouse_inside = rect.has_point(mouse_pos)

	if is_mouse_inside != _is_hovered:
		_is_hovered = is_mouse_inside
		if _is_hovered:
			_on_mouse_entered()
		else:
			_on_mouse_exited()

## 设置卡片显示
## @param data: 卡片数据字典
## @param front_texture: 卡片正面纹理（图片模式使用）
## @param use_template: 是否使用模板渲染
## @param large_mode: 是否使用大图模式（预览用）
## @param mode: 交互模式（决定显示哪些按钮）
func setup(data: Dictionary, front_texture: Texture2D = null, use_template: bool = true, large_mode: bool = false, mode: InteractionMode = InteractionMode.PREVIEW_ONLY) -> void:
	card_data = data
	card_id = data.get("id", "Unknown")
	interaction_mode = mode

	# 设置尺寸模式
	if large_mode:
		target_width = DESIGN_WIDTH
		target_height = DESIGN_HEIGHT
	else:
		target_width = SMALL_WIDTH
		target_height = SMALL_HEIGHT

	# 计算缩放比例
	scale_factor = target_width / DESIGN_WIDTH

	# 设置控件尺寸（确保size和custom_minimum_size一致）
	custom_minimum_size = Vector2(target_width, target_height)
	size = Vector2(target_width, target_height)

	# 强制立即更新布局以确保子节点正确填充
	queue_redraw()

	# 初始化节点引用
	_ensure_nodes_initialized()

	# 选择显示模式
	if use_template and front_texture == null:
		display_mode = DisplayMode.TEMPLATE
		_render_template_mode()
	else:
		display_mode = DisplayMode.IMAGE
		_render_image_mode(front_texture)

func _ensure_nodes_initialized():
	if artwork_layer == null:
		artwork_layer = $ArtworkLayer
	if framework_layer == null:
		framework_layer = $FrameworkLayer
	if content_layer == null:
		content_layer = $ContentLayer
	if title_label == null:
		title_label = $ContentLayer/TitleLabel
	if attribute_container == null:
		attribute_container = $ContentLayer/AttributeContainer
	if cost_icon == null:
		cost_icon = $ContentLayer/AttributeContainer/CostIcon
	if property_icon == null:
		property_icon = $ContentLayer/AttributeContainer/PropertyIcon
	if category_label == null:
		category_label = $ContentLayer/CategoryLabel
	if effect_container == null:
		effect_container = $ContentLayer/EffectContainer
	if special_attr_label == null:
		special_attr_label = $ContentLayer/SpecialAttrLabel
	if card_id_label == null:
		card_id_label = $ContentLayer/CardIdLabel
	if hover_overlay == null:
		hover_overlay = $HoverOverlay
	if preview_button == null:
		preview_button = $PreviewButton
	if add_button == null:
		add_button = $AddButton
	if remove_button == null:
		remove_button = $RemoveButton
	if click_overlay == null:
		click_overlay = $ClickOverlay

## 模板渲染模式
func _render_template_mode():
	# 加载并显示卡图（底层）
	_load_artwork()

	# 加载框架
	_load_framework()

	# 渲染文本内容
	_render_title()
	_render_attributes()
	_render_category()
	_render_effect_area()
	_render_special_attr()
	_render_card_id()

	# 显示模板层，隐藏图片层
	artwork_layer.visible = true
	framework_layer.visible = true
	content_layer.visible = true

## 图片渲染模式
func _render_image_mode(front_texture: Texture2D):
	artwork_layer.visible = true
	framework_layer.visible = false
	content_layer.visible = false

	artwork_layer.ignore_texture_size = true
	artwork_layer.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED

	if front_texture != null:
		artwork_layer.texture = front_texture
	else:
		artwork_layer.texture = _create_placeholder_texture()

## 加载卡图（优先png，其次jpg）
func _load_artwork():
	var png_path = "res://images/cards/%s.png" % card_id
	var jpg_path = "res://images/cards/%s.jpg" % card_id

	if ResourceLoader.exists(png_path):
		artwork_layer.texture = load(png_path)
	elif ResourceLoader.exists(jpg_path):
		artwork_layer.texture = load(jpg_path)
	else:
		artwork_layer.texture = _create_placeholder_texture()

## 加载框架
func _load_framework():
	var card_type = card_data.get("card_type", "Character")
	var frame_path: String

	match card_type:
		"Character":
			frame_path = "res://images/framework/frame-c.png"
		"Strategy":
			frame_path = "res://images/framework/frame-s.png"
		"Item":
			frame_path = "res://images/framework/frame-i.png"
		_:
			frame_path = "res://images/framework/frame-c.png"

	if ResourceLoader.exists(frame_path):
		framework_layer.texture = load(frame_path)

## 渲染标题
func _render_title():
	var name = card_data.get("name", "未知")
	title_label.text = name
	title_label.add_theme_font_size_override("font_size", int(80 * scale_factor))

	# 设置位置和尺寸（向上移动修正偏低问题）
	title_label.position = Vector2(125 * scale_factor, 60 * scale_factor)
	title_label.size = Vector2(1250 * scale_factor, 80 * scale_factor)

## 渲染属性（费用+图标）
func _render_attributes():
	var cost = card_data.get("cost", 0)
	var property = card_data.get("property", "")

	# 加载费用图标
	var cost_path = "res://images/framework/costs/%d.svg" % cost
	if ResourceLoader.exists(cost_path):
		cost_icon.texture = load(cost_path)

	# 加载属性图标
	var prop_path: String
	match property:
		"Divine":
			prop_path = "res://images/framework/attribute/god.png"
		"Rational":
			prop_path = "res://images/framework/attribute/lx.png"
		"Spiritual":
			prop_path = "res://images/framework/attribute/ling.png"
		_:
			prop_path = ""

	if prop_path != "" and ResourceLoader.exists(prop_path):
		property_icon.texture = load(prop_path)

	# 设置容器位置和尺寸
	attribute_container.position = Vector2(125 * scale_factor, 90 * scale_factor)
	attribute_container.size = Vector2(1250 * scale_factor, 80 * scale_factor)

	# 设置图标尺寸（使用固定最小尺寸保持清晰度）
	var icon_size = max(16, int(80 * scale_factor))
	cost_icon.custom_minimum_size = Vector2(icon_size, icon_size)
	property_icon.custom_minimum_size = Vector2(icon_size, icon_size)
	# 使用整数缩放保持清晰度
	cost_icon.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	property_icon.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST

## 渲染范畴
func _render_category():
	var category = card_data.get("category", "")
	var category_cn = CATEGORY_NAMES.get(category, category)
	category_label.text = category_cn
	category_label.add_theme_font_size_override("font_size", int(60 * scale_factor))

	category_label.position = Vector2(125 * scale_factor, 1887 * scale_factor)
	category_label.size = Vector2(1250 * scale_factor, 70 * scale_factor)

## 渲染效果区域
func _render_effect_area():
	# 清除旧内容
	for child in effect_container.get_children():
		child.queue_free()

	var card_type = card_data.get("card_type", "")
	var fields_text = ""

	# 第一行：字段信息
	match card_type:
		"Character":
			var prop = card_data.get("property", "")
			var cat = card_data.get("category", "")
			fields_text = "%s | %s" % [prop, CATEGORY_NAMES.get(cat, cat)]
		"Strategy":
			var kind = card_data.get("strategy_kind", "")
			var prop = card_data.get("property", "")
			fields_text = "%s | %s" % [STRATEGY_KIND_NAMES.get(kind, kind), prop]
		"Item":
			var kind = card_data.get("item_kind", "")
			var prop = card_data.get("property", "")
			fields_text = "%s | %s" % [ITEM_KIND_NAMES.get(kind, kind), prop]

	if fields_text != "":
		var fields_label = Label.new()
		fields_label.text = fields_text
		fields_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		fields_label.add_theme_font_size_override("font_size", int(60 * scale_factor))
		fields_label.add_theme_color_override("font_color", Color.WHITE)
		effect_container.add_child(fields_label)

		# 添加分隔线
		var line = ColorRect.new()
		line.custom_minimum_size = Vector2(1180 * scale_factor, 2)
		line.color = Color(1, 1, 1, 0.5)
		effect_container.add_child(line)

	# 效果文本
	var effects = card_data.get("effects", {})
	if not effects.is_empty():
		var effect_texts = _get_effects_text(effects)
		for text in effect_texts:
			var effect_label = Label.new()
			effect_label.text = "• " + text
			effect_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT
			effect_label.add_theme_font_size_override("font_size", int(60 * scale_factor))
			effect_label.add_theme_color_override("font_color", Color.WHITE)
			effect_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			effect_container.add_child(effect_label)

	# 设置容器位置和尺寸
	effect_container.position = Vector2(160 * scale_factor, 1300 * scale_factor)
	effect_container.size = Vector2(1180 * scale_factor, 500 * scale_factor)

	# 智能排版：如果内容超出，缩小字体
	_auto_scale_container_fonts(effect_container, 500 * scale_factor)

## 渲染特殊属性
func _render_special_attr():
	var card_type = card_data.get("card_type", "")
	var text = ""

	match card_type:
		"Character":
			var attack = card_data.get("attack", 0)
			text = "%d ATK" % attack
		"Strategy":
			var kind = card_data.get("strategy_kind", "")
			text = STRATEGY_KIND_NAMES.get(kind, kind)
		"Item":
			var kind = card_data.get("item_kind", "")
			text = ITEM_KIND_NAMES.get(kind, kind)

	special_attr_label.text = text
	special_attr_label.add_theme_font_size_override("font_size", int(60 * scale_factor))

	special_attr_label.position = Vector2(115 * scale_factor, 1885 * scale_factor)
	special_attr_label.size = Vector2(165 * scale_factor, 70 * scale_factor)

## 渲染卡片编号
func _render_card_id():
	card_id_label.text = card_id
	card_id_label.add_theme_font_size_override("font_size", int(32 * scale_factor))

	card_id_label.position = Vector2(110 * scale_factor, 2010 * scale_factor)
	card_id_label.size = Vector2(640 * scale_factor, 80 * scale_factor)

## 字体自动缩放
func _auto_scale_container_fonts(container: VBoxContainer, max_height: float):
	var total_height = 0.0
	for child in container.get_children():
		if child is Label:
			total_height += child.get_theme_font_size("font_size") + 5

	if total_height > max_height:
		var scale_ratio = max_height / total_height
		for child in container.get_children():
			if child is Label:
				var new_size = int(child.get_theme_font_size("font_size") * scale_ratio)
				child.add_theme_font_size_override("font_size", new_size)

## 获取效果文本
func _get_effects_text(effects: Dictionary) -> Array[String]:
	var result: Array[String] = []

	# 获取工具路径
	var tool_path = ProjectSettings.globalize_path("res://tools/card-effect-tool")

	if not FileAccess.file_exists("res://tools/card-effect-tool"):
		# Fallback: 返回简单的触发器名称
		for effect_id in effects:
			var effect = effects[effect_id]
			var trigger = effect.get("trigger", "未知")
			result.append(trigger)
		return result

	# 将效果字典转换为 JSON
	var json_string = JSON.stringify(effects)

	# 写入临时文件
	var temp_path = OS.get_user_data_dir() + "/temp_effects.json"
	var file = FileAccess.open(temp_path, FileAccess.WRITE)
	if file:
		file.store_string(json_string)
		file.close()
	else:
		return result

	# 执行外部工具
	var output = []
	var exit_code = OS.execute(tool_path, [temp_path], output, true, true)

	if exit_code == 0 and output.size() > 0:
		var output_text = output[0]
		var lines = output_text.split("\n", false)
		for line in lines:
			line = line.strip_edges()
			if not line.is_empty():
				result.append(line)
	else:
		# Fallback
		for effect_id in effects:
			var effect = effects[effect_id]
			var trigger = effect.get("trigger", "未知")
			result.append(trigger)

	return result

## 创建占位纹理
func _create_placeholder_texture() -> Texture2D:
	var img_width = int(target_width)
	var img_height = int(target_height)
	var image = Image.create(img_width, img_height, false, Image.FORMAT_RGBA8)
	image.fill(Color(0.2, 0.2, 0.25, 1.0))

	# 添加边框
	for x in range(img_width):
		image.set_pixel(x, 0, Color(0.5, 0.5, 0.5))
		image.set_pixel(x, img_height - 1, Color(0.5, 0.5, 0.5))
	for y in range(img_height):
		image.set_pixel(0, y, Color(0.5, 0.5, 0.5))
		image.set_pixel(img_width - 1, y, Color(0.5, 0.5, 0.5))

	return ImageTexture.create_from_image(image)

## 更新悬停状态
func _update_hover_state(hovered: bool) -> void:
	if hover_overlay != null:
		hover_overlay.visible = hovered

	# 根据交互模式显示对应的按钮并设置位置
	match interaction_mode:
		InteractionMode.PREVIEW_ONLY:
			# 只显示预览按钮（居中）
			if preview_button != null:
				preview_button.visible = hovered
				preview_button.position = Vector2(size.x / 2 - 40, size.y / 2 - 20)
			if add_button != null:
				add_button.visible = false
			if remove_button != null:
				remove_button.visible = false

		InteractionMode.ADD_MODE:
			# 搜索列表：预览和+按钮（竖直排列）
			if preview_button != null:
				preview_button.visible = hovered
				preview_button.position = Vector2(size.x / 2 - 40, size.y / 2 - 20)
			if add_button != null:
				add_button.visible = hovered and not _add_disabled
				add_button.position = Vector2(size.x / 2 - 30, size.y / 2 + 30)
			if remove_button != null:
				remove_button.visible = false

		InteractionMode.REMOVE_MODE:
			# 预览和-按钮（竖直排列）
			if preview_button != null:
				preview_button.visible = hovered
				preview_button.position = Vector2(size.x / 2 - 40, size.y / 2 - 40)
			if add_button != null:
				add_button.visible = false
			if remove_button != null:
				remove_button.visible = hovered
				remove_button.position = Vector2(size.x / 2 - 30, size.y / 2 + 10)

		InteractionMode.DECK_MODE:
			# 卡组列表：预览、+、- 三个按钮（竖直排列）
			if preview_button != null:
				preview_button.visible = hovered
				preview_button.position = Vector2(size.x / 2 - 40, size.y / 2 - 60)
			if add_button != null:
				add_button.visible = hovered and not _add_disabled
				add_button.position = Vector2(size.x / 2 - 30, size.y / 2)
			if remove_button != null:
				remove_button.visible = hovered
				remove_button.position = Vector2(size.x / 2 - 30, size.y / 2 + 40)

	if artwork_layer != null:
		if hovered:
			artwork_layer.modulate = Color(1.1, 1.1, 1.1)
		else:
			artwork_layer.modulate = Color.WHITE

## 鼠标事件处理
func _on_mouse_entered() -> void:
	emit_signal("card_hovered", card_data)
	_update_hover_state(true)

func _on_mouse_exited() -> void:
	emit_signal("card_unhovered", card_data)
	_update_hover_state(false)

func _on_preview_pressed() -> void:
	print("[CardDisplay] Preview button clicked: %s" % card_id)
	emit_signal("preview_requested", card_data)

func _on_add_pressed() -> void:
	print("[CardDisplay] Add button clicked: %s" % card_id)
	emit_signal("add_card", card_id)
	emit_signal("add_to_deck", card_id)

func _on_remove_pressed() -> void:
	print("[CardDisplay] Remove button clicked: %s" % card_id)
	emit_signal("remove_card", card_id)

## 设置+按钮是否禁用（用于卡组数量限制）
func set_add_disabled(disabled: bool) -> void:
	_add_disabled = disabled
