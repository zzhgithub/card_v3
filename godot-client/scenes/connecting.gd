## 连接中场景
## 显示连接状态，30秒超时处理

extends Control

signal connection_success
signal connection_failed(error_message: String)
signal cancelled

const TIMEOUT_SECONDS = 30.0

var _elapsed_time: float = 0.0
var _is_connecting: bool = false

@onready var spinner: TextureRect = $CenterContainer/VBoxContainer/Spinner
@onready var status_label: Label = $CenterContainer/VBoxContainer/StatusLabel
@onready var timer_label: Label = $CenterContainer/VBoxContainer/TimerLabel
@onready var cancel_button: Button = $CenterContainer/VBoxContainer/CancelButton

func _ready():
	cancel_button.pressed.connect(_on_cancel_pressed)
	_reset_state()


func _process(delta: float) -> void:
	if _is_connecting:
		_elapsed_time += delta
		_update_timer_display()

		# 旋转动画
		if spinner:
			spinner.rotation += delta * 5.0

		# 超时检查
		if _elapsed_time >= TIMEOUT_SECONDS:
			_on_connection_timeout()


## 开始连接
func start_connection() -> void:
	_is_connecting = true
	_elapsed_time = 0.0
	status_label.text = "正在连接服务器..."
	timer_label.visible = true


## 更新计时器显示
func _update_timer_display() -> void:
	var remaining = int(TIMEOUT_SECONDS - _elapsed_time)
	timer_label.text = "超时倒计时: %d秒" % remaining


## 连接超时
func _on_connection_timeout() -> void:
	_is_connecting = false
	status_label.text = "连接超时"
	_show_error_dialog("连接服务器超时，请检查网络或服务器地址后重试。")


## 显示错误弹窗
func _show_error_dialog(message: String) -> void:
	var dialog = AcceptDialog.new()
	dialog.title = "连接失败"
	dialog.dialog_text = message
	dialog.ok_button_text = "返回"
	dialog.confirmed.connect(func(): emit_signal("connection_failed", message))
	add_child(dialog)
	dialog.popup_centered()


## 连接成功（外部调用）
func on_connection_success() -> void:
	_is_connecting = false
	status_label.text = "连接成功！"
	emit_signal("connection_success")


## 连接失败（外部调用）
func on_connection_failed(error: String) -> void:
	_is_connecting = false
	status_label.text = "连接失败"
	_show_error_dialog(error)


## 取消按钮点击
func _on_cancel_pressed() -> void:
	_is_connecting = false
	emit_signal("cancelled")


## 重置状态
func _reset_state() -> void:
	_is_connecting = false
	_elapsed_time = 0.0
	status_label.text = "准备连接..."
	timer_label.visible = false
	if spinner:
		spinner.rotation = 0.0
