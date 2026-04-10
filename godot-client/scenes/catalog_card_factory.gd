@tool
## 卡片图鉴专用工厂
## 使用 CardFactory API 创建用于图鉴显示的卡片

class_name CatalogCardFactory
extends CardFactory

@export_group("card_scenes")
## 卡片显示场景
@export var card_display_scene: PackedScene

@export_group("asset_paths")
## 卡片图片资源目录
@export var card_asset_dir: String = "res://images/cards"
## 卡片数据JSON目录
@export var card_info_dir: String = "res://scripts_json/S000"

@export_group("default_textures")
## 默认卡背图片
@export var back_image: Texture2D
## 默认占位图片（当卡片图片缺失时）
@export var placeholder_texture: Texture2D

## 当前处理的卡片数据
var _current_card_data: Dictionary = {}

func _ready() -> void:
	if card_display_scene == null:
		push_error("[CatalogCardFactory] card_display_scene is not assigned!")
		return

	# 验证场景类型
	var temp_instance = card_display_scene.instantiate()
	if not (temp_instance is CatalogCardDisplay):
		push_error("[CatalogCardFactory] card_display_scene must be a CatalogCardDisplay!")
		card_display_scene = null
	temp_instance.queue_free()


## 覆写 CardFactory.create_card 以符合 API 签名
## 注意：图鉴不使用可拖动 Card，所以返回 null
## 请使用 create_card_display() 方法创建图鉴显示
func create_card(card_name: String, target: CardContainer) -> Card:
	# 图鉴不需要创建可拖动 Card，返回 null
	# 使用 create_card_display() 替代
	push_warning("[CatalogCardFactory] create_card() returns null. Use create_card_display() for catalog.")
	return null


## 创建并配置卡片显示（图鉴专用方法）
## @param card_id: 卡片ID
## @param card_data: 卡片数据字典
## @param parent: 父节点
## @return: CatalogCardDisplay 实例
func create_card_display(card_id: String, card_data: Dictionary, parent: Node = null) -> CatalogCardDisplay:
	"""
	直接使用已有的卡片数据创建显示
	"""
	if card_display_scene == null:
		return null

	var display = card_display_scene.instantiate() as CatalogCardDisplay
	if display == null:
		return null

	if parent != null:
		parent.add_child(display)

	var front_image = _load_card_image(card_id)
	if front_image == null:
		front_image = placeholder_texture

	display.setup(card_data, front_image, back_image)

	return display


## 加载所有可用的卡片ID
func get_available_card_ids() -> Array[String]:
	var ids: Array[String] = []

	var dir = DirAccess.open(card_info_dir)
	if dir == null:
		push_error("[CatalogCardFactory] Failed to open directory: %s" % card_info_dir)
		return ids

	dir.list_dir_begin()
	var file_name = dir.get_next()

	while file_name != "":
		if file_name.ends_with(".json"):
			var card_id = file_name.get_basename()
			ids.append(card_id)
		file_name = dir.get_next()

	dir.list_dir_end()
	ids.sort()

	return ids


## 加载单张卡片的完整数据
func load_card_full_data(card_id: String) -> Dictionary:
	return _load_card_info(card_id)


## 预加载所有卡片数据到缓存
func preload_card_data() -> void:
	preloaded_cards.clear()

	var dir = DirAccess.open(card_info_dir)
	if dir == null:
		push_error("[CatalogCardFactory] Failed to open directory: %s" % card_info_dir)
		return

	dir.list_dir_begin()
	var file_name = dir.get_next()

	while file_name != "":
		if file_name.ends_with(".json"):
			var card_id = file_name.get_basename()
			var card_data = _load_card_info(card_id)
			if not card_data.is_empty():
				preloaded_cards[card_id] = card_data
				print("[CatalogCardFactory] Preloaded: %s" % card_id)
		file_name = dir.get_next()

	dir.list_dir_end()


## 从JSON文件加载卡片信息
func _load_card_info(card_name: String) -> Dictionary:
	# 如果已在缓存中，直接返回
	if preloaded_cards.has(card_name):
		return preloaded_cards[card_name]

	var json_path = card_info_dir + "/" + card_name + ".json"
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
		push_error("[CatalogCardFactory] Failed to parse JSON: %s" % json_path)
		return {}

	var data = json.data
	# 缓存数据
	preloaded_cards[card_name] = data

	return data


## 加载卡片图片
func _load_card_image(card_id: String) -> Texture2D:
	var image_path = card_asset_dir + "/" + card_id + ".png"

	if not ResourceLoader.exists(image_path):
		return null

	var texture = load(image_path) as Texture2D
	return texture
