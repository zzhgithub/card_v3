## 战场格子测试场景
## 测试在 BattleFieldSlot 中放置来自 scripts_json 的真实卡片数据

extends Control

const DULE_CARD_SCENE = preload("res://dule/dule_card.tscn")
const CARD_JSON_PATH = "res://../scripts_json/S000/S000-C-001.json"

@onready var front_slot = $SlotsContainer/FrontSlot
@onready var back_slot = $SlotsContainer/BackSlot
@onready var place_button: Button = $ButtonContainer/PlaceButton
@onready var clear_button: Button = $ButtonContainer/ClearButton
@onready var info_label: Label = $InfoLabel

var _card_data: Dictionary = {}

func _ready() -> void:
	place_button.pressed.connect(_on_place_pressed)
	clear_button.pressed.connect(_on_clear_pressed)

	# 加载真实卡片数据
	_load_card_data()

	# 延迟一帧确保容器布局完成
	await get_tree().process_frame

	if not _card_data.is_empty():
		info_label.text = "已加载: %s (%s)" % [_card_data.get("name", "?"), _card_data.get("id", "?")]
	else:
		info_label.text = "卡片数据加载失败，使用默认数据"

	# 自动放置一张卡片到前场
	_on_place_pressed()


func _load_card_data() -> void:
	# 注意：scripts_json 在项目根目录的上一级（相对于 godot-client）
	# Godot 的 res:// 映射到 godot-client 目录，所以需要用绝对路径或相对路径读取
	var path = "res://../scripts_json/S000/S000-C-001.json"
	if FileAccess.file_exists(path):
		var file = FileAccess.open(path, FileAccess.READ)
		if file:
			var json_text = file.get_as_text()
			file.close()
			var parsed = JSON.parse_string(json_text)
			if parsed is Dictionary:
				_card_data = parsed
				print("[BattleFieldSlotTest] 加载卡片数据: %s" % _card_data.get("id", "?"))
				return

	# 如果上面的路径不行，尝试另一种方式
	# 使用 OS 绝对路径
	var project_path = ProjectSettings.globalize_path("res://")
	var abs_path = project_path.path_join("../scripts_json/S000/S000-C-001.json")
	if FileAccess.file_exists(abs_path):
		var file = FileAccess.open(abs_path, FileAccess.READ)
		if file:
			var json_text = file.get_as_text()
			file.close()
			var parsed = JSON.parse_string(json_text)
			if parsed is Dictionary:
				_card_data = parsed
				print("[BattleFieldSlotTest] 加载卡片数据(绝对路径): %s" % _card_data.get("id", "?"))
				return

	push_warning("[BattleFieldSlotTest] 无法加载卡片数据，使用默认数据")
	_card_data = {
		"id": "S000-C-001",
		"name": "理性学者",
		"card_type": "Character",
		"property": "Rational",
		"category": "Math",
		"cost": 3,
		"attack": 1500,
		"tags": [],
		"effects": {}
	}


func _on_place_pressed() -> void:
	# 如果前场已有卡片，尝试放到后场
	var target_slot = front_slot
	if not target_slot._held_cards.is_empty():
		target_slot = back_slot
		if not target_slot._held_cards.is_empty():
			print("[BattleFieldSlotTest] 两个格子都已有卡片")
			return

	var card = DULE_CARD_SCENE.instantiate() as DuleCard
	if card == null:
		push_error("[BattleFieldSlotTest] 实例化 DuleCard 失败")
		return

	var card_size = $CardManager.card_size
	card.custom_minimum_size = card_size
	card.size = card_size
	card.card_size = card_size

	target_slot.add_card(card)
	card.setup(_card_data)

	var slot_name = "前场" if target_slot == front_slot else "后场"
	print("[BattleFieldSlotTest] 已放置卡片到 %s: %s" % [slot_name, _card_data.get("name", "?")])


func _on_clear_pressed() -> void:
	for slot in [front_slot, back_slot]:
		while not slot._held_cards.is_empty():
			var card = slot._held_cards[0]
			slot.remove_card(card)
			card.queue_free()
	print("[BattleFieldSlotTest] 已清空所有格子")
