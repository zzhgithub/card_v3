extends Control

const DULE_CARD_SCENE = preload("res://dule/dule_card.tscn")

@onready var card_stack = $CardStack
@onready var add_button: Button = $ButtonContainer/AddButton
@onready var remove_button: Button = $ButtonContainer/RemoveButton
@onready var flip_button: Button = $ButtonContainer/FlipButton
@onready var rotate_button: Button = $ButtonContainer/RotateButton
@onready var gap_spinbox: SpinBox = $ButtonContainer/GapSpinBox

var _card_counter := 0
var _is_face_up := true
var _is_rotated := false

func _ready() -> void:
	add_button.pressed.connect(_on_add_pressed)
	remove_button.pressed.connect(_on_remove_pressed)
	flip_button.pressed.connect(_on_flip_pressed)
	rotate_button.pressed.connect(_on_rotate_pressed)
	gap_spinbox.value_changed.connect(_on_gap_changed)
	# 延迟一帧添加卡片，确保容器布局已完成
	await get_tree().process_frame
	for i in range(8):
		_on_add_pressed()


func _on_add_pressed() -> void:
	var card = DULE_CARD_SCENE.instantiate() as DuleCard
	if card == null:
		push_error("[CardStackTest] 实例化 DuleCard 失败")
		return

	_card_counter += 1
	var data = {
		"id": "TEST-%03d" % _card_counter,
		"name": "测试卡片 %d" % _card_counter,
		"card_type": "Character",
		"property": "Divine",
		"category": "Math",
		"cost": (_card_counter % 5) + 1,
		"attack": 1000 + _card_counter * 100,
		"effects": {}
	}
	var card_size = $CardManager.card_size
	card.custom_minimum_size = card_size
	card.size = card_size
	card.card_size = card_size
	card_stack.add_card(card)
	card.setup(data)


func _on_remove_pressed() -> void:
	if card_stack.get_card_count() == 0:
		return

	var card = card_stack._held_cards[card_stack._held_cards.size() - 1]
	# 强制退出交互状态，防止静态计数器泄漏
	if card.current_state != DraggableObject.DraggableState.IDLE:
		card.change_state(DraggableObject.DraggableState.IDLE)
	card_stack.remove_card(card)
	card.queue_free()


func _on_flip_pressed() -> void:
	_is_face_up = !_is_face_up
	card_stack.card_face_up = _is_face_up
	card_stack.update_card_ui()
	flip_button.text = "正面" if _is_face_up else "背面"


func _on_rotate_pressed() -> void:
	_is_rotated = !_is_rotated
	card_stack.card_rotation_offset = PI if _is_rotated else 0.0
	card_stack.update_card_ui()
	rotate_button.text = "旋转: 180°" if _is_rotated else "旋转: 0°"


func _on_gap_changed(value: float) -> void:
	card_stack.stack_gap = value
	card_stack.update_card_ui()
