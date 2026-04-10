extends Control

## 卡片图鉴场景
## 显示所有可用卡片，支持滚动浏览和悬停查看详情

const CARD_DISPLAY_SCENE = preload("res://scenes/card_display.tscn")

@onready var title_label: Label = $VBoxContainer/TitleLabel
@onready var back_button: Button = $VBoxContainer/BackButton
@onready var scroll_container: ScrollContainer = $VBoxContainer/ScrollContainer
@onready var grid_container: GridContainer = $VBoxContainer/ScrollContainer/GridContainer
@onready var status_label: Label = $VBoxContainer/StatusLabel

## CardFactory 实例（遵循 CardFactory API）
var card_factory: CatalogCardFactory
var card_items: Array = []

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

	# 创建每个卡片项（包含ID标签和卡片显示）
	for card_id in card_ids:
		var card_data = card_factory.load_card_full_data(card_id)
		if card_data.is_empty():
			continue

		_create_card_item(card_data)

	# 更新状态
	_update_status(card_ids.size())

	print("[CardCatalog] Displayed %d cards" % card_items.size())


## 创建单个卡片项（包含ID标签和卡片显示）
func _create_card_item(card_data: Dictionary) -> void:
	var card_id = card_data.get("id", "Unknown")

	# 创建容器
	var item_container = VBoxContainer.new()
	item_container.custom_minimum_size = Vector2(150, 240)
	item_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# 创建卡片显示
	var card_display = card_factory.create_card_display(card_id, card_data)
	if card_display == null:
		return

	card_display.custom_minimum_size = Vector2(150, 190)
	card_display.size_flags_vertical = Control.SIZE_EXPAND_FILL

	# 连接卡片信号
	card_display.card_hovered.connect(_on_card_hovered.bind(card_display))
	card_display.card_unhovered.connect(_on_card_unhovered.bind(card_display))
	card_display.card_clicked.connect(_on_card_clicked.bind(card_data))

	# 创建ID标签
	var id_label = Label.new()
	id_label.text = card_id
	id_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	id_label.add_theme_font_size_override("font_size", 12)

	# 创建预留按钮（默认隐藏，hover时显示）
	var reserve_button = Button.new()
	reserve_button.text = "预留"
	reserve_button.visible = false
	reserve_button.custom_minimum_size = Vector2(60, 30)
	reserve_button.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	reserve_button.pressed.connect(_on_reserve_button_pressed.bind(card_data))

	# 将组件添加到容器
	item_container.add_child(card_display)
	item_container.add_child(id_label)
	item_container.add_child(reserve_button)

	# 存储引用
	card_items.append({
		"container": item_container,
		"display": card_display,
		"id_label": id_label,
		"reserve_button": reserve_button,
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


## 卡片悬停处理
func _on_card_hovered(card_data: Dictionary, card_display: CardDisplay) -> void:
	# 找到对应的项并显示预留按钮
	for item in card_items:
		if item.display == card_display:
			item.reserve_button.visible = true
			break


## 卡片取消悬停处理
func _on_card_unhovered(card_display: CardDisplay) -> void:
	# 隐藏预留按钮
	for item in card_items:
		if item.display == card_display:
			item.reserve_button.visible = false
			break


## 卡片点击处理 - 显示大图预览
func _on_card_clicked(card_data: Dictionary) -> void:
	_show_large_card_preview(card_data)


## 预留按钮点击处理
func _on_reserve_button_pressed(card_data: Dictionary) -> void:
	print("[CardCatalog] Reserved card: %s" % card_data.get("id", "Unknown"))
	# TODO: 实现预留逻辑


## 显示大图预览
func _show_large_card_preview(card_data: Dictionary) -> void:
	var card_id = card_data.get("id", "Unknown")

	# 创建预览弹窗
	var preview_popup = Control.new()
	preview_popup.name = "CardPreviewPopup"
	preview_popup.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(preview_popup)

	# 背景遮罩
	var overlay = ColorRect.new()
	overlay.color = Color(0, 0, 0, 0.8)
	overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
	preview_popup.add_child(overlay)

	# 点击背景关闭
	overlay.gui_input.connect(func(event):
		if event is InputEventMouseButton and event.pressed:
			preview_popup.queue_free()
	)

	# 内容容器（居中）
	var content = CenterContainer.new()
	content.set_anchors_preset(Control.PRESET_FULL_RECT)
	preview_popup.add_child(content)

	# 卡片容器
	var card_container = VBoxContainer.new()
	card_container.alignment = BoxContainer.ALIGNMENT_CENTER
	content.add_child(card_container)

	# 创建大图卡片显示
	var large_display = CARD_DISPLAY_SCENE.instantiate()
	large_display.custom_minimum_size = Vector2(300, 420)

	var front_image = _load_card_image(card_id)
	large_display.setup(card_data, front_image)
	card_container.add_child(large_display)

	# 卡片名称
	var name_label = Label.new()
	name_label.text = card_data.get("name", card_id)
	name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	name_label.add_theme_font_size_override("font_size", 24)
	name_label.add_theme_color_override("font_color", Color.WHITE)
	card_container.add_child(name_label)

	# 详细属性（预留时显示）
	var detail_text = _format_card_detail(card_data)
	var detail_label = RichTextLabel.new()
	detail_label.bbcode_enabled = true
	detail_label.text = detail_text
	detail_label.custom_minimum_size = Vector2(300, 150)
	detail_label.fit_content = true
	detail_label.add_theme_color_override("default_color", Color.WHITE)
	card_container.add_child(detail_label)

	# 关闭按钮
	var close_button = Button.new()
	close_button.text = "×"
	close_button.add_theme_font_size_override("font_size", 32)
	close_button.custom_minimum_size = Vector2(50, 50)
	close_button.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	close_button.position = Vector2(20, 20)
	close_button.pressed.connect(func(): preview_popup.queue_free())
	preview_popup.add_child(close_button)


## 加载卡片图片
func _load_card_image(card_id: String) -> Texture2D:
	var image_path = "res://images/cards/%s.png" % card_id
	if ResourceLoader.exists(image_path):
		return load(image_path) as Texture2D
	return null


## 格式化卡片详情文本
func _format_card_detail(card_data: Dictionary) -> String:
	var result = ""

	result += "[b]编号:[/b] %s\n" % card_data.get("id", "Unknown")
	result += "[b]类型:[/b] %s\n" % card_data.get("card_type", "Unknown")

	# 根据卡片类型显示不同信息
	match card_data.get("card_type"):
		"Character":
			result += "[b]属性:[/b] %s\n" % card_data.get("property", "-")
			result += "[b]领域:[/b] %s\n" % card_data.get("category", "-")
			result += "[b]费用:[/b] %d\n" % card_data.get("cost", 0)
			result += "[b]攻击力:[/b] %d\n" % card_data.get("attack", 0)
		"Item":
			result += "[b]种类:[/b] %s\n" % card_data.get("item_kind", "-")
			result += "[b]属性:[/b] %s\n" % card_data.get("property", "-")
			result += "[b]领域:[/b] %s\n" % card_data.get("category", "-")
			result += "[b]费用:[/b] %d\n" % card_data.get("cost", 0)
		"Strategy":
			result += "[b]种类:[/b] %s\n" % card_data.get("strategy_kind", "-")
			result += "[b]属性:[/b] %s\n" % card_data.get("property", "-")
			result += "[b]领域:[/b] %s\n" % card_data.get("category", "-")
			result += "[b]费用:[/b] %d\n" % card_data.get("cost", 0)
		"Legend":
			result += "[b]属性:[/b] %s\n" % card_data.get("property", "-")
			result += "[b]费用:[/b] %d\n" % card_data.get("cost", 0)

	# 显示效果
	var effects = card_data.get("effects", {})
	if not effects.is_empty():
		result += "\n[b]效果:[/b]"
		for effect_id in effects:
			var effect = effects[effect_id]
			var trigger = effect.get("trigger", "")
			result += "\n• %s" % trigger

	return result


## 更新状态显示
func _update_status(count: int) -> void:
	status_label.text = "共 %d 张卡片" % count


## 返回主菜单
func _on_back_pressed() -> void:
	# 清理 CardFactory
	if card_factory != null:
		card_factory.queue_free()
		card_factory = null

	get_tree().change_scene_to_file("res://scenes/main.tscn")
