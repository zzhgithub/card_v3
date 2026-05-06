## 回放控制器 v3 — 代码构建 UI，数据驱动，可验证
extends Control

const Types = preload("res://scenes/replay/replay_types.gd")

# ============================================================================
# 棋盘标签引用 (动态创建)
# ============================================================================
var self_hp_lbl: Label
var self_rp_lbl: Label
var self_deck_lbl: Label
var opp_hp_lbl: Label
var opp_rp_lbl: Label
var opp_deck_lbl: Label
var self_hand: Control
var opp_hand: Control
var self_front: Control
var self_back: Control
var opp_front: Control
var opp_back: Control
var self_cost: Control
var self_grave: Control
var opp_cost: Control
var opp_grave: Control

# 控制栏
var file_selector: OptionButton
var play_pause_btn: Button
var step_btn: Button
var speed_btn: Button
var progress_slider: HSlider
var message_label: Label
var jump_input: LineEdit
var jump_btn: Button
var detail_title: Label
var detail_json: TextEdit
var detail_diff: TextEdit
var console_log: TextEdit

# ============================================================================
# 回放状态
# ============================================================================
var _messages: Array = []
var _current_index: int = 0
var _is_playing: bool = false
var _speed_multiplier: float = 1.0
var _pending_delay: float = 0.0
var _prev_state: Dictionary = {}
var _scenario_data: Dictionary = {}
var _card_data: Dictionary = {}

var _replay_files: Array = [
	{"name": "--- 场景测试 ---", "path": "", "disabled": true},
	{"name": "1. 初始状态", "path": "res://assets/replay/test_scenarios/scenario_01.json"},
	{"name": "2. 抽卡", "path": "res://assets/replay/test_scenarios/scenario_02.json"},
	{"name": "3. 登场(前场)+费用", "path": "res://assets/replay/test_scenarios/scenario_03.json"},
	{"name": "4. 登场(后场)+费用", "path": "res://assets/replay/test_scenarios/scenario_04.json"},
	{"name": "5. 对手前场登场", "path": "res://assets/replay/test_scenarios/scenario_05.json"},
	{"name": "6. 攻击+破坏+HP", "path": "res://assets/replay/test_scenarios/scenario_06.json"},
	{"name": "7. 自己被攻击破坏", "path": "res://assets/replay/test_scenarios/scenario_07.json"},
	{"name": "8. 回合切换+P2抽卡", "path": "res://assets/replay/test_scenarios/scenario_08.json"},
	{"name": "9. RP变化", "path": "res://assets/replay/test_scenarios/scenario_09.json"},
	{"name": "10. GameOver", "path": "res://assets/replay/test_scenarios/scenario_10.json"},
	{"name": "11. 多事件混合", "path": "res://assets/replay/test_scenarios/scenario_11.json"},
	{"name": "12. 费用区多卡", "path": "res://assets/replay/test_scenarios/scenario_12.json"},
]
const SPEEDS: Array[float] = [1.0, 5.0, 10.0]


func _ready():
	_build_ui()
	for f in _replay_files:
		if f.get("disabled", false): file_selector.add_separator(f["name"])
		else: file_selector.add_item(f["name"])
	file_selector.select(1)
	play_pause_btn.pressed.connect(_on_play_pause)
	step_btn.pressed.connect(_on_step)
	speed_btn.pressed.connect(_on_speed_toggle)
	progress_slider.value_changed.connect(_on_slider_changed)
	progress_slider.drag_ended.connect(_on_slider_drag_ended)
	jump_btn.pressed.connect(_on_jump)
	file_selector.item_selected.connect(_on_file_selected)
	_load_replay(_replay_files[1]["path"])


# ============================================================================
# UI 构建 (all in code)
# ============================================================================
func _build_ui():
	_build_control_bar()
	_build_console()
	_build_main_area()

func _build_control_bar():
	var bar = Panel.new(); bar.name = "ControlBar"
	bar.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	bar.offset_top = -50.0; bar.grow_vertical = Control.GROW_DIRECTION_BEGIN
	add_child(bar)
	var hbox = HBoxContainer.new(); hbox.name = "HBox"
	hbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	hbox.offset_left = 8; hbox.offset_top = 10; hbox.offset_right = -8; hbox.offset_bottom = -10
	hbox.add_theme_constant_override("separation", 6)
	bar.add_child(hbox)
	file_selector = OptionButton.new(); file_selector.custom_minimum_size = Vector2(150, 30)
	hbox.add_child(file_selector)
	play_pause_btn = _btn("u", 40); step_btn = _btn("⏭", 40); speed_btn = _btn("1x", 46)
	hbox.add_child(play_pause_btn); hbox.add_child(step_btn); hbox.add_child(speed_btn)
	progress_slider = HSlider.new(); progress_slider.size_flags_horizontal = 3
	progress_slider.custom_minimum_size = Vector2(80, 30); progress_slider.max_value = 100; progress_slider.step = 1
	hbox.add_child(progress_slider)
	message_label = Label.new(); message_label.custom_minimum_size = Vector2(180, 30)
	message_label.text = "[0/0] --"; message_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	message_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER; message_label.clip_text = true
	hbox.add_child(message_label)
	jump_input = LineEdit.new(); jump_input.custom_minimum_size = Vector2(40, 30); jump_input.placeholder_text = "#"
	hbox.add_child(jump_input)
	jump_btn = _btn("go", 44); hbox.add_child(jump_btn)

func _build_console():
	var panel = Panel.new(); panel.name = "ConsolePanel"
	panel.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	panel.offset_top = -212; panel.offset_bottom = -52; panel.grow_vertical = Control.GROW_DIRECTION_BEGIN
	add_child(panel)
	var vbox = VBoxContainer.new(); vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	vbox.offset_left = 8; vbox.offset_top = 6; vbox.offset_right = -8; vbox.offset_bottom = -6
	panel.add_child(vbox)
	var lbl = Label.new(); lbl.text = "i: logs:"; lbl.add_theme_color_override("font_color", Color(0.5, 0.8, 0.5))
	vbox.add_child(lbl)
	console_log = TextEdit.new(); console_log.size_flags_horizontal = 3; console_log.size_flags_vertical = 3
	console_log.editable = false
	vbox.add_child(console_log)

func _build_main_area():
	var hbox = HBoxContainer.new(); hbox.name = "MainArea"
	hbox.set_anchors_preset(Control.PRESET_FULL_RECT); hbox.offset_bottom = -52
	hbox.grow_vertical = Control.GROW_DIRECTION_BEGIN; hbox.add_theme_constant_override("separation", 0)
	add_child(hbox)
	# 棋盘
	var bc = Control.new(); bc.name = "Board"
	bc.size_flags_horizontal = 3; bc.size_flags_vertical = 3; hbox.add_child(bc)
	_build_board(bc)
	# 详情
	var detail = Panel.new(); detail.name = "Detail"; detail.custom_minimum_size = Vector2(320, 0)
	detail.size_flags_vertical = 3; hbox.add_child(detail)
	var dv = VBoxContainer.new(); dv.set_anchors_preset(Control.PRESET_FULL_RECT)
	dv.offset_left = 8; dv.offset_top = 8; dv.offset_right = -8; dv.offset_bottom = -8
	dv.add_theme_constant_override("separation", 6); detail.add_child(dv)
	detail_title = Label.new(); detail_title.text = "[0/0]"; detail_title.clip_text = true
	detail_title.add_theme_color_override("font_color", Color(0.35, 0.65, 0.85)); dv.add_child(detail_title)
	dv.add_child(_slbl("Data:")); detail_json = _tedit(150); dv.add_child(detail_json)
	dv.add_child(_slbl("Diff:")); detail_diff = _tedit(100); dv.add_child(detail_diff)

func _build_board(parent: Control):
	var sv = ScrollContainer.new(); sv.name = "Scroll"
	sv.set_anchors_preset(Control.PRESET_FULL_RECT); sv.horizontal_scroll_mode = 0; parent.add_child(sv)
	var vbox = VBoxContainer.new(); vbox.size_flags_horizontal = 3
	vbox.add_theme_constant_override("separation", 6); sv.add_child(vbox)
	_build_player_area(vbox, "opponent", Color(1,0.5,0.5))
	vbox.add_child(_sep())
	_build_player_area(vbox, "self", Color(0.5,0.8,1))

func _build_player_area(parent: Control, side: String, tc: Color):
	var area = VBoxContainer.new(); area.add_theme_constant_override("separation", 3); parent.add_child(area)
	var hprow = HBoxContainer.new(); hprow.add_theme_constant_override("separation", 12); area.add_child(hprow)
	var t = Label.new(); t.text = "opp" if side == "opponent" else "you"
	t.add_theme_color_override("font_color", tc); hprow.add_child(t)
	var hp = Label.new(); hp.text = "HP:5"; hprow.add_child(hp)
	var rp = Label.new(); rp.text = "RP:0"; hprow.add_child(rp)
	var dk = Label.new(); dk.text = "deck:35"; hprow.add_child(dk)
	if side == "self": self_hp_lbl = hp; self_rp_lbl = rp; self_deck_lbl = dk
	else: opp_hp_lbl = hp; opp_rp_lbl = rp; opp_deck_lbl = dk
	for zn in [["hand:", "hand"], ["front:", "front"], ["back:", "back"], ["cost:", "cost_zone"], ["grave:", "grave"]]:
		_build_zone_row(area, zn[0], side, zn[1])

func _build_zone_row(parent: Control, label: String, side: String, zone: String):
	var row = HBoxContainer.new(); row.add_theme_constant_override("separation", 4); parent.add_child(row)
	row.add_child(_slbl(label))
	var c: Control
	if zone == "front" or zone == "back":
		c = HBoxContainer.new(); c.add_theme_constant_override("separation", 4)
		for _i in range(5): c.add_child(_slbl("[ ]"))
	else: c = HFlowContainer.new()
	c.size_flags_horizontal = 3; row.add_child(c)
	var key = side + "/" + zone
	if key == "self/hand": self_hand = c
	elif key == "self/front": self_front = c
	elif key == "self/back": self_back = c
	elif key == "self/cost_zone": self_cost = c
	elif key == "self/grave": self_grave = c
	elif key == "opponent/hand": opp_hand = c
	elif key == "opponent/front": opp_front = c
	elif key == "opponent/back": opp_back = c
	elif key == "opponent/cost_zone": opp_cost = c
	elif key == "opponent/grave": opp_grave = c

func _btn(txt: String, w: float) -> Button:
	var b = Button.new(); b.text = txt; b.custom_minimum_size = Vector2(w, 30); return b
func _slbl(txt: String) -> Label:
	var l = Label.new(); l.text = txt; return l
func _tedit(h: float) -> TextEdit:
	var t = TextEdit.new(); t.size_flags_horizontal = 3; t.size_flags_vertical = 1
	t.custom_minimum_size = Vector2(0, h); t.editable = false
	return t
func _sep() -> HSeparator: return HSeparator.new()

func _mk_label(def_id: String, iid: int, atk) -> Label:
	var l = Label.new()
	var s = def_id.get_slice("-", 2)
	if s == "" or s == def_id: s = def_id
	l.text = "[%s]" % s
	l.set_meta("iid", iid); l.set_meta("def_id", def_id)
	var av = atk; if typeof(av) != TYPE_INT: av = 0
	l.set_meta("atk", av); l.tooltip_text = "iid=%d def=%s atk=%d" % [iid, def_id, av]
	return l


# ============================================================================
# 棋盘操作
# ============================================================================
func _upd(v, fn: Callable): fn.call(v)
func _add_card(container: Control, card_info: Dictionary, slot_idx: int = -1):
	var iid: int = card_info.get("instance_id", 0)
	var def_id: String = card_info.get("definition_id", "")
	var atk = card_info.get("current_attack", 0); _card_data[iid] = card_info
	var lbl = _mk_label(def_id, iid, atk)
	if slot_idx >= 0 and container is HBoxContainer:
		if slot_idx < container.get_child_count():
			var old = container.get_child(slot_idx)
			if old is Label and old.get_meta("iid", 0) <= 0: old.queue_free()
		container.add_child(lbl); container.move_child(lbl, min(slot_idx, container.get_child_count()-1))
	else: container.add_child(lbl)

func _rm_card(container: Control, iid: int):
	for c in container.get_children():
		if c is Label and c.get_meta("iid", 0) == iid: c.queue_free()
	_card_data.erase(iid)

func _clear(container: Control):
	for c in container.get_children():
		if c is Label: c.queue_free()

func _upd_opp_hand(v: int):
	_clear(opp_hand)
	for _i in range(min(v, 7)): opp_hand.add_child(_mk_label("?", -1, 0))


# ============================================================================
# 渲染 (inline)
# ============================================================================
func _apply_changes(changes: Array):
	for ch in changes:
		var zn = ch.zone_name; var sd = ch.player_side
		var adds: Array = ch.adds; var rms: Array = ch.removes; var si = ch.slot_index
		var ctr = _zone_ctr(zn, sd); var iss = (sd == "self")
		if zn in ["front","back"]:
			if si >= 0: for a in adds: _add_card(ctr, a, si)
			for r in rms: _rm_card(ctr, r)
		elif zn in ["hand","cost_zone","grave"]:
			for a in adds: _add_card(ctr, a)
			for r in rms: _rm_card(ctr, r)
		elif zn == "HP":
			var val = adds[0].get("value", 0)
			if iss: self_hp_lbl.text = "HP:%d" % val
			else: opp_hp_lbl.text = "HP:%d" % val
		elif zn == "RP":
			var val = adds[0].get("value", 0)
			if iss: self_rp_lbl.text = "RP:%d" % val
			else: opp_rp_lbl.text = "RP:%d" % val
		elif zn == "DECK":
			var val = adds[0].get("value", 0)
			if iss: self_deck_lbl.text = "deck:%d" % val
			else: opp_deck_lbl.text = "deck:%d" % val
		elif zn == "HAND":
			_upd_opp_hand(adds[0].get("value", 0))

func _zone_ctr(zn: String, sd: String) -> Control:
	var key = sd + "/" + zn
	if key == "self/hand": return self_hand
	if key == "opponent/hand": return opp_hand
	if key == "self/front": return self_front
	if key == "opponent/front": return opp_front
	if key == "self/back": return self_back
	if key == "opponent/back": return opp_back
	if key == "self/cost_zone": return self_cost
	if key == "opponent/cost_zone": return opp_cost
	if key == "self/grave": return self_grave
	if key == "opponent/grave": return opp_grave
	return self_hand


# ============================================================================
# 动画 (inline)
# ============================================================================
func _play_events(events: Array):
	if events.is_empty(): return
	for ev in events:
		if typeof(ev) != TYPE_DICTIONARY: continue
		for et in ev.keys():
			var d = ev[et]; var iid = d.get("instance_id",0) if typeof(d)==TYPE_DICTIONARY else 0
			match et:
				"DrawCard":
					var c = _fl(iid)
					if c: c.modulate=Color(1,1,1,0); var tw=create_tween(); tw.tween_property(c,"modulate",Color.WHITE,0.3)
				"CardSummoned":
					var c = _fl(iid)
					if c: c.scale=Vector2(0.6,0.6); c.modulate=Color(1,1,1,0.2); var tw=create_tween(); tw.set_parallel(); tw.tween_property(c,"scale",Vector2(1,1),0.3).set_ease(Tween.EASE_OUT).set_trans(Tween.TRANS_BACK); tw.tween_property(c,"modulate",Color.WHITE,0.2)
				"CardExposed":
					var c = _fl(iid)
					if c: c.rotation=deg_to_rad(90); c.scale=Vector2(0.6,0.6); var tw=create_tween(); tw.set_parallel(); tw.tween_property(c,"rotation",0.0,0.25); tw.tween_property(c,"scale",Vector2(1,1),0.25)
				"CardDestroyed":
					var c = _fl(iid)
					if c: var tw=create_tween(); tw.set_parallel(); tw.tween_property(c,"scale",Vector2(0.3,0.3),0.3); tw.tween_property(c,"modulate",Color(1,0.3,0.3,0),0.3)

func _fl(iid: int) -> Control:
	for z in [self_hand,opp_hand,self_front,opp_front,self_back,opp_back,self_cost,opp_cost,self_grave,opp_grave]:
		if z == null: continue
		for c in z.get_children():
			if c is Label and c.get_meta("iid",0) == iid: return c
	return null


# ============================================================================
# 回放逻辑
# ============================================================================
func _load_replay(path: String):
	var f = FileAccess.open(path, FileAccess.READ)
	if not f: push_error("open fail: %s"%path); return
	var d = JSON.parse_string(f.get_as_text()); f.close()
	if d == null: push_error("parse fail"); return
	_scenario_data = d; _messages = d.get("messages",[])
	_is_playing = false; play_pause_btn.text = "u"
	_current_index = 0; _prev_state = {}
	progress_slider.max_value = max(0, _messages.size()-1); progress_slider.value = 0
	console_log.text = ""; _seek(0); _update_ui(); _update_detail()
	_log("[LOAD] %s" % d.get("scenario", path))

func _process(delta: float):
	if not _is_playing: return
	_pending_delay -= delta * _speed_multiplier
	while _pending_delay <= 0.0 and _current_index < _messages.size():
		_advance(); if _pending_delay > 0.0: break

func _advance():
	if _current_index >= _messages.size(): return
	var m: Dictionary = _messages[_current_index]
	var mt = m.get("type",""); var md = m.get("data",{})
	match mt:
		"state_update": _step_state(md)
		"game_over":
			_log("[END] GameOver"); _is_playing = false; play_pause_btn.text = "u"
			_current_index = 0; _prev_state = {}; progress_slider.value = 0
			_seek(0); _update_ui(); _update_detail(); _run_verify(); return
		_: _log("[SKIP] %s"%mt)
	_current_index += 1; _update_ui(); _update_detail()
	if _current_index < _messages.size():
		_pending_delay = float(_messages[_current_index].get("delay_ms",0)) / 1000.0

func _step_state(sd: Dictionary):
	var evts = sd.get("recent_events",[])
	if typeof(evts) != TYPE_ARRAY: evts = []
	var changes = Types.DiffEngine.compare(_prev_state, sd)
	_log("--- Msg %d/%d ---" % [_current_index+1, _messages.size()])
	var dt = ""
	for ch in changes:
		var ln = ch.format_change(); _log("[VERIFY] %s"%ln); dt += ln + "\n"
	detail_diff.text = dt
	_apply_changes(changes)
	_play_events(evts)
	_prev_state = sd.duplicate(true)
	_run_verify()

func _run_verify():
	var v = _scenario_data.get("verification",{})
	var cs: Array = v.get("expected_checks",[])
	if cs.is_empty(): return
	_log("-".repeat(40))
	for ck in cs:
		if detail_diff.text.find(ck) != -1: _log("[PASS] %s"%ck)
		else: _log("[FAIL] %s"%ck)
	_log("-".repeat(40))

func _seek(target: int):
	_prev_state = {}
	for i in range(min(target, _messages.size()-1), -1, -1):
		var m = _messages[i]
		if m.get("type") == "state_update":
			var d = m.get("data",{})
			var chs = Types.DiffEngine.compare(_prev_state, d)
			_apply_changes(chs); _prev_state = d.duplicate(true); break
	_current_index = target

func _update_ui():
	if _messages.is_empty(): return
	var i = min(_current_index, _messages.size()-1)
	message_label.text = "[%d/%d] %s" % [i+1, _messages.size(), _messages[i].get("type","?")]
	progress_slider.set_value_no_signal(i)

func _update_detail():
	if _messages.is_empty() or _current_index >= _messages.size(): return
	var m = _messages[_current_index]
	detail_title.text = "[%d/%d] %s" % [_current_index+1, _messages.size(), _scenario_data.get("scenario","?")]
	detail_json.text = JSON.stringify(m.get("data",{}), "\t")

func _log(t: String):
	console_log.text += t + "\n"
	var lc = console_log.get_line_count()
	if lc > 0: console_log.set_caret_line(lc-1)


# ============================================================================
# 按钮
# ============================================================================
func _on_play_pause():
	_is_playing = not _is_playing; play_pause_btn.text = "⏸" if _is_playing else "▶"

func _on_step():
	_is_playing = false
	play_pause_btn.text = "\u25b6"
	if _current_index < _messages.size():
		_advance()
	else:
		_current_index = 0
		_prev_state = {}
		_seek(0)
		_update_ui()
		_update_detail()

func _on_speed_toggle():
	var idx = SPEEDS.find(_speed_multiplier)
	_speed_multiplier = SPEEDS[(idx+1) % SPEEDS.size()]
	speed_btn.text = "%gx" % _speed_multiplier

func _on_slider_changed(_v: float):
	message_label.text = "[%d/%d] ..." % [int(progress_slider.value)+1, _messages.size()]

func _on_slider_drag_ended(c: bool):
	if not c:
		return
	_seek(int(progress_slider.value))
	_pending_delay = 0.0
	_update_detail()

func _on_jump():
	var t = jump_input.text.strip_edges()
	if t.is_valid_int():
		_seek(clampi(t.to_int() - 1, 0, _messages.size() - 1))
		_pending_delay = 0.0
		_update_detail()

func _on_file_selected(idx: int):
	var f = _replay_files[idx]
	if not f.get("disabled", false):
		_load_replay(f["path"])
