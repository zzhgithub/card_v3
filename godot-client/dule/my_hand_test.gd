extends Control

const DULE_CARD_SCENE = preload("res://dule/dule_card.tscn")

@onready var my_hand = $PlayerArea/PlayerBottomRow/MyHand
@onready var add_button: Button = $ButtonContainer/AddButton
@onready var remove_button: Button = $ButtonContainer/RemoveButton

var _card_counter := 0

func _ready() -> void:
	add_button.pressed.connect(_on_add_pressed)
	remove_button.pressed.connect(_on_remove_pressed)
	# 延迟一帧添加卡片，确保容器布局已完成
	await get_tree().process_frame
	for i in range(12):
		_on_add_pressed()


func _on_add_pressed() -> void:
	if my_hand.get_card_count() >= my_hand.max_hand_size:
		return

	var card = DULE_CARD_SCENE.instantiate() as DuleCard
	if card == null:
		push_error("[MyHandTest] 实例化 DuleCard 失败")
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
	my_hand.add_card(card)
	card.setup(data)


func _on_remove_pressed() -> void:
	if my_hand.get_card_count() == 0:
		return

	var card = my_hand._held_cards[my_hand._held_cards.size() - 1]
	# 强制退出交互状态，防止静态计数器泄漏
	if card.current_state != DraggableObject.DraggableState.IDLE:
		card.change_state(DraggableObject.DraggableState.IDLE)
	my_hand.remove_card(card)
	card.queue_free()
