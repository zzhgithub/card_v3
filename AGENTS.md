# PROJECT KNOWLEDGE BASE

**Generated:** 2026-03-12
**Branch:** master (no commits yet)

## OVERVIEW

Networked card battle game (2-player). Rust core engine + Lua scripting for card definitions. P2P TCP architecture, no central game server. Greenfield — zero implementation, design spec only.

## STRUCTURE

```
card_v3/
├── src/
│   └── main.rs          # Entry point (stub)
├── plan/
│   └── ReadMe.md        # Full game design spec (184 lines) — THE source of truth
├── logs/                 # Runtime logs (gitignored content)
├── Cargo.toml            # Edition 2024, no deps yet
└── .gitignore
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Game rules & mechanics | `plan/ReadMe.md` | Authoritative spec — phases, combat, costs, effects |
| Card type definitions | `plan/ReadMe.md` §卡片信息 | 4 types: Character/Strategy/Item/Legendary |
| Effect/chain system | `plan/ReadMe.md` §效果和连锁系统 | FILO chain resolution, Trigger/Condition/Action model |
| Architecture decisions | `plan/ReadMe.md` §部分设计和原则 | P2P, Lua scripting, client abstraction |
| Card ID format | `plan/ReadMe.md` §卡片ID定义原则 | `S[pack]-[type]-[num]` e.g. `S001-C-001` |

## ARCHITECTURE DECISIONS (from spec)

- **Lua scripting**: Cards defined in Lua. Scripts describe effects only — Rust executes logic.
- **P2P networking**: TCP-based, no central game server. Game host runs a server on startup.
- **Matchmaking server**: Separate standalone server only for matchmaking + version checks.
- **Client abstraction**: Well-defined client API supporting TUI, Bevy, Web, Unity frontends.
- **Replay system**: Video replay capability required.
- **Save/resume**: Local games support mid-game save and restore (残局系统).
- **Timeouts**: Per-operation timeout + per-game total timeout. Timeout = loss.
- **Per-game rules**: Each match can customize rules before start.

## DOMAIN MODEL (from spec, not yet implemented)

### Card Types
- **Character** (人物卡) — Has attack power. Fights in ForEnd Zone.
- **Strategy** (策略卡) — Subtypes: Normal, Trick (诡计), Instant (瞬时).
- **Item** (物品卡) — Subtypes: Normal, Persistent (存留).
- **Legendary** (传奇卡) — Max 5 per deck.

### Card Attributes
- Property: Rational (理性) / Divine (神性) / Spiritual (灵性) — exactly one
- Category: Math / Science / Literature / Philosophy / Mystery
- Effects: HashMap<String, Effect>

### Player Zones
Deck (40-60) | Hand (max 20) | ForEnd (5 slots) | BackEnd (5 slots) | Cost (max 6) | Grave | HP (init 5, max 6) | RealPoint (max 6)

### Game Phases
Turn Start → Draw → Recovery → Main1 → Battle → Main2 → Turn End

### Effect Structure (reference)
```
Effect { trigger, optional, activation_limit, conditions, choices, actions, costs }
```

## CONVENTIONS

- **Edition 2024** — use latest Rust features.
- **unsafe**: Allowed ONLY in Lua API loading layer. Nowhere else.
- **Language**: Game spec is in Chinese. Code and APIs should use English identifiers.
- **AI interaction language**: Always explain and answer in **Chinese (中文)**. All conversations, analysis, and explanations for this project must be in Chinese.
- **Logging**: Good logging is an explicit requirement.

## ANTI-PATTERNS

- DO NOT put game logic in Lua scripts — Lua describes effects, Rust executes.
- DO NOT build a central game server — P2P architecture is a hard requirement.
- DO NOT hardcode game rules — rules are per-match configurable.
- DO NOT use unsafe outside Lua API integration.

## COMMANDS

```bash
cargo build          # Build
cargo run            # Run (stub only currently)
cargo test           # No tests yet
cargo clippy         # No clippy config yet
```

## NOTES

- Project is pure greenfield. `src/main.rs` is Hello World.
- All game design lives in `plan/ReadMe.md` — read it before implementing anything.
- Cost system is unique: hand cards go to Cost Zone as payment, Cost Zone cards can still be played.
- RealPoint overflow (>6) emits an event that cards can react to.
- Chain system follows FILO (like Yu-Gi-Oh chain resolution).
- Recovery phase mechanic is opponent-dependent: recover X cards where X = highest cost card on opponent's field.
