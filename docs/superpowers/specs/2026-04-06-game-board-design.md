# Game Board Redesign Spec
Date: 2026-04-06

## Overview

Rewrite `crates/card-bevy/src/game_board.rs` to match the new layout design.
No other files are modified. Public interfaces (`ZoneType`, `ZoneSprite`, `CardSprite`,
`GameBoardEntity`, `HpDisplay`, `BoardLayout`, `bw`, `spawn_card_sprite`,
`setup_game_board`, `cleanup_game_board`) remain unchanged so `local_game.rs` requires
no edits.

## Screen Layout (1920×1080)

```
┌──────────┬────────────────────────────────────────────┬──────────┐
│ Left 240 │                Center 1440                 │ Right 240│
└──────────┴────────────────────────────────────────────┴──────────┘
```

### Left Panel (x: 0–240)
- Top: Card image placeholder (250×320 area, centered)
- Bottom: Card basic info placeholder (250×200 area, centered)

### Center Panel (x: 240–1680)

From top to bottom:

| Region           | Height | Description                                    |
|------------------|--------|------------------------------------------------|
| Opponent hand    | 80px   | 手卡 label, full center width                  |
| Opponent area    | 440px  | Zone column (left) + field rows (right)        |
| Turn info bar    | 40px   | 回合数 / 当前玩家 / 当前回合阶段               |
| Player area      | 440px  | Zone column (left) + field rows (right)        |
| Player hand      | 80px   | 手卡 label, full center width                  |

Total: 80+440+40+440+80 = 1080px ✓

#### Zone Column Layout (within each player area, leftmost 160px of center)

```
┌──────┬──────┐
│ 费用区│ 卡组 │
│ 6格   │ 65×95│
│ 竖排  ├──────┤
│      │ 墓地 │
│      │ 65×95│
└──────┴──────┘
```

- Cost zone slots: 6 cells, 28×20px each, 6px gap, stacked vertically; centered in left 70px of zone column
- Deck: 65×95px, right 90px of zone column, upper half
- Grave: 65×95px, right 90px of zone column, lower half

#### Field Rows

Each player has two rows of 5 card slots (85×120px, 10px gap between slots).

- **Opponent area** (top→bottom): 后场 row, then 前场 row (front field closer to center)
- **Player area** (top→bottom): 前场 row, then 后场 row (front field closer to center)

Row vertical positions within each area are centered, leaving ~15px top/bottom padding.

### Right Panel (x: 1680–1920)

From top to bottom:
- Opponent basic info block (~130px high)
- Skip button (跳过) — vertically centered
- Self basic info block (~130px high)
- Surrender button (认输) — near bottom

## Data Structures

### `BoardLayout` (Resource)
```rust
pub struct BoardLayout {
    pub player_areas: [PlayerAreaLayout; 2],  // [0]=Player1, [1]=Player2(opponent)
    pub center_y: f32,
    pub turn_info_pos: Vec2,
}
```

### `PlayerAreaLayout`
```rust
pub struct PlayerAreaLayout {
    pub deck_pos: Vec2,
    pub grave_pos: Vec2,
    pub cost_slots: [Vec2; 6],       // 6 cost zone slot centers
    pub front_field_slots: [Vec2; 5],
    pub back_field_slots: [Vec2; 5],
    pub hand_y: f32,
    pub hp_pos: Vec2,
}
```

- `cost_start_pos` + `cost_spacing` replaced by explicit `cost_slots` array for clarity
- `back_field_start` + `front_field_start` + `field_slot_spacing` replaced by explicit arrays

> **Note:** `local_game.rs` uses `layout.player_areas[i].deck_pos`, `grave_pos`, `hand_y`, `front_field_start`, `back_field_start`, and `field_slot_spacing` from the old struct via `spawn_initial_cards`. Since we're rewriting `game_board.rs` completely and not modifying `local_game.rs`, we must keep backward-compatible field names OR adjust both files.
>
> **Decision:** Keep `back_field_start`, `front_field_start`, `field_slot_spacing` in the struct alongside the new explicit arrays, so `local_game.rs` compiles unchanged.

### `ZoneType` (unchanged)
```rust
pub enum ZoneType { Deck, Hand, FrontField, BackField, CostZone, Grave }
```

## Coordinate System

`bw(pos: Vec2) -> Vec2` converts top-left–origin pixel coordinates to Bevy world
coordinates (origin at screen center):

```rust
fn bw(pos: Vec2) -> Vec2 { Vec2::new(pos.x - 960.0, pos.y - 540.0) }
```

All layout computations use top-left pixel space; `bw()` is applied at spawn time.

## Colors

| Zone       | Color (sRGBA)                |
|------------|------------------------------|
| Deck       | (0.22, 0.16, 0.08, 0.55)    |
| Grave      | (0.18, 0.08, 0.08, 0.55)    |
| CostZone   | (0.12, 0.22, 0.32, 0.55)    |
| BackField  | (0.08, 0.18, 0.16, 0.55)    |
| FrontField | (0.16, 0.16, 0.26, 0.55)    |
| Hand       | (0.10, 0.10, 0.13, 0.35)    |
| Background | (0.05, 0.07, 0.10, 1.0)     |
| Left panel | (0.08, 0.08, 0.14, 0.95)    |
| Right panel| (0.08, 0.08, 0.14, 0.95)    |
| Center bg  | (0.06, 0.10, 0.16, 0.90)    |
| Info bar   | (0.12, 0.18, 0.32, 0.90)    |
| Skip btn   | (0.18, 0.32, 0.55, 0.70)    |
| Surrender  | (0.55, 0.18, 0.18, 0.70)    |

## Unit Tests

- `bw_origin`: bw(0,0) == (-960, -540)
- `bw_center`: bw(960,540) == (0,0)
- `mirror`: player1 and player2 areas are symmetric about center_y
- `turn_centered`: turn_info_pos is horizontally centered and at center_y
- `hand_zones`: player1 hand_y < center_y, player2 hand_y > center_y
- `cost_slots_count`: each player has exactly 6 cost slots
- `field_slots_count`: each player has exactly 5 front and 5 back field slots

## Files Changed

- `crates/card-bevy/src/game_board.rs` — complete rewrite
- No other files modified
