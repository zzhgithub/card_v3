## 卡片预留确认弹窗
## 显示预留确认界面

class_name CardReservePopup
extends Control

signal confirmed(card_data: Dictionary)
signal cancelled

var card_data: Dictionary = {}

@onready var overlay: ColorRect = $Overlay
@onready var content: Panel = $Content
@onready var card_name_label: Label = $Content/VBoxContainer/CardNameLabel
@onready var card_id_label: Label = $Content/VBoxContainer/CardIdLabel
@onready var confirm_button: Button = $Content/VBoxContainer/ButtonContainer/ConfirmButton
@onready var cancel_button: Button = $Content/VBoxContainer/ButtonContainer/CancelButton

func _ready():
	# 设置全屏
	set_anchors_preset(Control.PRESET_FULL_RECT)

	# 连接按钮
	confirm_button.pressed.connect(_on_confirm_pressed)
	cancel_button.pressed.connect(_on_cancel_pressed)

	# 点击背景取消
	overlay.gui_input.connect(_on_overlay_input)

	# 动画进入
	modulate = Color(1, 1, 1, 0)
	var tween = create_tween()
	tween.tween_property(self, "modulate", Color(1, 1, 1, 1), 0.2)


## 设置卡片数据
func setup(data: Dictionary) -> void:
	card_data = data
	card_name_label.text = "预留卡片：%s" % data.get("name", "Unknown")
	card_id_label.text = "编号：%s" % data.get("id", "Unknown")


## 点击背景
func _on_overlay_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed:
		_on_cancel_pressed()


## 确认预留
func _on_confirm_pressed() -> void:
	emit_signal("confirmed", card_data)
	_close_popup()


## 取消
func _on_cancel_pressed() -> void:
	emit_signal("cancelled")
	_close_popup()


## 关闭弹窗
func _close_popup() -> void:
	var tween = create_tween()
	tween.tween_property(self, "modulate", Color(1, 1, 1, 0), 0.15)
	tween.tween_callback(queue_free)


## 静态方法：便捷创建
static func show_reserve_dialog(parent: Node, data: Dictionary) -> CardReservePopup:
	var popup = preload("res://scenes/card_reserve_popup.tscn").instantiate()
	parent.add_child(popup)
	popup.setup(data)
	return popup
