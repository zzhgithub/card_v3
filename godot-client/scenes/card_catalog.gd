extends Control

## 卡片图鉴场景
## 使用 CardFactory API 显示所有可用卡片

@onready var title_label: Label = $VBoxContainer/TitleLabel
@onready var back_button: Button = $VBoxContainer/BackButton
@onready var scroll_container: ScrollContainer = $VBoxContainer/ScrollContainer
@onready var grid_container: GridContainer = $VBoxContainer/ScrollContainer/GridContainer
@onready var status_label: Label = $VBoxContainer/StatusLabel
@onready var card_detail_panel: Panel = $CardDetailPanel
@onready var card_detail_label: RichTextLabel = $CardDetailPanel/CardDetailLabel

## CardFactory 实例（遵循 CardFactory API）
var card_factory: CatalogCardFactory
var card_displays: Array = []

func _ready():
	# 连接信号
	back_button.pressed.connect(_on_back_pressed)

	# 初始化UI
	card_detail_panel.visible = false

	# 创建并配置 CardFactory
	_setup_card_factory()

	# 加载并显示所有卡片
	_load_and_display_cards()


## 设置 CardFactory 实例
func _setup_card_factory() -> void:
	card_factory = CatalogCardFactory.new()
	card_factory.card_display_scene = preload("res://scenes/catalog_card_display.tscn")
	card_factory.card_asset_dir = "res://images/cards"
	card_factory.card_info_dir = "res://scripts_json/S000"

	# 预加载所有卡片数据
	card_factory.preload_card_data()

	print("[CardCatalog] CardFactory initialized with %d cards" % card_factory.preloaded_cards.size())


## 加载并显示所有卡片（使用 CardFactory API）
func _load_and_display_cards() -> void:
	# 清除现有内容
	_clear_displayed_cards()

	# 使用 CardFactory 获取所有卡片ID
	var card_ids = card_factory.get_available_card_ids()

	# 使用 CardFactory API 创建每个卡片显示
	for card_id in card_ids:
		var card_data = card_factory.load_card_full_data(card_id)
		if card_data.is_empty():
			continue

		# 使用 CardFactory 创建卡片显示
		var card_display = card_factory.create_card_display(card_id, card_data, grid_container)

		if card_display != null:
			# 连接信号
			card_display.card_hovered.connect(_on_card_hovered)
			card_display.card_unhovered.connect(_on_card_unhovered)
			card_displays.append(card_display)

	# 更新状态
	_update_status(card_ids.size())

	print("[CardCatalog] Displayed %d cards using CardFactory API" % card_displays.size())


## 清除已显示的卡片
func _clear_displayed_cards() -> void:
	for display in card_displays:
		if is_instance_valid(display):
			display.queue_free()
	card_displays.clear()

	# 清除网格容器中剩余的子节点
	for child in grid_container.get_children():
		child.queue_free()


## 卡片悬停处理
func _on_card_hovered(card_data: Dictionary) -> void:
	var detail_text = _format_card_detail(card_data)
	card_detail_label.text = detail_text
	card_detail_panel.visible = true


## 卡片取消悬停处理
func _on_card_unhovered() -> void:
	card_detail_panel.visible = false


## 格式化卡片详情文本
func _format_card_detail(card_data: Dictionary) -> String:
	var result = ""

	result += "[b]编号:[/b] %s\n" % card_data.get("id", "Unknown")
	result += "[b]名称:[/b] %s\n" % card_data.get("name", "Unknown")
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

	# 显示效果（简化显示）
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
