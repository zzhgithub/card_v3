# 回放系统核心类型
# 纯数据处理，不依赖任何 UI 节点
## 回放系统核心类型
## 纯数据处理，不依赖任何 UI 节点

# ============================================================================
# ZoneChange — 单个 zone 的差异结果
# ============================================================================
class ZoneChange:
	var zone_name: String           ## "hand", "front", "back", "cost_zone", "grave", "deck"
	var player_side: String         ## "self" or "opponent"
	var adds: Array[Dictionary]     ## 新增的卡片 info [{instance_id, definition_id, current_attack}, ...]
	var removes: Array[int]         ## 要移除的 instance_id 列表
	var keeps: Array[int]           ## 保持不变的 instance_id 列表
	var slot_index: int = -1        ## 仅 front/back: 变化的 slot 索引（-1 表示整个区域）

	func _init(p_zone: String, p_side: String):
		zone_name = p_zone
		player_side = p_side
		adds = []
		removes = []
		keeps = []

	func has_changes() -> bool:
		return not adds.is_empty() or not removes.is_empty()

	func format_change() -> String:
		var label = "%s %s" % [zone_name.to_upper(), player_side]
		var parts: Array[String] = []
		if not adds.is_empty():
			var id_strs: Array[String] = []
			for a in adds:
				id_strs.append(str(a.get("instance_id", 0)))
			parts.append("ADD [%s]" % ",".join(id_strs))
		if not removes.is_empty():
			var rm_strs: Array[String] = []
			for r in removes:
				rm_strs.append(str(r))
			parts.append("REMOVE [%s]" % ",".join(rm_strs))
		if not keeps.is_empty():
			var kp_strs: Array[String] = []
			for k in keeps:
				kp_strs.append(str(k))
			parts.append("KEEP [%s]" % ",".join(kp_strs))
		if parts.is_empty():
			return "%s: (no change)" % label
		return "%s: %s" % [label, " | ".join(parts)]


# ============================================================================
# DiffEngine — 比较新旧 state，生成 ZoneChange 列表
# ============================================================================
class DiffEngine:

	## 完整 diff：比较 prev_state 和 new_state，返回所有变化的 zone
	static func compare(prev_state: Dictionary, new_state: Dictionary) -> Array[ZoneChange]:
		var changes: Array[ZoneChange] = []

		# 顶层字段
		changes.append_array(_diff_scalar(prev_state, new_state, "turn_number", "INFO"))
		changes.append_array(_diff_scalar(prev_state, new_state, "current_phase", "INFO"))
		changes.append_array(_diff_scalar(prev_state, new_state, "current_player", "INFO"))

		# your_state
		var prev_ys = prev_state.get("your_state", {})
		var new_ys = new_state.get("your_state", {})
		changes.append_array(_diff_zone(prev_ys, new_ys, "hand", "self"))
		changes.append_array(_diff_field_slots(prev_ys, new_ys, "front", "self"))
		changes.append_array(_diff_field_slots(prev_ys, new_ys, "back", "self"))
		changes.append_array(_diff_zone(prev_ys, new_ys, "cost_zone", "self"))
		changes.append_array(_diff_zone(prev_ys, new_ys, "grave", "self"))
		changes.append_array(_diff_scalar(prev_ys, new_ys, "deck_count", "DECK"))
		changes.append_array(_diff_scalar(prev_ys, new_ys, "hp", "HP"))
		changes.append_array(_diff_scalar(prev_ys, new_ys, "real_point", "RP"))

		# opponent_state
		var prev_os = prev_state.get("opponent_state", {})
		var new_os = new_state.get("opponent_state", {})
		changes.append_array(_diff_zone(prev_os, new_os, "hand", "opponent"))
		changes.append_array(_diff_field_slots(prev_os, new_os, "front", "opponent"))
		changes.append_array(_diff_field_slots(prev_os, new_os, "back", "opponent"))
		changes.append_array(_diff_zone(prev_os, new_os, "cost_zone", "opponent"))
		changes.append_array(_diff_zone(prev_os, new_os, "grave", "opponent"))
		changes.append_array(_diff_scalar(prev_os, new_os, "deck_count", "DECK"))
		changes.append_array(_diff_scalar(prev_os, new_os, "hp", "HP"))
		changes.append_array(_diff_scalar(prev_os, new_os, "hand_count", "HAND"))

		return changes


	## 比较单个 zone（数组类型：hand, cost_zone, grave）
	static func _diff_zone(prev: Dictionary, new: Dictionary, zone_key: String, side: String) -> Array[ZoneChange]:
		var prev_arr: Array = prev.get(zone_key, [])
		var new_arr: Array = new.get(zone_key, [])
		if typeof(prev_arr) != TYPE_ARRAY: prev_arr = []
		if typeof(new_arr) != TYPE_ARRAY: new_arr = []

		var change := ZoneChange.new(zone_key, side)

		# 构建 iid 集合
		var prev_ids: Array[int] = []
		for c in prev_arr:
			if typeof(c) == TYPE_DICTIONARY:
				prev_ids.append(c.get("instance_id", 0))

		var new_ids: Array[int] = []
		for c in new_arr:
			if typeof(c) == TYPE_DICTIONARY:
				new_ids.append(c.get("instance_id", 0))

		# 找出新增的
		for c in new_arr:
			if typeof(c) != TYPE_DICTIONARY:
				continue
			var iid = c.get("instance_id", 0)
			if iid not in prev_ids:
				change.adds.append(c)

		# 找出移除的
		for iid in prev_ids:
			if iid not in new_ids:
				change.removes.append(iid)

		# 保持的
		for iid in prev_ids:
			if iid in new_ids:
				change.keeps.append(iid)

		return [change]


	## 比较前/后场（固定 5 格数组）
	static func _diff_field_slots(prev: Dictionary, new: Dictionary, zone_key: String, side: String) -> Array[ZoneChange]:
		var prev_arr: Array = prev.get(zone_key, [])
		var new_arr: Array = new.get(zone_key, [])
		if typeof(prev_arr) != TYPE_ARRAY: prev_arr = []
		if typeof(new_arr) != TYPE_ARRAY: new_arr = []

		var changes: Array[ZoneChange] = []

		for i in range(5):
			var prev_card = prev_arr[i] if i < prev_arr.size() else null
			var new_card = new_arr[i] if i < new_arr.size() else null

			var prev_iid = prev_card.get("instance_id", 0) if typeof(prev_card) == TYPE_DICTIONARY else 0
			var new_iid = new_card.get("instance_id", 0) if typeof(new_card) == TYPE_DICTIONARY else 0

			if prev_iid != new_iid:
				var change := ZoneChange.new(zone_key, side)
				change.slot_index = i
				if prev_iid != 0:
					change.removes.append(prev_iid)
				if new_iid != 0:
					change.adds.append(new_card)
				changes.append(change)

		return changes


	## 比较标量值（hp, rp, deck_count, turn_number 等）
	static func _diff_scalar(prev: Dictionary, new: Dictionary, key: String, label: String) -> Array[ZoneChange]:
		var p = prev.get(key)
		var n = new.get(key)
		if p == n:
			return []
		var change := ZoneChange.new(label, "")
		change.adds.append({"value": n})  # 复用 adds 来传递新值
		return [change]


# ============================================================================
# VerifyLogger — 格式化输出 diff 结果
# ============================================================================
class VerifyLogger:

	static var _msg_index: int = 0
	static var _total: int = 0

	static func set_context(msg_idx: int, total: int):
		_msg_index = msg_idx
		_total = total

	## 打印所有 diff 变化
	static func log_changes(changes: Array[ZoneChange]):
		print("[VERIFY] ─── Message %d/%d ───" % [_msg_index + 1, _total])
		for ch in changes:
			print("[VERIFY] %s" % ch.to_string())

	## 打印动画执行
	static func log_animation(event_type: String, instance_id: int, extra: String = ""):
		var suffix = " " + extra if extra != "" else ""
		print("[ANIM] %s iid=%d%s" % [event_type, instance_id, suffix])

	## 打印跳过
	static func log_skip(event_type: String, reason: String = ""):
		print("[ANIM] SKIP %s: %s" % [event_type, reason])

	## 打印通过/失败
	static func log_pass(scenario: String, check: String):
		print("[PASS] %s: %s" % [scenario, check])

	static func log_fail(scenario: String, check: String, expected: String, actual: String):
		print("[FAIL] %s: %s | expected='%s' actual='%s'" % [scenario, check, expected, actual])
