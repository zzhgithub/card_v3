## 预览专用卡片显示
## 简单的卡片图片显示，无悬停效果，用于预览弹窗

class_name PreviewCardDisplay
extends Control

var card_data: Dictionary = {}
var card_id: String = ""

@onready var card_texture: TextureRect = $CardTexture

func _ready():
	pass


## 设置卡片显示
## @param data: 卡片数据字典
## @param front_texture: 卡片正面纹理
func setup(data: Dictionary, front_texture: Texture2D = null) -> void:
	card_data = data
	card_id = data.get("id", "Unknown")

	# 确保节点已初始化
	if card_texture == null:
		card_texture = $CardTexture

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
