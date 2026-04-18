extends Control

const CARD_DISPLAY_SCENE = preload("res://scenes/card_display.tscn")
const CARD_INFO_DIR = "res://scripts_json/S000"

@onready var slider: HSlider = $VBoxContainer/HSlider
@onready var width_label: Label = $VBoxContainer/WidthLabel
@onready var card_container: CenterContainer = $CenterContainer

var card_display: CardDisplay
var test_card_data: Dictionary = {}

func _ready() -> void:
	_load_test_card_data()
	_create_card_display()

	slider.value_changed.connect(_on_slider_changed)
	_on_slider_changed(slider.value)


func _load_test_card_data() -> void:
	var dir = DirAccess.open(CARD_INFO_DIR)
	if dir:
		dir.list_dir_begin()
		var file_name = dir.get_next()
		while file_name != "":
			if file_name.ends_with(".json"):
				var _card_id = file_name.get_basename()
				var path = CARD_INFO_DIR + "/" + file_name
				var file = FileAccess.open(path, FileAccess.READ)
				if file:
					var json = JSON.new()
					if json.parse(file.get_as_text()) == OK:
						test_card_data = json.data
					file.close()
				if not test_card_data.is_empty():
					break
			file_name = dir.get_next()
		dir.list_dir_end()

	if test_card_data.is_empty():
		test_card_data = {
			"id": "S000-C-001",
			"name": "测试卡片",
			"card_type": "Character",
			"property": "Rational",
			"category": "Math",
			"cost": 3,
			"attack": 1500,
			"effects": {
				"effect_1": {"trigger": "入场时：抽一张卡"}
			}
		}


func _create_card_display() -> void:
	card_display = CARD_DISPLAY_SCENE.instantiate()
	card_container.add_child(card_display)
	card_display.setup(test_card_data, null, true, false, CardDisplay.InteractionMode.PREVIEW_ONLY)


func _on_slider_changed(value: float) -> void:
	var width = value
	var height = width * 2100.0 / 1500.0

	if card_display != null:
		card_display.custom_minimum_size = Vector2(width, height)
		card_display.size = Vector2(width, height)

	width_label.text = "尺寸: %.0f x %.0f px" % [width, height]
