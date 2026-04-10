## 通用卡片显示组件
## 可复用的卡片图片显示，支持悬停蒙层和预览按钮

class_name CardDisplay
extends Control

signal preview_requested(card_data: Dictionary)
signal card_hovered(card_data: Dictionary)
signal card_unhovered

var card_data: Dictionary = {}
var card_id: String = ""
var _is_hovered: bool = false

@onready var card_texture: TextureRect = $CardTexture
@onready var hover_overlay: ColorRect = $HoverOverlay
@onready var preview_button: Button = $PreviewButton

func _ready():
	# 初始化状态
	_update_hover_state(false)

	# 连接预览按钮信号
	preview_button.pressed.connect(_on_preview_pressed)

	# 设置鼠标过滤器
	mouse_filter = Control.MOUSE_FILTER_STOP
	preview_button.mouse_filter = Control.MOUSE_FILTER_STOP


func _process(_delta):
	# 检查鼠标是否在控件内（更可靠的悬停检测）
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


## 更新悬停状态显示
func _update_hover_state(hovered: bool) -> void:
	if hover_overlay != null:
		hover_overlay.visible = hovered
	if preview_button != null:
		preview_button.visible = hovered

	if card_texture != null:
		if hovered:
			card_texture.modulate = Color(1.1, 1.1, 1.1)
		else:
			card_texture.modulate = Color.WHITE


## 鼠标进入
func _on_mouse_entered() -> void:
	emit_signal("card_hovered", card_data)
	_update_hover_state(true)


## 鼠标离开
func _on_mouse_exited() -> void:
	emit_signal("card_unhovered", card_data)
	_update_hover_state(false)


## 预览按钮点击
func _on_preview_pressed() -> void:
	print("[CardDisplay] Preview button clicked: %s" % card_id)
	emit_signal("preview_requested", card_data)
