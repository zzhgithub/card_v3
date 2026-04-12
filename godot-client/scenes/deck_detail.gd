## 卡组详情页
## 左右结构：左侧卡组列表(70%)，右侧卡片查询(30%)

extends Control

signal back_requested(saved: bool)
signal deck_renamed(old_name: String, new_name: String)

const DECKS_DIR = "res://desks"
const CARD_ASSET_DIR = "res://images/cards"
const CARD_INFO_DIR = "res://scripts_json/S000"
const CARD_PREVIEW_POPUP = preload("res://scenes/card_preview_popup.tscn")
const CARD_DISPLAY_SCENE = preload("res://scenes/card_display.tscn")

@onready var title_label: Label = $VBoxContainer/TitleLabel
@onready var card_count_label: Label = $VBoxContainer/CardCountLabel
@onready var rename_button: Button = $VBoxContainer/RenameButton
@onready var save_button: Button = $VBoxContainer/ButtonContainer/SaveButton
@onready var back_button: Button = $VBoxContainer/ButtonContainer/BackButton
@onready var deck_scroll: ScrollContainer = $VBoxContainer/MainHBox/DeckPanel/VBoxContainer/ScrollContainer
@onready var deck_grid: GridContainer = $VBoxContainer/MainHBox/DeckPanel/VBoxContainer/ScrollContainer/DeckGrid
@onready var catalog_scroll: ScrollContainer = $VBoxContainer/MainHBox/CatalogPanel/VBoxContainer/ScrollContainer
@onready var catalog_grid: GridContainer = $VBoxContainer/MainHBox/CatalogPanel/VBoxContainer/ScrollContainer/CatalogGrid

var deck_name: String = ""
var original_data: Dictionary = {}
var current_data: Dictionary = {}
var has_unsaved_changes: bool = false
var all_card_data: Dictionary = {}  # 缓存所有卡片数据

func _ready():
	rename_button.pressed.connect(_on_rename_pressed)
	save_button.pressed.connect(_on_save_pressed)
	back_button.pressed.connect(_on_back_pressed)

	# 预加载所有卡片数据
	_preload_all_cards()


## 预加载所有可用卡片
func _preload_all_cards() -> void:
	all_card_data.clear()
	var dir = DirAccess.open(CARD_INFO_DIR)
	if dir == null:
		push_error("[DeckDetail] 无法打开卡片目录: %s" % CARD_INFO_DIR)
		return

	dir.list_dir_begin()
	var file_name = dir.get_next()
	while file_name != "":
		if file_name.ends_with(".json"):
			var card_id = file_name.get_basename()
			var card_data = _load_card_info(card_id)
			if not card_data.is_empty():
				all_card_data[card_id] = card_data
		file_name = dir.get_next()
	dir.list_dir_end()

	print("[DeckDetail] 预加载了 %d 张卡片" % all_card_data.size())


## 从JSON加载卡片信息
func _load_card_info(card_id: String) -> Dictionary:
	var json_path = CARD_INFO_DIR + "/" + card_id + ".json"
	if not FileAccess.file_exists(json_path):
		return {}

	var file = FileAccess.open(json_path, FileAccess.READ)
	if file == null:
		return {}

	var json_string = file.get_as_text()
	file.close()

	var json = JSON.new()
	var error = json.parse(json_string)
	if error != OK:
		return {}

	return json.data


## 加载卡片图片
func _load_card_image(card_id: String) -> Texture2D:
	var image_path = CARD_ASSET_DIR + "/" + card_id + ".png"
	if ResourceLoader.exists(image_path):
		return load(image_path) as Texture2D
	return null


## 设置卡组数据
func setup(deck_name_param: String) -> void:
	deck_name = deck_name_param
	title_label.text = "卡组: %s" % deck_name

	_load_deck_data()
	_refresh_deck_grid()
	_refresh_catalog_grid()


## 加载卡组数据
func _load_deck_data() -> void:
	var file_path = DECKS_DIR + "/" + deck_name + ".json"
	var file = FileAccess.open(file_path, FileAccess.READ)

	if file:
		var json_string = file.get_as_text()
		file.close()

		var json = JSON.new()
		var error = json.parse(json_string)
		if error == OK:
			original_data = json.data.duplicate(true)
			current_data = json.data.duplicate(true)
		else:
			original_data = {"name": deck_name, "cards": []}
			current_data = {"name": deck_name, "cards": []}
	else:
		original_data = {"name": deck_name, "cards": []}
		current_data = {"name": deck_name, "cards": []}

	has_unsaved_changes = false
	_update_save_button()


## 保存卡组数据
func _save_deck_data() -> bool:
	var file_path = DECKS_DIR + "/" + deck_name + ".json"
	var file = FileAccess.open(file_path, FileAccess.WRITE)
	if file == null:
		push_error("[DeckDetail] 无法保存卡组: %s" % deck_name)
		return false

	var json_string = JSON.stringify(current_data, "  ")
	file.store_string(json_string)
	file.close()

	original_data = current_data.duplicate(true)
	has_unsaved_changes = false
	_update_save_button()

	print("[DeckDetail] 卡组已保存: %s" % deck_name)
	return true


## 检查是否有未保存的更改
func _check_unsaved_changes() -> bool:
	if current_data.is_empty() or original_data.is_empty():
		return false
	return JSON.stringify(current_data) != JSON.stringify(original_data)


## 刷新左侧卡组网格
func _refresh_deck_grid() -> void:
	# 清除现有内容
	for child in deck_grid.get_children():
		child.queue_free()

	var cards = current_data.get("cards", [])
	card_count_label.text = "卡组卡片: %d" % cards.size()

	# 创建卡组卡片显示（卡组列表：预览、+、- 三个按钮）
	for card_id in cards:
		var card_data = all_card_data.get(card_id, {"id": card_id, "name": card_id})
		var card_display = CARD_DISPLAY_SCENE.instantiate()
		card_display.custom_minimum_size = Vector2(100, 140)
		card_display.setup(card_data, null, true, false, CardDisplay.InteractionMode.DECK_MODE)

		# 连接信号（卡组列表需要预览、添加、移除三个功能）
		card_display.preview_requested.connect(_on_preview_requested)
		card_display.add_to_deck.connect(_on_add_card_to_deck)
		card_display.remove_card.connect(_on_remove_card_from_deck)

		deck_grid.add_child(card_display)


## 刷新右侧查询网格
func _refresh_catalog_grid() -> void:
	# 清除现有内容
	for child in catalog_grid.get_children():
		child.queue_free()

	# 创建所有卡片显示
	for card_id in all_card_data:
		var card_data = all_card_data[card_id]
		var card_display = CARD_DISPLAY_SCENE.instantiate()
		card_display.custom_minimum_size = Vector2(100, 140)
		card_display.setup(card_data, null, true, false, CardDisplay.InteractionMode.ADD_MODE)

		# 连接信号
		card_display.preview_requested.connect(_on_preview_requested)
		card_display.add_to_deck.connect(_on_add_card_to_deck)

		catalog_grid.add_child(card_display)


## 预览请求
func _on_preview_requested(card_data: Dictionary) -> void:
	var popup = CARD_PREVIEW_POPUP.instantiate()
	add_child(popup)
	popup.setup(card_data)


## 添加卡片到卡组
func _on_add_card_to_deck(card_id: String) -> void:
	var cards = current_data.get("cards", [])
	cards.append(card_id)
	has_unsaved_changes = true
	_update_save_button()
	_refresh_deck_grid()
	print("[DeckDetail] 添加卡片: %s" % card_id)


## 从卡组移除卡片
func _on_remove_card_from_deck(card_id: String) -> void:
	var cards = current_data.get("cards", [])
	var index = cards.find(card_id)
	if index >= 0:
		cards.remove_at(index)
		has_unsaved_changes = true
		_update_save_button()
		_refresh_deck_grid()
		print("[DeckDetail] 移除卡片: %s" % card_id)


## 更新保存按钮状态
func _update_save_button() -> void:
	if has_unsaved_changes:
		save_button.text = "保存更改"
		save_button.add_theme_color_override("font_color", Color(1.0, 0.8, 0.2))
	else:
		save_button.text = "已保存"
		save_button.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))


## 保存按钮点击
func _on_save_pressed() -> void:
	if _save_deck_data():
		card_count_label.text = "卡组卡片: %d (已保存)" % current_data.get("cards", []).size()


## 返回按钮点击
func _on_back_pressed() -> void:
	has_unsaved_changes = _check_unsaved_changes()

	if has_unsaved_changes:
		var confirm_dialog = ConfirmationDialog.new()
		confirm_dialog.title = "未保存的更改"
		confirm_dialog.dialog_text = "卡组有未保存的更改，是否保存？"
		confirm_dialog.ok_button_text = "保存并返回"
		confirm_dialog.cancel_button_text = "直接返回"
		confirm_dialog.confirmed.connect(func():
			_save_deck_data()
			emit_signal("back_requested", true)
		)
		confirm_dialog.canceled.connect(func():
			emit_signal("back_requested", false)
		)
		add_child(confirm_dialog)
		confirm_dialog.popup_centered()
	else:
		emit_signal("back_requested", true)


## 重命名按钮点击
func _on_rename_pressed() -> void:
	print("[DeckDetail] 重命名卡组: %s" % deck_name)

	var dialog = Window.new()
	dialog.title = "重命名卡组"
	dialog.size = Vector2(400, 150)
	dialog.unresizable = true

	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 20)
	margin.add_theme_constant_override("margin_top", 20)
	margin.add_theme_constant_override("margin_right", 20)
	margin.add_theme_constant_override("margin_bottom", 20)
	dialog.add_child(margin)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 15)
	margin.add_child(vbox)

	var label = Label.new()
	label.text = "请输入新名称："
	vbox.add_child(label)

	var line_edit = LineEdit.new()
	line_edit.text = deck_name
	line_edit.placeholder_text = "支持中文名称"
	line_edit.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	vbox.add_child(line_edit)

	var button_container = HBoxContainer.new()
	button_container.alignment = BoxContainer.ALIGNMENT_CENTER
	button_container.add_theme_constant_override("separation", 20)
	vbox.add_child(button_container)

	var confirm_button = Button.new()
	confirm_button.text = "确认"
	confirm_button.custom_minimum_size = Vector2(100, 35)
	button_container.add_child(confirm_button)

	var cancel_button = Button.new()
	cancel_button.text = "取消"
	cancel_button.custom_minimum_size = Vector2(100, 35)
	button_container.add_child(cancel_button)

	confirm_button.pressed.connect(func():
		var new_name = line_edit.text.strip_edges()
		if new_name.is_empty():
			label.text = "名称不能为空！"
			label.add_theme_color_override("font_color", Color(1.0, 0.3, 0.3))
			return
		if new_name == deck_name:
			dialog.queue_free()
			return
		_rename_deck(new_name)
		dialog.queue_free()
	)

	cancel_button.pressed.connect(func():
		dialog.queue_free()
	)

	line_edit.text_submitted.connect(func(_text):
		var new_name = line_edit.text.strip_edges()
		if new_name.is_empty():
			label.text = "名称不能为空！"
			label.add_theme_color_override("font_color", Color(1.0, 0.3, 0.3))
			return
		if new_name == deck_name:
			dialog.queue_free()
			return
		_rename_deck(new_name)
		dialog.queue_free()
	)

	dialog.visibility_changed.connect(func():
		if dialog.visible:
			line_edit.grab_focus()
			line_edit.select_all()
	)

	add_child(dialog)
	dialog.popup_centered()


## 重命名卡组
func _rename_deck(new_name: String) -> void:
	var old_file_path = DECKS_DIR + "/" + deck_name + ".json"
	var new_file_path = DECKS_DIR + "/" + new_name + ".json"

	if FileAccess.file_exists(new_file_path):
		push_error("[DeckDetail] 卡组 '%s' 已存在" % new_name)
		return

	current_data["name"] = new_name

	var file = FileAccess.open(new_file_path, FileAccess.WRITE)
	if file == null:
		push_error("[DeckDetail] 无法创建新文件: %s" % new_file_path)
		return

	var json_string = JSON.stringify(current_data, "  ")
	file.store_string(json_string)
	file.close()

	DirAccess.remove_absolute(old_file_path)

	var old_name = deck_name
	deck_name = new_name
	title_label.text = "卡组: %s" % deck_name

	original_data = current_data.duplicate(true)
	has_unsaved_changes = false
	_update_save_button()

	print("[DeckDetail] 卡组已重命名: %s -> %s" % [old_name, deck_name])
	emit_signal("deck_renamed", old_name, deck_name)
