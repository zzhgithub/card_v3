## 场景管理器
## 处理场景切换和过渡动画

extends Node

signal scene_changed(scene_name: String)

const SCENES = {
	"lobby": "res://scenes/game_boads/game_lobby.tscn",
	"connecting": "res://scenes/game_boads/connecting.tscn",
	"room_waiting": "res://scenes/game_boads/room_waiting.tscn",
	"game_board": "res://scenes/game_boads/game_board.tscn"
}

var current_scene: Node = null
var transition_overlay: ColorRect

func _ready() -> void:
	_create_transition_overlay()

func _create_transition_overlay() -> void:
	transition_overlay = ColorRect.new()
	transition_overlay.color = Color(0, 0, 0, 0)
	transition_overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
	transition_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	transition_overlay.z_index = 1000
	transition_overlay.hide()

## 切换场景
func change_scene(scene_name: String, params: Dictionary = {}) -> void:
	if not SCENES.has(scene_name):
		_print("Unknown scene: %s" % scene_name)
		return

	_print("Changing to scene: %s" % scene_name)

	# 淡出
	await _fade_out()

	# 加载新场景
	var scene_path = SCENES[scene_name]
	var packed_scene = load(scene_path)
	if not packed_scene:
		_print("Failed to load scene: %s" % scene_path)
		return

	var new_scene = packed_scene.instantiate()

	# 传递参数
	for key in params:
		if key in new_scene:
			new_scene.set(key, params[key])

	# 替换场景
	var root = get_tree().root
	if current_scene:
		root.remove_child(current_scene)
		current_scene.queue_free()

	current_scene = new_scene
	root.add_child(current_scene)

	# 淡入
	await _fade_in()

	scene_changed.emit(scene_name)
	_print("Scene changed to: %s" % scene_name)

## 淡出效果
func _fade_out() -> void:
	var tree = get_tree()
	var root = tree.root

	if transition_overlay.get_parent() == null:
		root.add_child(transition_overlay)

	transition_overlay.show()

	var tween = tree.create_tween()
	tween.tween_property(transition_overlay, "color:a", 1.0, 0.2)
	await tween.finished

## 淡入效果
func _fade_in() -> void:
	var tree = get_tree()

	var tween = tree.create_tween()
	tween.tween_property(transition_overlay, "color:a", 0.0, 0.2)
	await tween.finished

	transition_overlay.hide()

## 打印日志
func _print(msg: String) -> void:
	print("[SceneManager] %s" % msg)
