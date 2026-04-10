## 通用卡片显示组件
## 可复用的卡片图片显示，支持悬停效果和点击事件

class_name CardDisplay
extends Control

signal card_clicked(card_data: Dictionary)
signal card_hovered(card_data: Dictionary)
signal card_unhovered

var card_data: Dictionary = {}
var card_id: String = ""

@onready var card_button: TextureButton = $CardButton

func _ready():
	# 确保 TextureButton 可以接收鼠标事件
	if card_button != null:
		_setup_card_button()


## 设置卡片显示
## @param data: 卡片数据字典
## @param front_texture: 卡片正面纹理
func setup(data: Dictionary, front_texture: Texture2D = null) -> void:
	card_data = data
	card_id = data.get("id", "Unknown")

	# 确保 card_button 已初始化（setup可能在_ready之前调用）
	if card_button == null:
		card_button = $CardButton

	# 连接信号（在_ready之前调用时需要手动连接）
	if not card_button.mouse_entered.is_connected(_on_mouse_entered):
		card_button.mouse_entered.connect(_on_mouse_entered)
		card_button.mouse_exited.connect(_on_mouse_exited)
		card_button.pressed.connect(_on_card_pressed)

	# 配置 TextureButton 缩放模式
	card_button.ignore_texture_size = true
	card_button.stretch_mode = TextureButton.STRETCH_KEEP_ASPECT_CENTERED

	# 设置卡片图片
	if front_texture != null:
		card_button.texture_normal = front_texture
	else:
		card_button.texture_normal = _create_placeholder_texture()


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


## 初始化 CardButton 设置
func _setup_card_button() -> void:
	if card_button == null:
		return
	card_button.mouse_filter = Control.MOUSE_FILTER_STOP
	card_button.mouse_entered.connect(_on_mouse_entered)
	card_button.mouse_exited.connect(_on_mouse_exited)
	card_button.pressed.connect(_on_card_pressed)


func _on_mouse_entered():
	emit_signal("card_hovered", card_data)
	if card_button != null:
		card_button.modulate = Color(1.1, 1.1, 1.1)


func _on_mouse_exited():
	emit_signal("card_unhovered")
	if card_button != null:
		card_button.modulate = Color.WHITE


func _on_card_pressed():
	emit_signal("card_clicked", card_data)
