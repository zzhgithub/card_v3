extends Control

## 卡组编辑页面
## 显示卡组列表，支持新增、删除、编辑卡组

const DECK_DETAIL_SCENE = preload("res://scenes/deck_detail.tscn")
const DECKS_DIR = "res://desks"

@onready var title_label: Label = $VBoxContainer/TitleLabel
@onready var status_label: Label = $VBoxContainer/StatusLabel
@onready var deck_list: VBoxContainer = $VBoxContainer/ScrollContainer/DeckList
@onready var add_button: Button = $VBoxContainer/AddDeckButton
@onready var back_button: Button = $VBoxContainer/BackButton

var deck_items: Array = []

func _ready():
	# 连接信号
	add_button.pressed.connect(_on_add_deck_pressed)
	back_button.pressed.connect(_on_back_pressed)

	# 加载卡组列表
	_load_decks()

	print("[DeckEditor] 卡组编辑页面已打开")


## 加载所有卡组
func _load_decks() -> void:
	# 清除现有列表
	_clear_deck_list()

	var dir = DirAccess.open(DECKS_DIR)
	if dir == null:
		status_label.text = "无法读取卡组目录"
		return

	var deck_count = 0
	dir.list_dir_begin()
	var file_name = dir.get_next()

	while file_name != "":
		if file_name.ends_with(".json"):
			var deck_name = file_name.get_basename()
			_create_deck_item(deck_name)
			deck_count += 1
		file_name = dir.get_next()

	dir.list_dir_end()

	status_label.text = "共 %d 个卡组" % deck_count


## 创建卡组列表项
func _create_deck_item(deck_name: String) -> void:
	var item_container = HBoxContainer.new()
	item_container.custom_minimum_size = Vector2(0, 50)
	item_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# 卡组名称按钮
	var name_button = Button.new()
	name_button.text = deck_name
	name_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	name_button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	name_button.pressed.connect(_on_deck_clicked.bind(deck_name))
	item_container.add_child(name_button)

	# 删除按钮
	var delete_button = Button.new()
	delete_button.text = "删除"
	delete_button.custom_minimum_size = Vector2(80, 0)
	delete_button.pressed.connect(_on_delete_deck_pressed.bind(deck_name, item_container))
	item_container.add_child(delete_button)

	deck_list.add_child(item_container)
	deck_items.append(item_container)


## 清除卡组列表
func _clear_deck_list() -> void:
	for item in deck_items:
		if is_instance_valid(item):
			item.queue_free()
	deck_items.clear()


## 点击卡组名称
func _on_deck_clicked(deck_name: String) -> void:
	print("[DeckEditor] 打开卡组: %s" % deck_name)
	var detail = DECK_DETAIL_SCENE.instantiate()
	get_tree().root.add_child(detail)
	detail.setup(deck_name)
	detail.back_requested.connect(_on_detail_back.bind(detail))
	detail.deck_renamed.connect(_on_deck_renamed)
	self.visible = false


## 详情页返回
func _on_detail_back(saved: bool, detail_node: Node) -> void:
	print("[DeckEditor] 从详情页返回，已保存: %s" % saved)
	detail_node.queue_free()
	self.visible = true
	# 刷新列表
	_load_decks()


## 卡组重命名
func _on_deck_renamed(old_name: String, new_name: String) -> void:
	print("[DeckEditor] 卡组重命名: %s -> %s" % [old_name, new_name])
	# 刷新列表以显示新名称
	_load_decks()


## 新增卡组
func _on_add_deck_pressed() -> void:
	print("[DeckEditor] 新增卡组")

	# 弹出输入对话框
	var name_input_dialog = _create_name_input_dialog("新建卡组", "", func(new_name: String):
		if new_name.is_empty():
			status_label.text = "卡组名称不能为空"
			return

		# 检查名称是否已存在
		var file_path = DECKS_DIR + "/" + new_name + ".json"
		if FileAccess.file_exists(file_path):
			status_label.text = "卡组 '%s' 已存在" % new_name
			return

		var deck_data = {
			"name": new_name,
			"cards": []
		}

		# 保存新卡组
		if _save_deck(new_name, deck_data):
			_create_deck_item(new_name)
			_update_status()
			status_label.text = "卡组 '%s' 创建成功" % new_name
	)
	add_child(name_input_dialog)
	name_input_dialog.popup_centered()


## 创建名称输入对话框
func _create_name_input_dialog(title: String, default_name: String, callback: Callable) -> Window:
	var dialog = Window.new()
	dialog.title = title
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
	label.text = "请输入卡组名称："
	vbox.add_child(label)

	var line_edit = LineEdit.new()
	line_edit.text = default_name
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
		callback.call(line_edit.text.strip_edges())
		dialog.queue_free()
	)

	cancel_button.pressed.connect(func():
		dialog.queue_free()
	)

	line_edit.text_submitted.connect(func(_text):
		callback.call(line_edit.text.strip_edges())
		dialog.queue_free()
	)

	# 显示时聚焦输入框
	dialog.visibility_changed.connect(func():
		if dialog.visible:
			line_edit.grab_focus()
			line_edit.select_all()
	)

	return dialog


## 删除卡组
func _on_delete_deck_pressed(deck_name: String, item_container: Node) -> void:
	print("[DeckEditor] 删除卡组: %s" % deck_name)

	# 确认对话框
	var confirm_dialog = ConfirmationDialog.new()
	confirm_dialog.title = "确认删除"
	confirm_dialog.dialog_text = "确定要删除卡组 '%s' 吗？" % deck_name
	confirm_dialog.ok_button_text = "删除"
	confirm_dialog.cancel_button_text = "取消"
	confirm_dialog.confirmed.connect(func():
		_delete_deck_file(deck_name)
		item_container.queue_free()
		deck_items.erase(item_container)
		_update_status()
	)
	add_child(confirm_dialog)
	confirm_dialog.popup_centered()


## 生成新卡组名称
func _generate_new_deck_name() -> String:
	var index = 1
	while true:
		var new_name = "NewDeck%d" % index
		var file_path = DECKS_DIR + "/" + new_name + ".json"
		if not FileAccess.file_exists(file_path):
			return new_name
		index += 1
	return "NewDeck"


## 保存卡组到文件
func _save_deck(deck_name: String, deck_data: Dictionary) -> bool:
	var file_path = DECKS_DIR + "/" + deck_name + ".json"
	var file = FileAccess.open(file_path, FileAccess.WRITE)
	if file == null:
		push_error("[DeckEditor] 无法保存卡组: %s" % deck_name)
		return false

	var json_string = JSON.stringify(deck_data, "  ")
	file.store_string(json_string)
	file.close()
	return true


## 删除卡组文件
func _delete_deck_file(deck_name: String) -> void:
	var file_path = DECKS_DIR + "/" + deck_name + ".json"
	DirAccess.remove_absolute(file_path)


## 更新状态显示
func _update_status() -> void:
	status_label.text = "共 %d 个卡组" % deck_items.size()


## 返回主菜单
func _on_back_pressed() -> void:
	get_tree().change_scene_to_file("res://scenes/main.tscn")
