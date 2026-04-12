## 预览专用卡片显示
## 使用模板化渲染（大图模式 1500x2100）

class_name PreviewCardDisplay
extends Control

## 设计稿基准尺寸
const DESIGN_WIDTH := 1500.0
const DESIGN_HEIGHT := 2100.0

var card_data: Dictionary = {}
var card_id: String = ""

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

func _ready():
	pass

## 设置卡片显示（大图模板模式）
## @param data: 卡片数据字典
## @param front_texture: 卡片正面纹理（可选，如果不提供则使用模板渲染）
func setup(data: Dictionary, front_texture: Texture2D = null) -> void:
	card_data = data
	card_id = data.get("id", "Unknown")

	_ensure_nodes_initialized()

	if front_texture != null:
		# 如果有现成图片，直接显示
		artwork_layer.texture = front_texture
		framework_layer.visible = false
		content_layer.visible = false
	else:
		# 使用模板渲染
		_render_template()

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

## 模板渲染（大图模式，使用设计稿原始坐标）
func _render_template():
	_load_artwork()
	_load_framework()
	_render_title()
	_render_attributes()
	_render_category()
	_render_effect_area()
	_render_special_attr()
	_render_card_id()

func _load_artwork():
	var image_path = "res://images/cards/%s.png" % card_id
	if ResourceLoader.exists(image_path):
		artwork_layer.texture = load(image_path)
	else:
		artwork_layer.texture = _create_placeholder_texture()

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

func _render_title():
	var name = card_data.get("name", "未知")
	title_label.text = name
	title_label.add_theme_font_size_override("font_size", 80)

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
		_:
			prop_path = ""

	if prop_path != "" and ResourceLoader.exists(prop_path):
		property_icon.texture = load(prop_path)

	# 设置图标尺寸（原始尺寸 80px）
	cost_icon.custom_minimum_size = Vector2(80, 80)
	property_icon.custom_minimum_size = Vector2(80, 80)

func _render_category():
	var category = card_data.get("category", "")
	category_label.text = CATEGORY_NAMES.get(category, category)
	category_label.add_theme_font_size_override("font_size", 60)

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
		fields_label.add_theme_font_size_override("font_size", 60)
		fields_label.add_theme_color_override("font_color", Color.WHITE)
		effect_container.add_child(fields_label)

		# 添加分隔线
		var line = ColorRect.new()
		line.custom_minimum_size = Vector2(1180, 2)
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
			effect_label.add_theme_font_size_override("font_size", 60)
			effect_label.add_theme_color_override("font_color", Color.WHITE)
			effect_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			effect_container.add_child(effect_label)

	# 智能排版：如果内容超出，缩小字体
	_auto_scale_container_fonts(effect_container, 500)

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
	special_attr_label.add_theme_font_size_override("font_size", 60)

func _render_card_id():
	card_id_label.text = card_id
	card_id_label.add_theme_font_size_override("font_size", 32)

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

func _get_effects_text(effects: Dictionary) -> Array[String]:
	var result: Array[String] = []

	var tool_path = ProjectSettings.globalize_path("res://tools/card-effect-tool")

	if not FileAccess.file_exists("res://tools/card-effect-tool"):
		for effect_id in effects:
			var effect = effects[effect_id]
			var trigger = effect.get("trigger", "未知")
			result.append(trigger)
		return result

	var json_string = JSON.stringify(effects)
	var temp_path = OS.get_user_data_dir() + "/temp_effects.json"
	var file = FileAccess.open(temp_path, FileAccess.WRITE)
	if file:
		file.store_string(json_string)
		file.close()
	else:
		return result

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
		for effect_id in effects:
			var effect = effects[effect_id]
			var trigger = effect.get("trigger", "未知")
			result.append(trigger)

	return result

func _create_placeholder_texture() -> Texture2D:
	var image = Image.create(1500, 2100, false, Image.FORMAT_RGBA8)
	image.fill(Color(0.2, 0.2, 0.25, 1.0))

	for x in range(1500):
		image.set_pixel(x, 0, Color(0.5, 0.5, 0.5))
		image.set_pixel(x, 2099, Color(0.5, 0.5, 0.5))
	for y in range(2100):
		image.set_pixel(0, y, Color(0.5, 0.5, 0.5))
		image.set_pixel(1499, y, Color(0.5, 0.5, 0.5))

	return ImageTexture.create_from_image(image)
