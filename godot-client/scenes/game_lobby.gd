## 联机游戏入口场景
## 输入房间ID、用户名、服务器地址，点击Join进入游戏

extends Control

signal back_pressed
signal join_requested(room_id: String, username: String, server_url: String)

const CONFIG_PATH = "user://game_config.cfg"
const DEFAULT_SERVER = "ws://localhost:8080"

@onready var room_input: LineEdit = $VBoxContainer/RoomInput
@onready var username_input: LineEdit = $VBoxContainer/UsernameInput
@onready var server_input: LineEdit = $VBoxContainer/ServerInput
@onready var join_button: Button = $VBoxContainer/ButtonContainer/JoinButton
@onready var back_button: Button = $VBoxContainer/ButtonContainer/BackButton

func _ready():
	# 连接按钮信号
	join_button.pressed.connect(_on_join_pressed)
	back_button.pressed.connect(_on_back_pressed)

	# 加载保存的用户名
	_load_username()

	# 设置默认值
	server_input.text = DEFAULT_SERVER

	# 验证输入
	_validate_inputs()
	room_input.text_changed.connect(_on_input_changed)
	username_input.text_changed.connect(_on_input_changed)


## 加载本地保存的用户名
func _load_username() -> void:
	var config = ConfigFile.new()
	var err = config.load(CONFIG_PATH)
	if err == OK:
		var saved_username = config.get_value("player", "username", "")
		if saved_username != "":
			username_input.text = saved_username


## 保存用户名到本地
func _save_username(username: String) -> void:
	var config = ConfigFile.new()
	config.load(CONFIG_PATH)
	config.set_value("player", "username", username)
	config.save(CONFIG_PATH)


## 输入变化时验证
func _on_input_changed(_text: String) -> void:
	_validate_inputs()


## 验证输入是否有效
func _validate_inputs() -> void:
	var has_room = room_input.text.strip_edges() != ""
	var has_username = username_input.text.strip_edges() != ""
	join_button.disabled = not (has_room and has_username)


## Join 按钮点击
func _on_join_pressed() -> void:
	var room_id = room_input.text.strip_edges()
	var username = username_input.text.strip_edges()
	var server_url = server_input.text.strip_edges()

	if server_url == "":
		server_url = DEFAULT_SERVER

	# 保存用户名
	_save_username(username)

	print("[GameLobby] Join requested - Room: %s, User: %s, Server: %s" % [room_id, username, server_url])
	emit_signal("join_requested", room_id, username, server_url)


## 返回按钮点击
func _on_back_pressed() -> void:
	emit_signal("back_pressed")
