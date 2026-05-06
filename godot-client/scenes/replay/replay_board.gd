# 简化棋盘
class_name ReplayBoard
extends Control

# ============================================================================
# 状态面板 (Label)
# ============================================================================
@onready var self_hp_label: Label = $VBox/SelfArea/HPRow/HPValue
@onready var self_rp_label: Label = $VBox/SelfArea/HPRow/RPValue
@onready var self_deck_label: Label = $VBox/SelfArea/HPRow/DeckValue
@onready var opp_hp_label: Label = $VBox/OppArea/HPRow/HPValue
@onready var opp_rp_label: Label = $VBox/OppArea/HPRow/RPValue
@onready var opp_deck_label: Label = $VBox/OppArea/HPRow/DeckValue

# ============================================================================
# 手牌 (HFlowContainer of Labels)
# ============================================================================
@onready var self_hand: Control = $VBox/SelfArea/HandRow/Cards
@onready var opp_hand: Control = $VBox/OppArea/HandRow/Cards

# ============================================================================
# 前/后场 (HBoxContainer of slot Labels)
# ============================================================================
@onready var self_front: HBoxContainer = $VBox/SelfArea/FieldRow/FrontSlots
@onready var self_back: HBoxContainer = $VBox/SelfArea/FieldRow/BackSlots
@onready var opp_front: HBoxContainer = $VBox/OppArea/FieldRow/FrontSlots
@onready var opp_back: HBoxContainer = $VBox/OppArea/FieldRow/BackSlots

# ============================================================================
# 费用区 / 墓地 (HFlowContainer of Labels)
# ============================================================================
@onready var self_cost: Control = $VBox/SelfArea/CostRow/Cards
@onready var self_grave: Control = $VBox/SelfArea/GraveRow/Cards
@onready var opp_cost: Control = $VBox/OppArea/CostRow/Cards
@onready var opp_grave: Control = $VBox/OppArea/GraveRow/Cards

# ============================================================================
# 卡片数据缓存 (instance_id → {definition_id, current_attack})
# ============================================================================
var _card_data: Dictionary = {}


# ============================================================================
# 公开 API — 更新各个 zone
# ============================================================================

func update_self_hp(val: int):
	self_hp_label.text = "HP:%d" % val

func update_self_rp(val: int):
	self_rp_label.text = "RP:%d" % val

func update_self_deck(val: int):
	self_deck_label.text = "卡组:%d" % val

func update_opp_hp(val: int):
	opp_hp_label.text = "HP:%d" % val

func update_opp_rp(val: int):
	opp_rp_label.text = "RP:%d" % val

func update_opp_deck(val: int):
	opp_deck_label.text = "卡组:%d" % val

func update_opp_hand_count(val: int):
	_clear_cards(opp_hand)
	for _i in range(min(val, 7)):
		var lbl := _make_card_label("[?]", -1, "?", 0)
		lbl.text = "[?]"
		opp_hand.add_child(lbl)


## 增加卡片到 zone 容器
func add_card_to_zone(container: Control, card_info: Dictionary, slot_idx: int = -1):
	var iid: int = card_info.get("instance_id", 0)
	var def_id: String = card_info.get("definition_id", "")
	var atk = card_info.get("current_attack", 0)
	_card_data[iid] = card_info

	var lbl := _make_card_label(def_id, iid, def_id, atk)
	if slot_idx >= 0 and container is HBoxContainer:
		# 替换 slot 位置的旧节点
		var old = container.get_child(slot_idx) if slot_idx < container.get_child_count() else null
		if old and old is Label:
			old.queue_free()
		# 插入到正确位置
		if slot_idx < container.get_child_count():
			container.add_child(lbl)
			container.move_child(lbl, slot_idx)
		else:
			# 填充空位
			while container.get_child_count() < slot_idx:
				var empty := Label.new()
				empty.text = "[ ]"
				container.add_child(empty)
			container.add_child(lbl)
	else:
		container.add_child(lbl)


## 从 zone 容器移除卡片（通过 instance_id）
func remove_card_from_zone(container: Control, instance_id: int):
	for child in container.get_children():
		if child is Label and child.get_meta("iid", 0) == instance_id:
			child.queue_free()
	_card_data.erase(instance_id)


## 清除整个 zone
func clear_zone(container: Control):
	_clear_cards(container)


# ============================================================================
# 内部辅助
# ============================================================================

func _make_card_label(def_id: String, iid: int, _def_id: String, _atk) -> Label:
	var lbl := Label.new()
	# 显示缩写名
	var short = def_id.get_slice("-", 2)  # 取 S000-C-001 的最后一段
	if short == "" or short == def_id:
		short = def_id
	lbl.text = "[%s]" % short
	lbl.set_meta("iid", iid)
	lbl.set_meta("def_id", def_id)
	var atk_val = _atk
	if typeof(atk_val) != TYPE_INT:
		atk_val = 0
	lbl.set_meta("atk", atk_val)
	lbl.tooltip_text = "iid=%d  def=%s  atk=%d" % [iid, def_id, atk_val]
	lbl.mouse_filter = Control.MOUSE_FILTER_PASS
	return lbl


func _clear_cards(container: Control):
	for child in container.get_children():
		if child is Label:
			child.queue_free()
