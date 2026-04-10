## 图鉴卡片显示组件
## 用于在图鉴中显示单个卡片，配合 CatalogCardFactory 使用

class_name CatalogCardDisplay
extends Control

signal card_hovered(card_data: Dictionary)
signal card_unhovered

var card_data: Dictionary = {}
var card_id: String = ""

@onready var card_button: TextureButton = $CardButton
@onready var card_id_label: Label = $CardIdLabel

func _ready():
	# 确保 TextureButton 可以接收鼠标事件
	card_button.mouse_filter = Control.MOUSE_FILTER_STOP

	# 连接信号
	card_button.mouse_entered.connect(_on_mouse_entered)
	card_button.mouse_exited.connect(_on_mouse_exited)
	card_button.pressed.connect(_on_card_pressed)

	print("[CatalogCardDisplay] Ready: %s" % card_id)


## 设置卡片显示
## @param data: 卡片数据字典
## @param front_texture: 卡片正面纹理
## @param back_texture: 卡片背面纹理（图鉴中通常不使用）
func setup(data: Dictionary, front_texture: Texture2D = null, back_texture: Texture2D = null) -> void:
	card_data = data
	card_id = data.get("id", "Unknown")

	# 设置卡片编号显示
	card_id_label.text = card_id

	# 配置 TextureButton 缩放模式
	# 忽略纹理原始大小，使用按钮设定的大小
	card_button.ignore_texture_size = true
	# 保持宽高比并居中缩放
	card_button.stretch_mode = TextureButton.STRETCH_KEEP_ASPECT_CENTERED

	# 设置卡片图片
	if front_texture != null:
		card_button.texture_normal = front_texture
	else:
		# 使用默认占位显示
		card_button.texture_normal = _create_placeholder_texture()


## 创建占位纹理（当没有图片时）
func _create_placeholder_texture() -> Texture2D:
	# 创建占位图片
	var image = Image.create(150, 210, false, Image.FORMAT_RGBA8)
	image.fill(Color(0.2, 0.2, 0.25, 1.0))

	# 添加边框
	for x in range(150):
		image.set_pixel(x, 0, Color(0.5, 0.5, 0.5))
		image.set_pixel(x, 209, Color(0.5, 0.5, 0.5))
	for y in range(210):
		image.set_pixel(0, y, Color(0.5, 0.5, 0.5))
		image.set_pixel(149, y, Color(0.5, 0.5, 0.5))

	var texture = ImageTexture.create_from_image(image)
	return texture


func _on_mouse_entered():
	print("[CatalogCardDisplay] Mouse entered: %s" % card_id)
	emit_signal("card_hovered", card_data)
	# 悬停视觉效果
	card_button.modulate = Color(1.1, 1.1, 1.1)
	card_id_label.modulate = Color(1.2, 1.2, 1.2)


func _on_mouse_exited():
	emit_signal("card_unhovered")
	# 恢复视觉效果
	card_button.modulate = Color.WHITE
	card_id_label.modulate = Color.WHITE


func _on_card_pressed():
	print("[CatalogCardDisplay] Clicked: %s - %s" % [card_id, card_data.get("name", "")])
