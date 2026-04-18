extends Control

const CARD_DISPLAY_SCENE = preload("res://scenes/card_display.tscn")
const CARD_INFO_DIR = "res://scripts_json/S000"

@onready var deck_grid: GridContainer = $HBoxContainer/LeftPanel/ScrollContainer/DeckGrid
@onready var catalog_grid: GridContainer = $HBoxContainer/RightPanel/ScrollContainer/CatalogGrid

var all_card_data: Dictionary = {}
var deck_card_ids: Array[String] = []

var _deck_resize_pending: bool = false
var _catalog_resize_pending: bool = false

func _ready() -> void:
	_preload_all_cards()
	_generate_test_deck()

	deck_grid.resized.connect(_on_deck_grid_resized)
	catalog_grid.resized.connect(_on_catalog_grid_resized)

	call_deferred("_refresh_all")


func _preload_all_cards() -> void:
	all_card_data.clear()
	var dir = DirAccess.open(CARD_INFO_DIR)
	if dir == null:
		push_error("[DeckDetailTest] 无法打开卡片目录: %s" % CARD_INFO_DIR)
		return

	dir.list_dir_begin()
	var file_name = dir.get_next()
	while file_name != "":
		if file_name.ends_with(".json"):
			var card_id = file_name.get_basename()
			var path = CARD_INFO_DIR + "/" + file_name
			var file = FileAccess.open(path, FileAccess.READ)
			if file:
				var json = JSON.new()
				if json.parse(file.get_as_text()) == OK:
					all_card_data[card_id] = json.data
				file.close()
		file_name = dir.get_next()
	dir.list_dir_end()


func _generate_test_deck() -> void:
	deck_card_ids.clear()
	var ids: Array = all_card_data.keys()
	ids.shuffle()
	for i in range(min(40, ids.size())):
		deck_card_ids.append(ids[i])


func _refresh_all() -> void:
	_refresh_deck_grid()
	_refresh_catalog_grid()


func _refresh_deck_grid() -> void:
	for child in deck_grid.get_children():
		child.queue_free()

	var card_size = _calculate_card_size(deck_grid, 10)
	if card_size.x <= 0 or card_size.y <= 0:
		return

	for card_id in deck_card_ids:
		var card_data = all_card_data.get(card_id, {"id": card_id, "name": card_id})
		var card_display = CARD_DISPLAY_SCENE.instantiate()
		card_display.custom_minimum_size = card_size
		card_display.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
		card_display.size_flags_vertical = Control.SIZE_EXPAND_FILL
		card_display.setup(card_data, null, true, false, CardDisplay.InteractionMode.DECK_MODE)
		deck_grid.add_child(card_display)


func _refresh_catalog_grid() -> void:
	for child in catalog_grid.get_children():
		child.queue_free()

	var card_size = _calculate_card_size(catalog_grid, 3)
	if card_size.x <= 0 or card_size.y <= 0:
		return

	for card_id in all_card_data:
		var card_data = all_card_data[card_id]
		var card_display = CARD_DISPLAY_SCENE.instantiate()
		card_display.custom_minimum_size = card_size
		card_display.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
		card_display.size_flags_vertical = Control.SIZE_EXPAND_FILL
		card_display.setup(card_data, null, true, false, CardDisplay.InteractionMode.ADD_MODE)
		catalog_grid.add_child(card_display)


func _calculate_card_size(grid: GridContainer, columns: int) -> Vector2:
	if grid.size.x <= 0:
		return Vector2.ZERO

	var h_sep = grid.get_theme_constant("h_separation")
	var available_width = grid.size.x
	var cell_width = (available_width - (columns - 1) * h_sep) / columns
	var cell_height = cell_width * 2100.0 / 1500.0
	return Vector2(max(cell_width, 1.0), max(cell_height, 1.0))


func _on_deck_grid_resized() -> void:
	if _deck_resize_pending:
		return
	_deck_resize_pending = true
	call_deferred("_apply_deck_resize")


func _apply_deck_resize() -> void:
	_deck_resize_pending = false
	var card_size = _calculate_card_size(deck_grid, 10)
	if card_size.x <= 0:
		return
	for child in deck_grid.get_children():
		if child is CardDisplay:
			child.custom_minimum_size = card_size


func _on_catalog_grid_resized() -> void:
	if _catalog_resize_pending:
		return
	_catalog_resize_pending = true
	call_deferred("_apply_catalog_resize")


func _apply_catalog_resize() -> void:
	_catalog_resize_pending = false
	var card_size = _calculate_card_size(catalog_grid, 3)
	if card_size.x <= 0:
		return
	for child in catalog_grid.get_children():
		if child is CardDisplay:
			child.custom_minimum_size = card_size
