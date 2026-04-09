extends Control

## 卡片图鉴场景
## 显示所有可用卡片，支持滚动浏览和悬停查看详情

const CARD_DISPLAY_SCENE = preload("res://scenes/card_display.tscn")

@onready var title_label: Label = $VBoxContainer/TitleLabel
@onready var back_button: Button = $VBoxContainer/BackButton
@onready var scroll_container: ScrollContainer = $VBoxContainer/ScrollContainer
@onready var grid_container: GridContainer = $VBoxContainer/ScrollContainer/GridContainer
@onready var status_label: Label = $VBoxContainer/StatusLabel
@onready var card_detail_panel: Panel = $CardDetailPanel
@onready var card_detail_label: RichTextLabel = $CardDetailPanel/CardDetailLabel

var card_data_list: Array = []
var card_displays: Array = []

func _ready():
	# 连接信号
	back_button.pressed.connect(_on_back_pressed)

	# 初始化UI
	card_detail_panel.visible = false

	# 加载卡片数据
	_load_all_card_data()

	# 创建卡片显示
	_create_card_displays()

	# 更新状态
	_update_status()

func _load_all_card_data() -> void:
	"""从scripts_json目录加载所有卡片数据"""
	card_data_list.clear()

	# 扫描scripts_json/S000目录
	var card_dir = DirAccess.open("res://scripts_json/S000")
	if card_dir == null:
		push_error("[CardCatalog] Failed to open card data directory")
		return

	card_dir.list_dir_begin()
	var file_name = card_dir.get_next()

	while file_name != "":
		if file_name.ends_with(".json"):
			var card_data = _load_card_data("S000/" + file_name)
			if not card_data.is_empty():
				card_data_list.append(card_data)

		file_name = card_dir.get_next()

	card_dir.list_dir_end()

	# 按卡片ID排序
	card_data_list.sort_custom(func(a, b): return a.get("id", "") < b.get("id", ""))

	print("[CardCatalog] Loaded %d cards" % card_data_list.size())

func _load_card_data(file_path: String) -> Dictionary:
	"""加载单个卡片JSON数据"""
	var full_path = "res://scripts_json/" + file_path
	if not FileAccess.file_exists(full_path):
		return {}

	var file = FileAccess.open(full_path, FileAccess.READ)
	if file == null:
		return {}

	var json_string = file.get_as_text()
	file.close()

	var json = JSON.new()
	var error = json.parse(json_string)
	if error != OK:
		push_error("[CardCatalog] Failed to parse JSON: %s" % file_path)
		return {}

	return json.data

func _create_card_displays() -> void:
	"""创建所有卡片显示组件"""
	# 清除现有内容
	for child in grid_container.get_children():
		child.queue_free()
	card_displays.clear()

	# 创建卡片显示
	for card_data in card_data_list:
		var card_display = CARD_DISPLAY_SCENE.instantiate()
		grid_container.add_child(card_display)

		card_display.setup(card_data)
		card_display.card_hovered.connect(_on_card_hovered)
		card_display.card_unhovered.connect(_on_card_unhovered)

		card_displays.append(card_display)

func _on_card_hovered(card_data: Dictionary) -> void:
	"""鼠标悬停在卡片上时显示详情"""
	var detail_text = _format_card_detail(card_data)
	card_detail_label.text = detail_text
	card_detail_panel.visible = true

func _on_card_unhovered() -> void:
	"""鼠标离开卡片时隐藏详情"""
	card_detail_panel.visible = false

func _format_card_detail(card_data: Dictionary) -> String:
	"""格式化卡片详情文本"""
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

func _update_status() -> void:
	"""更新状态显示"""
	status_label.text = "共 %d 张卡片" % card_data_list.size()

func _on_back_pressed() -> void:
	"""返回主菜单"""
	get_tree().change_scene_to_file("res://scenes/main.tscn")
