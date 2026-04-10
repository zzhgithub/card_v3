## 卡组编辑专用卡片显示
## 支持预览、+、- 三个按钮

class_name CardDisplayDeck
extends Control

signal preview_requested(card_data: Dictionary)
signal add_card(card_id: String)
signal remove_card(card_id: String)

var card_data: Dictionary = {}
var card_id: String = ""
var _is_hovered: bool = false

@onready var card_texture: TextureRect = $CardTexture
@onready var hover_overlay: ColorRect = $HoverOverlay
@onready var button_container: VBoxContainer = $ButtonContainer

func _ready():
	_update_hover_state(false)
	mouse_filter = Control.MOUSE_FILTER_STOP

	# 连接按钮信号
	$ButtonContainer/PreviewButton.pressed.connect(_on_preview_pressed)
	$ButtonContainer/AddButton.pressed.connect(_on_add_pressed)
	$ButtonContainer/RemoveButton.pressed.connect(_on_remove_pressed)

func _process(_delta):
	var mouse_pos = get_global_mouse_position()
	var rect = get_global_rect()
	var is_mouse_inside = rect.has_point(mouse_pos)

	if is_mouse_inside != _is_hovered:
		_is_hovered = is_mouse_inside
		if _is_hovered:
			_on_mouse_entered()
		else:
			_on_mouse_exited()

func setup(data: Dictionary, front_texture: Texture2D = null) -> void:
	card_data = data
	card_id = data.get("id", "Unknown")

	if card_texture == null:
		card_texture = $CardTexture
	if hover_overlay == null:
		hover_overlay = $HoverOverlay
	if button_container == null:
		button_container = $ButtonContainer

	card_texture.ignore_texture_size = true
	card_texture.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED

	if front_texture != null:
		card_texture.texture = front_texture
	else:
		card_texture.texture = _create_placeholder_texture()

func _create_placeholder_texture() -> Texture2D:
	var image = Image.create(150, 210, false, Image.FORMAT_RGBA8)
	image.fill(Color(0.2, 0.2, 0.25, 1.0))
	for x in range(150):
		image.set_pixel(x, 0, Color(0.5, 0.5, 0.5))
		image.set_pixel(x, 209, Color(0.5, 0.5, 0.5))
	for y in range(210):
		image.set_pixel(0, y, Color(0.5, 0.5, 0.5))
		image.set_pixel(149, y, Color(0.5, 0.5, 0.5))
	return ImageTexture.create_from_image(image)

func _update_hover_state(hovered: bool) -> void:
	if hover_overlay != null:
		hover_overlay.visible = hovered
	if button_container != null:
		button_container.visible = hovered

func _on_mouse_entered() -> void:
	_update_hover_state(true)

func _on_mouse_exited() -> void:
	_update_hover_state(false)

func _on_preview_pressed() -> void:
	emit_signal("preview_requested", card_data)

func _on_add_pressed() -> void:
	emit_signal("add_card", card_id)

func _on_remove_pressed() -> void:
	emit_signal("remove_card", card_id)
