extends Control

## 卡组详情页
## 编辑单个卡组的卡片内容

signal back_requested(saved: bool)
signal deck_renamed(old_name: String, new_name: String)

const DECKS_DIR = "res://desks"

@onready var title_label: Label = $VBoxContainer/TitleLabel
@onready var card_count_label: Label = $VBoxContainer/CardCountLabel
@onready var rename_button: Button = $VBoxContainer/RenameButton
@onready var save_button: Button = $VBoxContainer/ButtonContainer/SaveButton
@onready var back_button: Button = $VBoxContainer/ButtonContainer/BackButton
@onready var card_list: VBoxContainer = $VBoxContainer/ScrollContainer/CardList

var deck_name: String = ""
var original_data: Dictionary = {}
var current_data: Dictionary = {}
var has_unsaved_changes: bool = false

func _ready():
	# 连接信号
	rename_button.pressed.connect(_on_rename_pressed)
	save_button.pressed.connect(_on_save_pressed)
	back_button.pressed.connect(_on_back_pressed)


## 设置卡组数据
func setup(deck_name_param: String) -> void:
	deck_name = deck_name_param
	title_label.text = "卡组: %s" % deck_name

	# 加载卡组数据
	_load_deck_data()

	# 显示卡片列表
	_refresh_card_list()


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
			push_error("[DeckDetail] 解析JSON失败: %s" % file_path)
			original_data = {"name": deck_name, "cards": []}
			current_data = {"name": deck_name, "cards": []}
	else:
		push_error("[DeckDetail] 无法读取文件: %s" % file_path)
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


## 刷新卡片列表显示
func _refresh_card_list() -> void:
	# 清除现有列表
	for child in card_list.get_children():
		child.queue_free()

	var cards = current_data.get("cards", [])
	var card_count = cards.size()

	# 更新卡片数量
	card_count_label.text = "卡片数量: %d" % card_count

	# 显示每张卡片
	for i in range(card_count):
		var card_id = cards[i]
		_create_card_item(i, card_id)

	# TODO: 添加卡片的UI
	var todo_label = Label.new()
	todo_label.text = "TODO: 卡片编辑功能"
	todo_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	todo_label.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))
	card_list.add_child(todo_label)


## 创建卡片列表项
func _create_card_item(index: int, card_id: String) -> void:
	var item = HBoxContainer.new()

	var index_label = Label.new()
	index_label.text = "%d." % (index + 1)
	index_label.custom_minimum_size = Vector2(40, 0)
	item.add_child(index_label)

	var card_label = Label.new()
	card_label.text = card_id
	card_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	item.add_child(card_label)

	var remove_button = Button.new()
	remove_button.text = "移除"
	remove_button.pressed.connect(_on_remove_card.bind(index))
	item.add_child(remove_button)

	card_list.add_child(item)


## 移除卡片
func _on_remove_card(index: int) -> void:
	var cards = current_data.get("cards", [])
	if index >= 0 and index < cards.size():
		cards.remove_at(index)
		has_unsaved_changes = true
		_update_save_button()
		_refresh_card_list()


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
		card_count_label.text = "卡片数量: %d (已保存)" % current_data.get("cards", []).size()


## 返回按钮点击
func _on_back_pressed() -> void:
	has_unsaved_changes = _check_unsaved_changes()

	if has_unsaved_changes:
		# 显示未保存提示
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

	# 检查新名称是否已存在
	if FileAccess.file_exists(new_file_path):
		push_error("[DeckDetail] 卡组 '%s' 已存在" % new_name)
		return

	# 更新数据中的名称
	current_data["name"] = new_name

	# 保存到新文件
	var file = FileAccess.open(new_file_path, FileAccess.WRITE)
	if file == null:
		push_error("[DeckDetail] 无法创建新文件: %s" % new_file_path)
		return

	var json_string = JSON.stringify(current_data, "  ")
	file.store_string(json_string)
	file.close()

	# 删除旧文件
	DirAccess.remove_absolute(old_file_path)

	var old_name = deck_name
	deck_name = new_name

	# 更新标题
	title_label.text = "卡组: %s" % deck_name

	# 更新原始数据
	original_data = current_data.duplicate(true)
	has_unsaved_changes = false
	_update_save_button()

	print("[DeckDetail] 卡组已重命名: %s -> %s" % [old_name, deck_name])

	# 发送重命名信号
	emit_signal("deck_renamed", old_name, deck_name)
