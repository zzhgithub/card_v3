## AnimationManager — 独立动画类，回放和真实对局共用
## 每个动画方法接受具体的节点引用和参数，不读取全局状态
extends RefCounted

var _board  # GameBoard 引用


func _init(board):
	_board = board


# ============================================================================
# 公开入口：处理 recent_events 数组
# ============================================================================

func play_events(events: Array):
	if events.is_empty():
		return
	for event in events:
		if typeof(event) != TYPE_DICTIONARY:
			continue
		for event_type in event.keys():
			_play_one(event_type, event[event_type])


func _play_one(event_type: String, data):
	var iid: int = _safe_int(data, "instance_id")

	match event_type:
		"DrawCard":
			play_draw_card(iid)

		"CardSummoned":
			play_summon_drop(iid)

		"CardExposed":
			play_cost_flip(iid)

		"CardMoved":
			play_card_moved(iid, data)

		"CardDestroyed":
			play_destroy_shrink(iid)

		"HpChanged":
			play_hp_flash(data)

		"PhaseChanged":
			play_phase_change(data)

		"AttackDeclared":
			play_attack_shake(iid)

		_:
			print("[ANIM] unhandled event: %s" % event_type)


# ============================================================================
# 抽卡动画 — 原地淡入
# ============================================================================

func play_draw_card(instance_id: int):
	var card = _find_card_in_hand(instance_id)
	if not card:
		print("[ANIM] SKIP draw: card iid=%d not in hand" % instance_id)
		return

	card.modulate = Color(1, 1, 1, 0.0)
	var tw = _board.create_tween()
	tw.tween_property(card, "modulate", Color.WHITE, 0.3)
	print("[ANIM] draw fade-in iid=%d" % instance_id)


# ============================================================================
# 登场动画 — 拍落效果（从上方掉落 + 弹性）
# ============================================================================

func play_summon_drop(instance_id: int):
	var card = _find_card_on_field(instance_id)
	if not card:
		print("[ANIM] SKIP summon: card iid=%d not on field" % instance_id)
		return

	# 卡已在 BattleFieldSlot 中正确定位，只做视觉效果，不移动位置
	card.scale = Vector2(0.6, 0.6)
	card.modulate = Color(1, 1, 1, 0.2)

	var tw = _board.create_tween()
	tw.set_parallel()
	# 缩放弹入
	tw.tween_property(card, "scale", Vector2(1.0, 1.0), 0.35) \
		.set_ease(Tween.EASE_OUT).set_trans(Tween.TRANS_BACK)
	# 淡入
	tw.tween_property(card, "modulate", Color.WHITE, 0.25)

	print("[ANIM] summon drop iid=%d" % instance_id)

# ============================================================================
# 费用支付动画 — 翻转飞入（旋转 + 缩放 + 透明）
# ============================================================================

func play_cost_flip(instance_id: int):
	var card = _find_card_in_cost_zone(instance_id)
	if not card:
		print("[ANIM] SKIP cost-flip: card iid=%d not in cost zone" % instance_id)
		return

	# 起始：从手牌方向飞入（左偏移 + 旋转90度 + 缩小 + 半透明）
	card.position.x -= 120
	card.rotation = deg_to_rad(90)
	card.scale = Vector2(0.6, 0.6)
	card.modulate = Color(1, 1, 1, 0.2)

	var tw = _board.create_tween()
	tw.set_parallel()
	# 位移动画
	tw.tween_property(card, "position:x", card.position.x + 120, 0.35) \
		.set_ease(Tween.EASE_OUT)
	# 旋转还原
	tw.tween_property(card, "rotation", 0.0, 0.3) \
		.set_ease(Tween.EASE_OUT)
	# 缩放还原
	tw.tween_property(card, "scale", Vector2(1.0, 1.0), 0.25) \
		.set_ease(Tween.EASE_OUT)
	# 淡入
	tw.tween_property(card, "modulate", Color.WHITE, 0.2)

	print("[ANIM] cost flip iid=%d" % instance_id)


# ============================================================================
# 卡牌移动动画（进墓地等）
# ============================================================================

func play_card_moved(instance_id: int, data):
	var to_zone = data.get("to", {})
	if typeof(to_zone) != TYPE_DICTIONARY:
		return
	var zone_name = to_zone.get("zone", "")
	# 如果移动到墓地，做灰度淡入
	if zone_name == "Grave":
		var card = _find_card_in_grave(instance_id)
		if card:
			card.modulate = Color(0.5, 0.5, 0.5, 0.4)
			var tw = _board.create_tween()
			tw.tween_property(card, "modulate", Color.WHITE, 0.3)
			print("[ANIM] grave enter iid=%d" % instance_id)


# ============================================================================
# 破坏动画 — 缩小 + 红色淡出
# ============================================================================

func play_destroy_shrink(instance_id: int):
	var card = _find_card_on_field(instance_id)
	if not card:
		print("[ANIM] SKIP destroy: card iid=%d not on field" % instance_id)
		return

	var tw = _board.create_tween()
	tw.set_parallel()
	tw.tween_property(card, "scale", Vector2(0.2, 0.2), 0.35) \
		.set_ease(Tween.EASE_IN)
	tw.tween_property(card, "modulate", Color(1, 0.2, 0.2, 0.0), 0.35)

	print("[ANIM] destroy shrink iid=%d" % instance_id)


# ============================================================================
# HP 变化 — 标签红色闪烁
# ============================================================================

func play_hp_flash(data):
	var player = data.get("player", "")
	var old_hp = data.get("old_hp", 0)
	var new_hp = data.get("new_hp", 0)

	# 找到对应的 HP 标签
	var label: Label = null
	if player == "Player1":
		label = _board.get_node_or_null("PlayerArea/PlayerBottomRow/PlayerStatus/HPLabel")
	else:
		label = _board.get_node_or_null("OpponentArea/OpponentTopRow/OpponentStatus/HPLabel")

	if label:
		label.modulate = Color.RED
		var tw = _board.create_tween()
		tw.tween_property(label, "modulate", Color.WHITE, 0.35)

	print("[ANIM] HP %s: %d → %d" % [player, old_hp, new_hp])


# ============================================================================
# 阶段切换 — 标签淡入
# ============================================================================

func play_phase_change(data):
	var new_phase = data.get("new_phase", "")
	print("[ANIM] phase → %s" % new_phase)


# ============================================================================
# 攻击宣言 — 攻击方轻微抖动
# ============================================================================

func play_attack_shake(instance_id: int):
	if instance_id <= 0:
		return
	var card = _find_card_on_field(instance_id)
	if not card:
		return
	var orig_pos = card.position
	var tw = _board.create_tween()
	# 水平抖动：左右快速移动 3 次
	tw.tween_property(card, "position:x", orig_pos.x + 6, 0.05)
	tw.tween_property(card, "position:x", orig_pos.x - 6, 0.05)
	tw.tween_property(card, "position:x", orig_pos.x + 4, 0.04)
	tw.tween_property(card, "position:x", orig_pos.x, 0.04)
	print("[ANIM] attack shake iid=%d" % instance_id)


# ============================================================================
# 卡片查找辅助方法
# ============================================================================

func _find_card_in_hand(instance_id: int):
	return _search_zone("hand", instance_id)

func _find_card_on_field(instance_id: int):
	var c = _search_zone("front", instance_id)
	if c: return c
	return _search_zone("back", instance_id)

func _find_card_in_cost_zone(instance_id: int):
	return _search_zone("cost_zone", instance_id)

func _find_card_in_grave(instance_id: int):
	return _search_zone("grave", instance_id)

## 递归搜索容器子树，按 instance_id 的 meta 查找卡牌
func _search_zone(zone_name: String, instance_id: int):
	var self_container = _get_zone_container(zone_name, "self")
	if self_container:
		var found = _search_children(self_container, instance_id)
		if found: return found

	var opp_container = _get_zone_container(zone_name, "opponent")
	if opp_container:
		var found = _search_children(opp_container, instance_id)
		if found: return found

	return null


## 递归遍历所有子节点，通过 meta("instance_id") 匹配
func _search_children(node, instance_id: int):
	for child in node.get_children():
		if child.get_meta("instance_id", 0) == instance_id:
			return child
		var found = _search_children(child, instance_id)
		if found:
			return found
	return null


func _get_zone_container(zone_name: String, side: String):
	var board = _board
	var key = side + "/" + zone_name
	if key == "self/hand":
		return board.get_node_or_null("PlayerArea/PlayerBottomRow/PlayerHand")
	if key == "opponent/hand":
		return board.get_node_or_null("OpponentArea/OpponentTopRow/OpponentHand")
	if key == "self/front":
		return board.get_node_or_null("PlayerArea/PlayerMainRow/PlayerBattleField/PlayerFrontField")
	if key == "opponent/front":
		return board.get_node_or_null("OpponentArea/OpponentMainRow/OpponentBattleField/OpponentFrontField")
	if key == "self/back":
		return board.get_node_or_null("PlayerArea/PlayerMainRow/PlayerBattleField/PlayerBackField")
	if key == "opponent/back":
		return board.get_node_or_null("OpponentArea/OpponentMainRow/OpponentBattleField/OpponentBackField")
	if key == "self/cost_zone":
		return board.get_node_or_null("PlayerArea/PlayerMainRow/PlayerCost")
	if key == "opponent/cost_zone":
		return board.get_node_or_null("OpponentArea/OpponentMainRow/OpponentCost")
	if key == "self/grave":
		return board.get_node_or_null("PlayerArea/PlayerMainRow/PlayerStacks/PlayerGrave")
	if key == "opponent/grave":
		return board.get_node_or_null("OpponentArea/OpponentMainRow/OpponentStacks/OpponentGrave")
	return null


func _safe_int(d, key: String) -> int:
	if typeof(d) != TYPE_DICTIONARY:
		return 0
	var v = d.get(key, 0)
	return int(v) if typeof(v) in [TYPE_INT, TYPE_FLOAT] else 0
