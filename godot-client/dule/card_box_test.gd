extends Control

const DULE_CARD_SCENE = preload("res://dule/dule_card.tscn")
const CARD_PREVIEW_POPUP = preload("res://scenes/card_preview_popup.tscn")
const CARD_INFO_DIR = "res://scripts_json/S000"

@onready var card_box = $CardBox

var all_card_data: Array[Dictionary] = []

func _ready() -> void:
	_load_card_data()
	print("[CardBoxTest] loaded ", all_card_data.size(), " cards")
	_generate_test_cards()
	card_box.card_clicked.connect(_on_card_clicked)

func _load_card_data() -> void:
	all_card_data.clear()
	var dir = DirAccess.open(CARD_INFO_DIR)
	if dir == null:
		push_error("[CardBoxTest] 无法打开卡片目录: %s" % CARD_INFO_DIR)
		return

	dir.list_dir_begin()
	var file_name = dir.get_next()
	while file_name != "":
		if file_name.ends_with(".json"):
			var path = CARD_INFO_DIR + "/" + file_name
			var file = FileAccess.open(path, FileAccess.READ)
			if file:
				var json = JSON.new()
				if json.parse(file.get_as_text()) == OK:
					all_card_data.append(json.data)
				file.close()
		file_name = dir.get_next()
	dir.list_dir_end()

	if all_card_data.is_empty():
		for i in range(40):
			all_card_data.append({
				"id": "TEST-%03d" % i,
				"name": "测试卡片 %d" % i,
				"card_type": "Character",
				"property": "Rational",
				"category": "Math",
				"cost": (i % 5) + 1,
				"attack": 1000 + i * 100,
				"effects": {
					"effect_1": {"trigger": "测试效果 %d" % i}
				}
			})

func _generate_test_cards() -> void:
	var card_ids: Array[String] = []
	for data in all_card_data:
		card_ids.append(data.get("id", ""))

	while card_ids.size() < 40:
		for data in all_card_data:
			if card_ids.size() >= 40:
				break
			card_ids.append(data.get("id", ""))

	print("[CardBoxTest] generating ", card_ids.size(), " cards")
	var cards: Array[DuleCard] = []
	for i in range(card_ids.size()):
		var card_id = card_ids[i]
		var card_data = _find_card_data(card_id)
		if card_data.is_empty():
			print("[CardBoxTest] empty data for ", card_id)
			continue

		var card = DULE_CARD_SCENE.instantiate() as DuleCard
		if card == null:
			print("[CardBoxTest] instantiate failed")
			continue

		card.setup(card_data)
		cards.append(card)
	card_box.add_cards(cards)
	print("[CardBoxTest] done generating")

func _find_card_data(card_id: String) -> Dictionary:
	for data in all_card_data:
		if data.get("id", "") == card_id:
			return data
	return {}

func _on_card_clicked(card_data: Dictionary) -> void:
	var popup = CARD_PREVIEW_POPUP.instantiate()
	add_child(popup)
	popup.setup(card_data)
