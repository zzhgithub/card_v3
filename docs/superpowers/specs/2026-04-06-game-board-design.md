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

**Constants change:** `LW` and `RW` change from 300px to 240px each.
Center width `CW = 1920 - 240 - 240 = 1440px`.

### Left Panel (x: 0–240)
- Top: Card image placeholder block (250×320, centered in panel)
- Bottom: Card basic info placeholder block (250×200, centered in panel)

### Center Panel (x: 240–1680)

From top to bottom:

| Region           | Height | Description                                    |
|------------------|--------|------------------------------------------------|
| Opponent hand    | 80px   | 手卡 zone bar, full center width               |
| Opponent area    | 440px  | Zone column (left) + field rows (right)        |
| Turn info bar    | 40px   | 回合数 / 当前玩家 / 当前回合阶段               |
| Player area      | 440px  | Zone column (left) + field rows (right)        |
| Player hand      | 80px   | 手卡 zone bar, full center width               |

Total: 80+440+40+440+80 = 1080px ✓

#### Coordinate System Note

`bw(pos)` converts **top-left-origin pixel coordinates** (x right, y down) to Bevy world
coordinates (origin at screen center). The formula is:

```rust
pub fn bw(pos: Vec2) -> Vec2 { Vec2::new(pos.x - 960.0, pos.y - 540.0) }
```

This does **not** flip the Y axis. In Bevy's 2D system Y increases upward, so a pixel Y
of 0 (screen top) maps to Bevy Y = -540 and a pixel Y of 1080 (screen bottom) maps to
Bevy Y = +540. All layout values in this spec use pixel space (Y increases downward); `bw`
is applied at spawn time. This matches the behavior of the existing implementation and the
existing unit tests.

#### Zone Column Layout (leftmost 160px of each player's center area)

```
┌────────┬────────┐
│ 费用区  │ 卡组   │
│ 6格    │ 65×95  │
│ 竖排   ├────────┤
│ 28×20  │ 墓地   │
│ 6px gap│ 65×95  │
└────────┴────────┘
```

- Zone column total width: 160px, starting at `CENTER_X` (= 240px from screen left)
- Left sub-column (cost zone, 70px wide): 6 cells of 28×20px, 6px vertical gap, stacked
  top-to-bottom. Total height: 6×20 + 5×6 = 150px. Vertically centered within the 440px
  player area: top offset = (440 - 150) / 2 = 145px below area top edge.
- Right sub-column (deck/grave, 90px wide):
  - Deck: 65×95px, vertically centered in the upper half of 440px → center at area_top + 110px
  - Grave: 65×95px, vertically centered in the lower half of 440px → center at area_top + 330px

`slot_index` for `ZoneSprite`: cost zone cells are spawned with `slot_index: Some(i)` where
`i` is 0–5 from top; deck and grave use `slot_index: None`.

#### Field Rows

Each player has two rows of 5 card slots (85×120px each, 10px horizontal gap between slots).
Field rows occupy the remaining center area after the 160px zone column:
  - Available x range: `CENTER_X + 160` to `CENTER_X + CW` = 400px to 1680px (= 1280px wide)
  - Five slots: 5×85 + 4×10 = 465px, left-aligned with 10px padding from zone column right edge.
  - Field slot x-start: `CENTER_X + 160 + 10 = 410px`

Vertical positions within the 440px player area (15px top/bottom padding, 10px row gap):
  - Row 1 center y (from area top): 15 + 60 = 75px (card half-height = 60px)
  - Row 2 center y (from area top): 15 + 120 + 10 + 60 = 205px
  - Remaining space at bottom: 440 - 15 - 120 - 10 - 120 - 15 = 160px (distributed as padding)

  To center more naturally use: area_top + (440 - 2×120 - 10) / 2 for row1 center offset.
  Simplified: row1_center = area_top + 145px, row2_center = area_top + 275px.

**Opponent area** (pixel y: 80–520, area_top = 80):
  - Row 1 (后场): y = 80 + 145 = 225px
  - Row 2 (前场): y = 80 + 275 = 355px (closer to center line at y=560)

**Player area** (pixel y: 560–1000, area_top = 560):
  - Row 1 (前场): y = 560 + 145 = 705px (closer to center line)
  - Row 2 (后场): y = 560 + 275 = 835px

### Right Panel (x: 1680–1920)

From top to bottom (all centered horizontally in panel):

| Element           | y center | Height | Notes                              |
|-------------------|----------|--------|------------------------------------|
| 对手基础信息 block | 135px    | 130px  | y: 70–200                         |
| 跳过 button        | 540px    | 45px   | Vertically centered on screen      |
| 自己的基础信息 block| 945px   | 130px  | y: 880–1010                        |
| 认输 button        | 1045px   | 45px   | Near bottom                        |

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

All fields listed below. Legacy fields (`back_field_start`, `front_field_start`,
`field_slot_spacing`) are retained so `local_game.rs` compiles unchanged. The new explicit
slot arrays are canonical; legacy fields are derived from them for compatibility.

```rust
pub struct PlayerAreaLayout {
    // Primary fields (new layout)
    pub deck_pos: Vec2,
    pub grave_pos: Vec2,
    pub cost_slots: [Vec2; 6],
    pub front_field_slots: [Vec2; 5],
    pub back_field_slots: [Vec2; 5],
    pub hand_y: f32,
    pub hp_pos: Vec2,
    // Legacy compat fields (derived, so local_game.rs compiles)
    pub back_field_start: Vec2,
    pub front_field_start: Vec2,
    pub field_slot_spacing: f32,
    pub cost_start_pos: Vec2,
    pub cost_spacing: f32,
}
```

`cost_start_pos` = `cost_slots[0]`, `cost_spacing` = slot height + gap = 26.0
`back_field_start` = `back_field_slots[0]`, `front_field_start` = `front_field_slots[0]`
`field_slot_spacing` = card width + gap = 95.0

### `ZoneType` (unchanged)
```rust
pub enum ZoneType { Deck, Hand, FrontField, BackField, CostZone, Grave }
```

## Colors

| Element            | sRGBA                          |
|--------------------|--------------------------------|
| Background         | (0.05, 0.07, 0.10, 1.00)      |
| Left panel         | (0.08, 0.08, 0.14, 0.95)      |
| Right panel        | (0.08, 0.08, 0.14, 0.95)      |
| Center bg          | (0.06, 0.10, 0.16, 0.90)      |
| Info bar           | (0.12, 0.18, 0.32, 0.90)      |
| Deck zone          | (0.22, 0.16, 0.08, 0.55)      |
| Grave zone         | (0.18, 0.08, 0.08, 0.55)      |
| CostZone           | (0.12, 0.22, 0.32, 0.55)      |
| BackField          | (0.08, 0.18, 0.16, 0.55)      |
| FrontField         | (0.16, 0.16, 0.26, 0.55)      |
| Hand bar           | (0.10, 0.10, 0.13, 0.35)      |
| HP display block   | (0.22, 0.22, 0.32, 1.00) sRGB |
| Skip button        | (0.18, 0.32, 0.55, 0.70)      |
| Surrender button   | (0.55, 0.18, 0.18, 0.70)      |

## Unit Tests

- `bw_origin`: `bw(Vec2::ZERO)` == `Vec2::new(-960.0, -540.0)`
- `bw_center`: `bw(Vec2::new(960.0, 540.0))` == `Vec2::ZERO`
- `mirror`: `p1.deck_pos.y + p2.deck_pos.y ≈ 2 × center_y`; same for `front_field_slots[0].y`
- `turn_centered`: `turn_info_pos.x ≈ 960.0`; `turn_info_pos.y ≈ 540.0`
- `hand_zones`: `p1.hand_y < center_y`, `p2.hand_y > center_y`
- `cost_slots_count`: both players have exactly 6 cost slots
- `field_slots_count`: both players have exactly 5 front and 5 back field slots

## Files Changed

- `crates/card-bevy/src/game_board.rs` — complete rewrite
- No other files modified
