# Decisions — card-game-engine

## [2026-03-13] Key architectural decisions
- 7 crates under crates/: card-core, card-script, card-protocol, card-server, card-client, card-tui, card-matchmaker
- Dependency graph: card-core ← card-script/card-protocol/card-server/card-client; card-protocol ← card-server/card-matchmaker; card-client ← card-tui/card-server
- mlua features: ["lua54", "serialize"] — NOT "luau"
- Sandbox: Lua::new_with(StdLib whitelist) — NOT lua.sandbox()
- Network: bincode for wire format, serde_json for save files
- Errors: thiserror in lib crates, anyhow in app crates (server/tui/matchmaker)
- State model: Command/Event pattern, GameState is pure data with PartialEq+Clone+Serialize

## [2026-03-22] card-tui 线程模型选择
- TUI 渲染/输入放主线程，游戏引擎放独立 `std::thread::spawn` 工作线程（因为 `PhaseClient` 同步阻塞）。
- `TuiClient` 只实现 `PhaseClient`（不接入 `ClientApi`），与 `GameEngine` 直接对接。
- 线程间协议分为三路：Action、Recovery、UI 事件日志，均采用 `std::sync::mpsc`。
