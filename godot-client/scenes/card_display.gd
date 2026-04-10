## 通用卡片显示组件
## 可复用的卡片图片显示，支持悬停蒙层和预览按钮

class_name CardDisplay
extends Control

signal preview_requested(card_data: Dictionary)
signal card_hovered(card_data: Dictionary)
signal card_unhovered

var card_data: Dictionary = {}
var card_id: String = ""

@onready var card_texture: TextureRect = $CardTexture
@onready var hover_overlay: ColorRect = $HoverOverlay
@onready var preview_button: Button = $PreviewButton

func _ready():
	# 初始化状态
	hover_overlay.visible = false
	preview_button.visible = false

	# 连接信号
	mouse_entered.connect(_on_mouse_entered)
	mouse_exited.connect(_on_mouse_exited)
	preview_button.pressed.connect(_on_preview_pressed)

	# 确保鼠标事件能接收
	mouse_filter = Control.MOUSE_FILTER_STOP


## 设置卡片显示
## @param data: 卡片数据字典
## @param front_texture: 卡片正面纹理
func setup(data: Dictionary, front_texture: Texture2D = null) -> void:
	card_data = data
	card_id = data.get("id", "Unknown")

	# 确保节点已初始化
	if card_texture == null:
		card_texture = $CardTexture
	if hover_overlay == null:
		hover_overlay = $HoverOverlay
	if preview_button == null:
		preview_button = $PreviewButton

	# 配置纹理显示
	card_texture.ignore_texture_size = true
	card_texture.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED

	# 设置卡片图片
	if front_texture != null:
		card_texture.texture = front_texture
	else:
		card_texture.texture = _create_placeholder_texture()


## 创建占位纹理（当没有图片时）
func _create_placeholder_texture() -> Texture2D:
	var image = Image.create(150, 210, false, Image.FORMAT_RGBA8)
	image.fill(Color(0.2, 0.2, 0.25, 1.0))

	# 添加边框
	for x in range(150):
		image.set_pixel(x, 0, Color(0.5, 0.5, 0.5))
		image.set_pixel(x, 209, Color(0.5, 0.5, 0.5))
	for y in range(210):
		image.set_pixel(0, y, Color(0.5, 0.5, 0.5))
		image.set_pixel(149, y, Color(0.5, 0.5, 0.5))

	return ImageTexture.create_from_image(image)


## 鼠标进入
func _on_mouse_entered() -> void:
	emit_signal("card_hovered", card_data)

	# 显示蒙层和预览按钮
	if hover_overlay != null:
		hover_overlay.visible = true
	if preview_button != null:
		preview_button.visible = true

	# 轻微放大效果
	if card_texture != null:
		card_texture.modulate = Color(1.1, 1.1, 1.1)


## 鼠标离开
func _on_mouse_exited() -> void:
	emit_signal("card_unhovered", card_data)

	# 隐藏蒙层和预览按钮
	if hover_overlay != null:
		hover_overlay.visible = false
	if preview_button != null:
		preview_button.visible = false

	# 恢复效果
	if card_texture != null:
		card_texture.modulate = Color.WHITE


## 预览按钮点击
func _on_preview_pressed() -> void:
	emit_signal("preview_requested", card_data)
