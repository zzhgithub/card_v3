extends Control

@onready var version_label: Label = $VersionLabel
@onready var status_bar: Label = $StatusBar
@onready var button_container: VBoxContainer = $CenterContainer/ButtonContainer

func _ready():
	# 设置版本号
	version_label.text = "v0.1.0"

	# 设置状态栏
	status_bar.text = "主菜单"

	# 连接按钮信号
	for button in button_container.get_children():
		if button is Button:
			button.pressed.connect(_on_button_pressed.bind(button))
			button.mouse_entered.connect(_on_button_hovered.bind(button))
			button.mouse_exited.connect(_on_button_exited.bind(button))

			# 设置退出按钮的特殊颜色
			if button.name == "ExitButton":
				button.add_theme_color_override("font_color", Color(0.9, 0.3, 0.3))
				button.add_theme_color_override("font_hover_color", Color(1.0, 0.4, 0.4))

func _on_button_pressed(button: Button):
	match button.name:
		"LocalGameButton":
			print("本地游戏 - 未实现")
			# TODO: 切换到本地游戏场景
		"OnlineGameButton":
			print("联机游戏 - 未实现")
			# TODO: 切换到联机游戏场景
		"CardCatalogButton":
			print("卡片图鉴")
			get_tree().change_scene_to_file("res://scenes/card_catalog.tscn")
		"DeckEditorButton":
			print("卡组编辑")
			get_tree().change_scene_to_file("res://scenes/deck_editor.tscn")
		"SettingsButton":
			print("设置 - 未实现")
			# TODO: 切换到设置场景
		"ExitButton":
			get_tree().quit()

func _on_button_hovered(button: Button):
	# 悬停效果
	if button.name == "ExitButton":
		button.modulate = Color(1.2, 0.8, 0.8)
	else:
		button.modulate = Color(1.1, 1.1, 1.1)

func _on_button_exited(button: Button):
	# 恢复正常
	button.modulate = Color.WHITE

func _input(event):
	# F11 切换全屏
	if event is InputEventKey and event.pressed and event.keycode == KEY_F11:
		var current_mode = DisplayServer.window_get_mode()
		if current_mode == DisplayServer.WINDOW_MODE_FULLSCREEN:
			DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_WINDOWED)
		else:
			DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN)
	# ESC 退出全屏
	if event is InputEventKey and event.pressed and event.keycode == KEY_ESCAPE:
		var current_mode = DisplayServer.window_get_mode()
		if current_mode == DisplayServer.WINDOW_MODE_FULLSCREEN:
			DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_WINDOWED)
