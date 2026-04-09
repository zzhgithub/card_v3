## 卡片显示组件
## 用于在图鉴中显示单个卡片，包含编号和悬停提示
class_name CardDisplay
extends Control

signal card_hovered(card_data: Dictionary)
signal card_unhovered

var card_data: Dictionary = {}
var card_id: String = ""

@onready var card_button: Button = $CardButton
@onready var card_id_label: Label = $CardIdLabel

func _ready():
	card_button.mouse_entered.connect(_on_mouse_entered)
	card_button.mouse_exited.connect(_on_mouse_exited)
	card_button.pressed.connect(_on_card_pressed)

## 初始化卡片显示
func setup(data: Dictionary) -> void:
	card_data = data
	card_id = data.get("id", "Unknown")

	# 设置卡片编号显示
	card_id_label.text = card_id

	# TODO: 加载卡片图片
	# 图片路径: res://images/cards/{card_id}.png
	var image_path = "res://images/cards/%s.png" % card_id
	if ResourceLoader.exists(image_path):
		var texture = load(image_path) as Texture2D
		if texture:
			card_button.icon = texture
			card_button.expand_icon = true
			card_button.icon_alignment = HORIZONTAL_ALIGNMENT_CENTER
	else:
		# 如果没有图片，显示卡片名称作为占位
		card_button.text = data.get("name", card_id)

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
	print("[CardDisplay] Clicked: %s - %s" % [card_id, card_data.get("name", "")])
