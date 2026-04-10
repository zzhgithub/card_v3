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
	card_button.mouse_filter = Control.MOUSE_FILTER_STOP

	# 连接信号
	card_button.mouse_entered.connect(_on_mouse_entered)
	card_button.mouse_exited.connect(_on_mouse_exited)
	card_button.pressed.connect(_on_card_pressed)


## 设置卡片显示
## @param data: 卡片数据字典
## @param front_texture: 卡片正面纹理
func setup(data: Dictionary, front_texture: Texture2D = null) -> void:
	card_data = data
	card_id = data.get("id", "Unknown")

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


func _on_mouse_entered():
	emit_signal("card_hovered", card_data)
	card_button.modulate = Color(1.1, 1.1, 1.1)


func _on_mouse_exited():
	emit_signal("card_unhovered")
	card_button.modulate = Color.WHITE


func _on_card_pressed():
	emit_signal("card_clicked", card_data)
