class_name CardBox
extends CardContainer

signal card_clicked(card_data: Dictionary)

@export var columns: int = 5:
	set(value):
		columns = max(1, value)
		if is_inside_tree():
			update_card_ui()

@export var h_separation: int = 10
@export var v_separation: int = 10

const DESIGN_ASPECT := 1500.0 / 2100.0

@onready var scroll_container: ScrollContainer = $ScrollContainer

var _suppress_layout_count: int = 0

func _ready() -> void:
	# 自行初始化 cards_node，跳过 CardContainer._ready() 中的 CardManager 查找
	if has_node("ScrollContainer/Cards"):
		cards_node = $ScrollContainer/Cards
	else:
		cards_node = Control.new()
		cards_node.name = "Cards"
		cards_node.mouse_filter = Control.MOUSE_FILTER_PASS
		if has_node("ScrollContainer"):
			$ScrollContainer.add_child(cards_node)
		else:
			add_child(cards_node)

	enable_drop_zone = false
	resized.connect(_on_resized)
	call_deferred("update_card_ui")

func _on_resized() -> void:
	update_card_ui()

func _update_target_positions() -> void:
	if _held_cards.is_empty() or scroll_container == null:
		return

	var available_width = scroll_container.size.x
	if available_width <= 0:
		return

	var card_width = (available_width - (columns - 1) * h_separation) / columns
	var card_height = card_width / DESIGN_ASPECT

	for i in range(_held_cards.size()):
		var card = _held_cards[i]
		var row = floori(i / float(columns))
		var col = i % columns
		var x = col * (card_width + h_separation)
		var y = row * (card_height + v_separation)
		card.position = Vector2(x, y)
		card.size = Vector2(card_width, card_height)
		card.custom_minimum_size = Vector2(card_width, card_height)
		if card is DuleCard:
			card._render_template()

	var rows = ceil(_held_cards.size() / float(columns))
	var total_height = rows * card_height + (rows - 1) * v_separation
	cards_node.custom_minimum_size = Vector2(available_width, total_height)
	cards_node.size = Vector2(available_width, total_height)

func update_card_ui() -> void:
	if _suppress_layout_count > 0:
		return
	super.update_card_ui()

func add_cards(cards: Array) -> void:
	_suppress_layout_count += 1
	for card in cards:
		if card is DuleCard:
			add_dule_card(card)
		else:
			add_card(card)
	_suppress_layout_count -= 1
	if _suppress_layout_count == 0:
		update_card_ui()

func add_dule_card(card: DuleCard) -> void:
	card.can_be_interacted_with = false
	card.gui_input.connect(_on_card_gui_input.bind(card))
	add_card(card)

func _on_card_gui_input(event: InputEvent, card: DuleCard) -> void:
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		card_clicked.emit(card.card_data)

func on_card_pressed(_card: Card) -> void:
	pass

func _card_can_be_added(_cards: Array) -> bool:
	return true
