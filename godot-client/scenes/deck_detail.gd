## 卡组详情页
## 左右结构：左侧卡组列表(70%)，右侧卡片查询(30%)
## 使用 CardBox + DuleCard 展示卡片

extends Control

signal back_requested(saved: bool)
signal deck_renamed(old_name: String, new_name: String)

const DECKS_DIR = "res://desks"
const CARD_ASSET_DIR = "res://images/cards"
const CARD_INFO_DIR = "res://scripts_json/S000"
const CARD_PREVIEW_POPUP = preload("res://scenes/card_preview_popup.tscn")
const DULE_CARD_SCENE = preload("res://dule/dule_card.tscn")

@onready var title_label: Label = $VBoxContainer/TitleLabel
@onready var card_count_label: Label = $VBoxContainer/CardCountLabel
@onready var rename_button: Button = $VBoxContainer/RenameButton
@onready var save_button: Button = $VBoxContainer/ButtonContainer/SaveButton
@onready var back_button: Button = $VBoxContainer/ButtonContainer/BackButton
@onready var deck_card_box: CardBox = $VBoxContainer/MainHBox/DeckPanel/VBoxContainer/DeckCardBox
@onready var catalog_card_box: CardBox = $VBoxContainer/MainHBox/CatalogPanel/VBoxContainer/CatalogCardBox

var deck_name: String = ""
var original_data: Dictionary = {}
var current_data: Dictionary = {}
var has_unsaved_changes: bool = false
var all_card_data: Dictionary = {}  # 缓存所有卡片数据

func _ready():
	rename_button.pressed.connect(_on_rename_pressed)
	save_button.pressed.connect(_on_save_pressed)
	back_button.pressed.connect(_on_back_pressed)

	# 禁用 CardBox 的拖放区，避免找不到 CardManager 报错
	deck_card_box.enable_drop_zone = false
	catalog_card_box.enable_drop_zone = false

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


## 获取卡组中每张卡片的数量
func _get_card_counts() -> Dictionary:
	var counts = {}
	var cards = current_data.get("cards", [])
	for card_id in cards:
		counts[card_id] = counts.get(card_id, 0) + 1
	return counts


## 刷新左侧卡组 CardBox
func _refresh_deck_grid() -> void:
	deck_card_box.clear_cards()

	var cards = current_data.get("cards", [])
	card_count_label.text = "卡组卡片: %d" % cards.size()

	var card_counts = _get_card_counts()
	var dule_cards: Array[DuleCard] = []

	for card_id in cards:
		var card_data = all_card_data.get(card_id, {"id": card_id, "name": card_id})
		var card = DULE_CARD_SCENE.instantiate() as DuleCard
		if card == null:
			continue
		card.setup(card_data)
		_setup_deck_card_overlay(card, card_counts)
		dule_cards.append(card)

	deck_card_box.add_cards(dule_cards)


## 刷新右侧查询 CardBox
func _refresh_catalog_grid() -> void:
	catalog_card_box.clear_cards()

	var card_counts = _get_card_counts()
	var dule_cards: Array[DuleCard] = []

	for card_id in all_card_data:
		var card_data = all_card_data[card_id]
		var card = DULE_CARD_SCENE.instantiate() as DuleCard
		if card == null:
			continue
		card.setup(card_data)
		_setup_catalog_card_overlay(card, card_counts)
		dule_cards.append(card)

	catalog_card_box.add_cards(dule_cards)


# ---------------------------------------------------------------------------
# 动态按钮覆盖层（类似 card_box_test.gd 的方式）
# ---------------------------------------------------------------------------

func _setup_deck_card_overlay(card: DuleCard, card_counts: Dictionary) -> void:
	var card_id = card.card_id

	# HoverOverlay — 使用锚点填满整个卡片
	var hover_overlay = ColorRect.new()
	hover_overlay.name = "HoverOverlay"
	hover_overlay.color = Color(0, 0, 0, 0.5)
	hover_overlay.z_index = 10
	hover_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hover_overlay.visible = false
	hover_overlay.anchor_right = 1.0
	hover_overlay.anchor_bottom = 1.0
	hover_overlay.offset_right = 0
	hover_overlay.offset_bottom = 0
	card.add_child(hover_overlay)

	# PreviewButton — 使用锚点自适应
	var preview_button = Button.new()
	preview_button.name = "PreviewButton"
	preview_button.text = "预览"
	preview_button.z_index = 11
	preview_button.visible = false
	preview_button.anchor_left = 0.1
	preview_button.anchor_right = 0.9
	preview_button.anchor_top = 0.15
	preview_button.anchor_bottom = 0.32
	preview_button.offset_left = 0
	preview_button.offset_right = 0
	preview_button.offset_top = 0
	preview_button.offset_bottom = 0
	card.add_child(preview_button)

	# AddButton
	var add_button = Button.new()
	add_button.name = "AddButton"
	add_button.text = "+"
	add_button.z_index = 11
	add_button.visible = false
	add_button.anchor_left = 0.1
	add_button.anchor_right = 0.9
	add_button.anchor_top = 0.36
	add_button.anchor_bottom = 0.53
	add_button.offset_left = 0
	add_button.offset_right = 0
	add_button.offset_top = 0
	add_button.offset_bottom = 0
	card.add_child(add_button)

	# RemoveButton
	var remove_button = Button.new()
	remove_button.name = "RemoveButton"
	remove_button.text = "-"
	remove_button.z_index = 11
	remove_button.visible = false
	remove_button.anchor_left = 0.1
	remove_button.anchor_right = 0.9
	remove_button.anchor_top = 0.57
	remove_button.anchor_bottom = 0.74
	remove_button.offset_left = 0
	remove_button.offset_right = 0
	remove_button.offset_top = 0
	remove_button.offset_bottom = 0
	card.add_child(remove_button)

	# 3张上限禁用
	if card_counts.get(card_id, 0) >= 3:
		add_button.disabled = true

	# 信号连接
	preview_button.pressed.connect(_on_preview_pressed.bind(card))
	add_button.pressed.connect(_on_add_card_to_deck.bind(card_id))
	remove_button.pressed.connect(_on_remove_card_from_deck.bind(card_id))
	card.resized.connect(_update_deck_button_fonts.bind(card))

	# 延迟一帧后更新字体，确保 CardBox 已完成布局
	call_deferred("_update_deck_button_fonts", card)


func _setup_catalog_card_overlay(card: DuleCard, card_counts: Dictionary) -> void:
	var card_id = card.card_id

	# HoverOverlay — 使用锚点填满整个卡片
	var hover_overlay = ColorRect.new()
	hover_overlay.name = "HoverOverlay"
	hover_overlay.color = Color(0, 0, 0, 0.5)
	hover_overlay.z_index = 10
	hover_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hover_overlay.visible = false
	hover_overlay.anchor_right = 1.0
	hover_overlay.anchor_bottom = 1.0
	hover_overlay.offset_right = 0
	hover_overlay.offset_bottom = 0
	card.add_child(hover_overlay)

	# PreviewButton — 使用锚点自适应
	var preview_button = Button.new()
	preview_button.name = "PreviewButton"
	preview_button.text = "预览"
	preview_button.z_index = 11
	preview_button.visible = false
	preview_button.anchor_left = 0.15
	preview_button.anchor_right = 0.85
	preview_button.anchor_top = 0.25
	preview_button.anchor_bottom = 0.43
	preview_button.offset_left = 0
	preview_button.offset_right = 0
	preview_button.offset_top = 0
	preview_button.offset_bottom = 0
	card.add_child(preview_button)

	# AddButton
	var add_button = Button.new()
	add_button.name = "AddButton"
	add_button.text = "+"
	add_button.z_index = 11
	add_button.visible = false
	add_button.anchor_left = 0.15
	add_button.anchor_right = 0.85
	add_button.anchor_top = 0.48
	add_button.anchor_bottom = 0.66
	add_button.offset_left = 0
	add_button.offset_right = 0
	add_button.offset_top = 0
	add_button.offset_bottom = 0
	card.add_child(add_button)

	# 3张上限禁用
	if card_counts.get(card_id, 0) >= 3:
		add_button.disabled = true

	# 信号连接
	preview_button.pressed.connect(_on_preview_pressed.bind(card))
	add_button.pressed.connect(_on_add_card_to_deck.bind(card_id))
	card.resized.connect(_update_catalog_button_fonts.bind(card))

	# 延迟一帧后更新字体，确保 CardBox 已完成布局
	call_deferred("_update_catalog_button_fonts", card)


## 卡组卡片按钮字体自适应
func _update_deck_button_fonts(card: DuleCard) -> void:
	var preview_button = card.get_node_or_null("PreviewButton")
	var add_button = card.get_node_or_null("AddButton")
	var remove_button = card.get_node_or_null("RemoveButton")

	if preview_button != null:
		var font_size = max(8, int(preview_button.size.y * 0.40))
		preview_button.add_theme_font_size_override("font_size", font_size)
	if add_button != null:
		var font_size = max(8, int(add_button.size.y * 0.55))
		add_button.add_theme_font_size_override("font_size", font_size)
	if remove_button != null:
		var font_size = max(8, int(remove_button.size.y * 0.55))
		remove_button.add_theme_font_size_override("font_size", font_size)


## 查询卡片按钮字体自适应
func _update_catalog_button_fonts(card: DuleCard) -> void:
	var preview_button = card.get_node_or_null("PreviewButton")
	var add_button = card.get_node_or_null("AddButton")

	if preview_button != null:
		var font_size = max(8, int(preview_button.size.y * 0.40))
		preview_button.add_theme_font_size_override("font_size", font_size)
	if add_button != null:
		var font_size = max(8, int(add_button.size.y * 0.55))
		add_button.add_theme_font_size_override("font_size", font_size)


# ---------------------------------------------------------------------------
# 悬停检测（_process 轮询）
# ---------------------------------------------------------------------------

func _process(_delta: float) -> void:
	var mouse_pos = get_global_mouse_position()
	_update_hover_for_card_box(deck_card_box, mouse_pos)
	_update_hover_for_card_box(catalog_card_box, mouse_pos)


func _update_hover_for_card_box(card_box: CardBox, mouse_pos: Vector2) -> void:
	var cards_node = card_box.get_node("ScrollContainer/Cards")
	for child in cards_node.get_children():
		if child is DuleCard:
			var is_inside = child.get_global_rect().has_point(mouse_pos)
			var was_hovered = child.get_meta("_hovered", false)
			if is_inside != was_hovered:
				child.set_meta("_hovered", is_inside)
				_update_card_hover(child, is_inside)


func _update_card_hover(card: DuleCard, hovered: bool) -> void:
	var hover_overlay = card.get_node_or_null("HoverOverlay")
	var preview_button = card.get_node_or_null("PreviewButton")
	var add_button = card.get_node_or_null("AddButton")
	var remove_button = card.get_node_or_null("RemoveButton")

	if hover_overlay != null:
		hover_overlay.visible = hovered
	if preview_button != null:
		preview_button.visible = hovered
	if add_button != null:
		add_button.visible = hovered
	if remove_button != null:
		remove_button.visible = hovered


# ---------------------------------------------------------------------------
# 交互处理
# ---------------------------------------------------------------------------

func _on_preview_pressed(card: DuleCard) -> void:
	CardPreviewPopup.show_preview(self, card.card_data)


## 添加卡片到卡组
func _on_add_card_to_deck(card_id: String) -> void:
	var cards = current_data.get("cards", [])
	cards.append(card_id)
	has_unsaved_changes = true
	_update_save_button()
	_refresh_deck_grid()
	_refresh_catalog_grid()
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
		_refresh_catalog_grid()
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
