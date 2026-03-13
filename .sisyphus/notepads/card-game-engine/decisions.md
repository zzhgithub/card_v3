# Decisions — card-game-engine

## [2026-03-13] Key architectural decisions
- 7 crates under crates/: card-core, card-script, card-protocol, card-server, card-client, card-tui, card-matchmaker
- Dependency graph: card-core ← card-script/card-protocol/card-server/card-client; card-protocol ← card-server/card-matchmaker; card-client ← card-tui/card-server
- mlua features: ["lua54", "serialize"] — NOT "luau"
- Sandbox: Lua::new_with(StdLib whitelist) — NOT lua.sandbox()
- Network: bincode for wire format, serde_json for save files
- Errors: thiserror in lib crates, anyhow in app crates (server/tui/matchmaker)
- State model: Command/Event pattern, GameState is pure data with PartialEq+Clone+Serialize
