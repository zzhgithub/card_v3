extends Control

const DULE_CARD_SCENE = preload("res://dule/dule_card.tscn")
const CARD_PREVIEW_POPUP = preload("res://scenes/card_preview_popup.tscn")
const CARD_INFO_DIR = "res://scripts_json/S000"

@onready var card_box = $CardBox

var all_card_data: Array[Dictionary] = []
var _cards_with_overlay: Array[DuleCard] = []
var _card_dusted: Dictionary = {}  # DuleCard -> bool

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
		_setup_card_overlay(card)
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

# ---------------------------------------------------------------------------
# 卡片覆盖层（悬停遮罩 + 预览按钮 + 蒙尘按钮 + 蒙尘遮罩）
# ---------------------------------------------------------------------------

func _setup_card_overlay(card: DuleCard) -> void:
	# HoverOverlay：悬停时的半透明遮罩
	var hover_overlay = ColorRect.new()
	hover_overlay.name = "HoverOverlay"
	hover_overlay.color = Color(0, 0, 0, 0.5)
	hover_overlay.z_index = 10
	hover_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hover_overlay.visible = false
	hover_overlay.size = card.size
	card.add_child(hover_overlay)

	# DustOverlay：蒙尘状态的灰褐色遮罩
	var dust_overlay = ColorRect.new()
	dust_overlay.name = "DustOverlay"
	dust_overlay.color = Color(0.4, 0.35, 0.3, 0.6)
	dust_overlay.z_index = 9
	dust_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	dust_overlay.visible = false
	dust_overlay.size = card.size
	card.add_child(dust_overlay)

	# PreviewButton
	var preview_button = Button.new()
	preview_button.name = "PreviewButton"
	preview_button.text = "预览"
	preview_button.z_index = 11
	preview_button.visible = false
	card.add_child(preview_button)

	# DustButton
	var dust_button = Button.new()
	dust_button.name = "DustButton"
	dust_button.text = "蒙尘"
	dust_button.z_index = 11
	dust_button.visible = false
	card.add_child(dust_button)

	# 连接信号
	preview_button.pressed.connect(_on_preview_pressed.bind(card))
	dust_button.pressed.connect(_on_dust_pressed.bind(card))
	card.resized.connect(_on_card_resized.bind(card))

	# 初始化布局
	_update_overlay_layout(card)

	# 记录到轮询列表
	_cards_with_overlay.append(card)

func _update_overlay_layout(card: DuleCard) -> void:
	var hover_overlay = card.get_node_or_null("HoverOverlay")
	var dust_overlay = card.get_node_or_null("DustOverlay")
	var preview_button = card.get_node_or_null("PreviewButton")
	var dust_button = card.get_node_or_null("DustButton")

	if hover_overlay != null:
		hover_overlay.size = card.size
	if dust_overlay != null:
		dust_overlay.size = card.size

	if preview_button != null:
		preview_button.size = Vector2(card.size.x * 0.6, card.size.y * 0.12)
		preview_button.position = Vector2(card.size.x * 0.2, card.size.y * 0.30)

	if dust_button != null:
		dust_button.size = Vector2(card.size.x * 0.6, card.size.y * 0.12)
		dust_button.position = Vector2(card.size.x * 0.2, card.size.y * 0.50)

func _process(_delta: float) -> void:
	var mouse_pos = get_global_mouse_position()
	for card in _cards_with_overlay:
		var is_inside = card.get_global_rect().has_point(mouse_pos)
		var was_hovered = card.get_meta("_hovered", false)
		if is_inside != was_hovered:
			card.set_meta("_hovered", is_inside)
			_update_card_hover(card, is_inside)

func _update_card_hover(card: DuleCard, hovered: bool) -> void:
	var hover_overlay = card.get_node_or_null("HoverOverlay")
	var preview_button = card.get_node_or_null("PreviewButton")
	var dust_button = card.get_node_or_null("DustButton")

	if hover_overlay != null:
		hover_overlay.visible = hovered
	if preview_button != null:
		preview_button.visible = hovered
	if dust_button != null:
		dust_button.visible = hovered

func _on_card_resized(card: DuleCard) -> void:
	_update_overlay_layout(card)

func _on_preview_pressed(card: DuleCard) -> void:
	print("[CardBoxTest] Preview button clicked: %s" % card.card_id)
	CardPreviewPopup.show_preview(self, card.card_data)

func _on_dust_pressed(card: DuleCard) -> void:
	var dusted = not _card_dusted.get(card, false)
	_card_dusted[card] = dusted

	var dust_overlay = card.get_node_or_null("DustOverlay")
	var dust_button = card.get_node_or_null("DustButton")

	if dust_overlay != null:
		dust_overlay.visible = dusted
	if dust_button != null:
		dust_button.text = "去尘" if dusted else "蒙尘"

	print("[CardBoxTest] Dust toggled for %s: %s" % [card.card_id, dusted])
