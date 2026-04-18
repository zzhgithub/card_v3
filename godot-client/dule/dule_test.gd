extends Control

@onready var dule_card: DuleCard = $DuleCard
@onready var slider: HSlider = $VBoxContainer/HSlider
@onready var width_label: Label = $VBoxContainer/WidthLabel
@onready var flip_button: Button = $VBoxContainer/FlipButton

var default_card_data: Dictionary = {
	"id": "S000-C-001",
	"name": "测试角色卡",
	"card_type": "Character",
	"cost": 2,
	"property": "Divine",
	"category": "Math",
	"attack": 1500,
	"effects": {}
}

func _ready() -> void:
	dule_card.setup(default_card_data)
	slider.value_changed.connect(_on_slider_changed)
	flip_button.pressed.connect(_on_flip_pressed)
	_on_slider_changed(slider.value)

func _on_slider_changed(value: float) -> void:
	var width = value
	var height = width * 210.0 / 150.0
	dule_card.custom_minimum_size = Vector2(width, height)
	dule_card.size = Vector2(width, height)
	width_label.text = "宽度: %d px" % int(width)

func _on_flip_pressed() -> void:
	dule_card.show_front = not dule_card.show_front
	flip_button.text = "显示正面" if not dule_card.show_front else "显示卡背"
