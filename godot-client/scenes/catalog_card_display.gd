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
	card_button.mouse_entered.connect(_on_mouse_entered)
	card_button.mouse_exited.connect(_on_mouse_exited)
	card_button.pressed.connect(_on_card_pressed)


## 设置卡片显示
## @param data: 卡片数据字典
## @param front_texture: 卡片正面纹理
## @param back_texture: 卡片背面纹理（图鉴中通常不使用）
func setup(data: Dictionary, front_texture: Texture2D = null, back_texture: Texture2D = null) -> void:
	card_data = data
	card_id = data.get("id", "Unknown")

	# 设置卡片编号显示
	card_id_label.text = card_id

	# 设置卡片图片
	if front_texture != null:
		card_button.texture_normal = front_texture
	else:
		# 使用默认占位显示
		card_button.texture_normal = _create_placeholder_texture()


## 创建占位纹理（当没有图片时）
func _create_placeholder_texture() -> Texture2D:
	var viewport = SubViewport.new()
	viewport.size = Vector2(150, 210)
	viewport.transparent_bg = true

	var bg = ColorRect.new()
	bg.color = Color(0.2, 0.2, 0.25, 1.0)
	bg.size = viewport.size
	viewport.add_child(bg)

	var label = Label.new()
	label.text = card_data.get("name", card_id)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	label.size = viewport.size
	label.autowrap_mode = TextServer.AUTOWRAP_WORD
	viewport.add_child(label)

	# 注意：实际项目中应该使用预加载的占位图
	# 这里简化处理，返回 null 让调用者处理
	return null


func _on_mouse_entered():
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
