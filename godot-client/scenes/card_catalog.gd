extends Control

## 卡片图鉴场景
## 显示所有可用卡片，支持预览和预留功能

const CARD_DISPLAY_SCENE = preload("res://scenes/card_display.tscn")
const CARD_PREVIEW_POPUP = preload("res://scenes/card_preview_popup.tscn")

@onready var title_label: Label = $VBoxContainer/TitleLabel
@onready var back_button: Button = $VBoxContainer/BackButton
@onready var scroll_container: ScrollContainer = $VBoxContainer/ScrollContainer
@onready var grid_container: GridContainer = $VBoxContainer/ScrollContainer/GridContainer
@onready var status_label: Label = $VBoxContainer/StatusLabel

## CardFactory 实例
var card_factory: CatalogCardFactory
var card_items: Array = []
var current_preview_popup: Node = null

func _ready():
	# 连接信号
	back_button.pressed.connect(_on_back_pressed)

	# 创建并配置 CardFactory
	_setup_card_factory()

	# 加载并显示所有卡片
	_load_and_display_cards()


## 设置 CardFactory 实例
func _setup_card_factory() -> void:
	card_factory = CatalogCardFactory.new()
	card_factory.card_display_scene = CARD_DISPLAY_SCENE
	card_factory.card_asset_dir = "res://images/cards"
	card_factory.card_info_dir = "res://scripts_json/S000"

	# 预加载所有卡片数据
	card_factory.preload_card_data()

	print("[CardCatalog] CardFactory initialized with %d cards" % card_factory.preloaded_cards.size())


## 加载并显示所有卡片
func _load_and_display_cards() -> void:
	# 清除现有内容
	_clear_displayed_cards()

	# 使用 CardFactory 获取所有卡片ID
	var card_ids = card_factory.get_available_card_ids()

	# 创建每个卡片项
	for card_id in card_ids:
		var card_data = card_factory.load_card_full_data(card_id)
		if card_data.is_empty():
			continue

		_create_card_item(card_data)

	# 更新状态
	_update_status(card_ids.size())

	print("[CardCatalog] Displayed %d cards" % card_items.size())


## 创建单个卡片项
func _create_card_item(card_data: Dictionary) -> void:
	var card_id = card_data.get("id", "Unknown")

	# 创建容器
	var item_container = VBoxContainer.new()
	item_container.custom_minimum_size = Vector2(150, 210)
	item_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# 创建卡片显示
	var card_display = card_factory.create_card_display(card_id, card_data)
	if card_display == null:
		return

	card_display.custom_minimum_size = Vector2(150, 210)
	card_display.size_flags_vertical = Control.SIZE_EXPAND_FILL

	# 连接预览信号
	card_display.preview_requested.connect(_on_preview_requested)

	# 创建ID标签
	var id_label = Label.new()
	id_label.text = card_id
	id_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	id_label.add_theme_font_size_override("font_size", 12)

	# 将组件添加到容器
	item_container.add_child(card_display)
	item_container.add_child(id_label)

	# 存储引用
	card_items.append({
		"container": item_container,
		"display": card_display,
		"id_label": id_label,
		"data": card_data
	})

	# 添加到网格
	grid_container.add_child(item_container)


## 清除已显示的卡片
func _clear_displayed_cards() -> void:
	for item in card_items:
		if is_instance_valid(item.container):
			item.container.queue_free()
	card_items.clear()

	# 清除网格容器中剩余的子节点
	for child in grid_container.get_children():
		child.queue_free()


## 预览请求处理
func _on_preview_requested(card_data: Dictionary) -> void:
	# 关闭已存在的预览弹窗
	if current_preview_popup != null and is_instance_valid(current_preview_popup):
		current_preview_popup.queue_free()

	# 创建新的预览弹窗
	current_preview_popup = CARD_PREVIEW_POPUP.instantiate()
	add_child(current_preview_popup)
	current_preview_popup.setup(card_data)

	# 连接关闭信号
	current_preview_popup.closed.connect(func(): current_preview_popup = null)


## 更新状态显示
func _update_status(count: int) -> void:
	status_label.text = "共 %d 张卡片" % count


## 返回主菜单
func _on_back_pressed() -> void:
	# 关闭预览弹窗
	if current_preview_popup != null and is_instance_valid(current_preview_popup):
		current_preview_popup.queue_free()

	# 清理 CardFactory
	if card_factory != null:
		card_factory.queue_free()
		card_factory = null

	get_tree().change_scene_to_file("res://scenes/main.tscn")
