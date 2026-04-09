use bevy::prelude::*;
use card_core::state::CardInstance;
use card_core::types::{CardId, InstanceId, PlayerId};

pub struct GameBoardPlugin;

const W: f32 = 1920.0;
const H: f32 = 1080.0;

// 三栏
const LW: f32 = 300.0; // 左面板宽
const RW: f32 = 300.0; // 右面板宽
const CX: f32 = LW; // 中央战场左边界
const CW: f32 = W - LW - RW; // 中央战场宽

impl Plugin for GameBoardPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BoardLayout::default());
    }
}

#[derive(Component)]
pub struct GameBoardCamera;

#[derive(Component)]
pub struct CardSprite {
    pub instance_id: InstanceId,
    pub definition_id: CardId,
    pub zone: ZoneType,
    pub owner: PlayerId,
}

#[derive(Component)]
pub struct ZoneSprite {
    pub zone_type: ZoneType,
    pub owner: PlayerId,
    pub slot_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneType {
    Deck,
    Hand,
    FrontField,
    BackField,
    CostZone,
    Grave,
}

#[derive(Component)]
pub struct GameBoardEntity;

#[derive(Component)]
pub struct HpDisplay {
    pub player_id: PlayerId,
}

#[derive(Resource)]
pub struct BoardLayout {
    pub player_areas: [PlayerAreaLayout; 2],
    pub center_y: f32,
    pub turn_info_pos: Vec2,
}

#[derive(Clone)]
pub struct PlayerAreaLayout {
    // 费用区 - 竖条排列6个槽位，位于左侧
    pub cost_zone_pos: Vec2,    // 费用区中心位置
    pub cost_slot_spacing: f32, // 费用区槽位间距（纵向）
    // 卡组和墓地 - 竖直排列，位于费用区右侧
    pub deck_pos: Vec2,  // 卡组位置（第1行）
    pub grave_pos: Vec2, // 墓地位置（第2行）
    // 前场和后场 - 5个槽位横向排列
    pub front_field_start: Vec2, // 前场起始位置
    pub back_field_start: Vec2,  // 后场起始位置
    pub field_slot_spacing: f32, // 场位间距（横向）
    // 手卡
    pub hand_y: f32,  // 手卡Y坐标
    pub hp_pos: Vec2, // HP显示位置
}

impl Default for BoardLayout {
    fn default() -> Self {
        let cy = H / 2.0;
        let center_left = CX + 40.0; // 中央战场左边界偏移
        let field_slot_width = 85.0; // 场位宽度
        let field_slot_gap = 8.0; // 场位间距
        let field_spacing = field_slot_width + field_slot_gap; // 场位总间距

        // 费用区参数 - 竖条排列
        let cost_zone_x = center_left + 30.0; // 费用区X位置
        let cost_slot_height = 28.0; // 费用槽高度
        let cost_slot_gap = 4.0; // 费用槽间距
        let _cost_zone_height = cost_slot_height * 6.0 + cost_slot_gap * 5.0; // 费用区总高度

        // 卡组和墓地位置 - 竖直排列在费用区右侧
        let deck_grave_x = cost_zone_x + 60.0; // 卡组和墓地X位置

        // 场地区域起始位置 - 在卡组/墓地右侧
        let field_start_x = deck_grave_x + 80.0;

        // 对手区域（上半部分）：后场在上，前场在下
        // 己方区域（下半部分）：前场在上，后场在下（镜像）

        // Player1 (己方，下半部分)
        let p1 = PlayerAreaLayout {
            // 费用区 - 竖条排列6个槽位
            cost_zone_pos: Vec2::new(cost_zone_x, cy - 135.0), // 费用区中心
            cost_slot_spacing: cost_slot_height + cost_slot_gap, // 纵向间距
            // 卡组和墓地 - 竖直排列
            deck_pos: Vec2::new(deck_grave_x, cy - 90.0), // 卡组在第1行
            grave_pos: Vec2::new(deck_grave_x, cy - 180.0), // 墓地在第2行
            // 前场和后场 - 己方前场在上(第1行)，后场在下(第2行)
            front_field_start: Vec2::new(field_start_x, cy - 90.0),
            back_field_start: Vec2::new(field_start_x, cy - 180.0),
            field_slot_spacing: field_spacing,
            // 手卡 - 底部
            hand_y: 65.0,
            hp_pos: Vec2::new(deck_grave_x, cy - 30.0),
        };

        // Player2 (对手，上半部分)
        let p2 = PlayerAreaLayout {
            // 费用区 - 竖条排列6个槽位
            cost_zone_pos: Vec2::new(cost_zone_x, H - (cy - 135.0)),
            cost_slot_spacing: -(cost_slot_height + cost_slot_gap), // 向上排列
            // 卡组和墓地 - 竖直排列
            deck_pos: Vec2::new(deck_grave_x, H - (cy - 90.0)), // 卡组在第1行（对手视角的上排）
            grave_pos: Vec2::new(deck_grave_x, H - (cy - 180.0)), // 墓地在第2行
            // 后场和前场 - 对手后场在上(第1行)，前场在下(第2行)
            back_field_start: Vec2::new(field_start_x, H - (cy - 90.0)),
            front_field_start: Vec2::new(field_start_x, H - (cy - 180.0)),
            field_slot_spacing: field_spacing,
            // 手卡 - 顶部
            hand_y: H - 65.0,
            hp_pos: Vec2::new(deck_grave_x, H - (cy - 30.0)),
        };

        Self {
            player_areas: [p1, p2],
            center_y: cy,
            turn_info_pos: Vec2::new(W / 2.0, cy),
        }
    }
}

pub fn setup_game_board(commands: &mut Commands, layout: &BoardLayout, font: &Handle<Font>) {
    info!("[GameBoard] Setting up game board");

    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        GameBoardCamera,
        GameBoardEntity,
    ));

    // 全屏背景
    bg(
        commands,
        0.0,
        0.0,
        W,
        H,
        Color::srgb(0.05, 0.07, 0.1),
        -30.0,
    );

    // 左面板
    bg(
        commands,
        LW / 2.0,
        H / 2.0,
        LW,
        H,
        Color::srgba(0.08, 0.08, 0.14, 0.95),
        -15.0,
    );
    lbl(commands, font, LW / 2.0, H - 45.0, "卡牌详情", 22.0);
    bg(
        commands,
        LW / 2.0,
        H / 2.0 + 90.0,
        250.0,
        320.0,
        Color::srgba(0.12, 0.12, 0.2, 0.7),
        -8.0,
    );
    lbl(commands, font, LW / 2.0, H / 2.0 + 265.0, "卡片图片", 16.0);
    bg(
        commands,
        LW / 2.0,
        H / 2.0 - 160.0,
        250.0,
        260.0,
        Color::srgba(0.12, 0.12, 0.2, 0.7),
        -8.0,
    );
    lbl(
        commands,
        font,
        LW / 2.0,
        H / 2.0 - 20.0,
        "卡片基本信息",
        16.0,
    );

    // 右面板
    bg(
        commands,
        W - RW / 2.0,
        H / 2.0,
        RW,
        H,
        Color::srgba(0.08, 0.08, 0.14, 0.95),
        -15.0,
    );
    lbl(commands, font, W - RW / 2.0, H - 45.0, "信息与操作", 22.0);
    bg(
        commands,
        W - RW / 2.0,
        H - 140.0,
        250.0,
        130.0,
        Color::srgba(0.12, 0.12, 0.2, 0.7),
        -8.0,
    );
    lbl(
        commands,
        font,
        W - RW / 2.0,
        H - 140.0,
        "对手基础信息",
        16.0,
    );
    bg(
        commands,
        W - RW / 2.0,
        H / 2.0 + 60.0,
        180.0,
        45.0,
        Color::srgba(0.18, 0.32, 0.55, 0.7),
        -5.0,
    );
    lbl(commands, font, W - RW / 2.0, H / 2.0 + 60.0, "跳过", 18.0);
    bg(
        commands,
        W - RW / 2.0,
        H / 2.0 - 100.0,
        250.0,
        130.0,
        Color::srgba(0.12, 0.12, 0.2, 0.7),
        -8.0,
    );
    lbl(
        commands,
        font,
        W - RW / 2.0,
        H / 2.0 - 100.0,
        "自己的基础信息",
        16.0,
    );
    bg(
        commands,
        W - RW / 2.0,
        75.0,
        180.0,
        45.0,
        Color::srgba(0.55, 0.18, 0.18, 0.7),
        -5.0,
    );
    lbl(commands, font, W - RW / 2.0, 75.0, "认输", 18.0);

    // 中央战场
    bg(
        commands,
        W / 2.0,
        H / 2.0,
        CW,
        H,
        Color::srgba(0.06, 0.1, 0.16, 0.9),
        -20.0,
    );

    for pid in [PlayerId::Player1, PlayerId::Player2] {
        let ai = if pid == PlayerId::Player1 { 0 } else { 1 };
        let a = &layout.player_areas[ai];
        info!("[GameBoard] Zones for {:?}", pid);

        // 手卡
        zb(
            commands,
            W / 2.0,
            a.hand_y,
            520.0,
            85.0,
            ZoneType::Hand,
            pid,
        );
        lbl(commands, font, W / 2.0, a.hand_y + 52.0, "手卡", 14.0);

        // 卡组
        zb(
            commands,
            a.deck_pos.x,
            a.deck_pos.y,
            70.0,
            100.0,
            ZoneType::Deck,
            pid,
        );
        lbl(
            commands,
            font,
            a.deck_pos.x,
            a.deck_pos.y - 68.0,
            "卡组",
            13.0,
        );

        // 墓地
        zb(
            commands,
            a.grave_pos.x,
            a.grave_pos.y,
            70.0,
            100.0,
            ZoneType::Grave,
            pid,
        );
        lbl(
            commands,
            font,
            a.grave_pos.x,
            a.grave_pos.y - 68.0,
            "墓地",
            13.0,
        );

        // 费用区 - 竖条排列6个槽位
        for i in 0..6 {
            let offset_y = a.cost_slot_spacing * (i as f32 - 2.5); // 中心对称排列
            let p = Vec2::new(a.cost_zone_pos.x, a.cost_zone_pos.y + offset_y);
            zb(commands, p.x, p.y, 42.0, 28.0, ZoneType::CostZone, pid);
        }
        lbl(
            commands,
            font,
            a.cost_zone_pos.x,
            a.cost_zone_pos.y,
            "费用",
            11.0,
        );

        // 后场
        for i in 0..5 {
            let p = a.back_field_start + Vec2::new(a.field_slot_spacing * i as f32, 0.0);
            zb(commands, p.x, p.y, 85.0, 120.0, ZoneType::BackField, pid);
        }
        lbl(
            commands,
            font,
            a.back_field_start.x + a.field_slot_spacing * 2.0,
            a.back_field_start.y + 78.0,
            "后场",
            13.0,
        );

        // 前场
        for i in 0..5 {
            let p = a.front_field_start + Vec2::new(a.field_slot_spacing * i as f32, 0.0);
            zb(commands, p.x, p.y, 85.0, 120.0, ZoneType::FrontField, pid);
        }
        lbl(
            commands,
            font,
            a.front_field_start.x + a.field_slot_spacing * 2.0,
            a.front_field_start.y + 78.0,
            "前场",
            13.0,
        );

        // HP
        commands.spawn((
            Sprite {
                color: Color::srgb(0.22, 0.22, 0.32),
                custom_size: Some(Vec2::new(90.0, 45.0)),
                ..default()
            },
            Transform::from_translation(bw(a.hp_pos).extend(0.0)),
            HpDisplay { player_id: pid },
            GameBoardEntity,
        ));
    }

    // 中线回合信息
    bg(
        commands,
        layout.turn_info_pos.x,
        layout.turn_info_pos.y,
        CW - 30.0,
        40.0,
        Color::srgba(0.12, 0.18, 0.32, 0.9),
        -5.0,
    );
    lbl(
        commands,
        font,
        layout.turn_info_pos.x,
        layout.turn_info_pos.y,
        "回合数 / 当前玩家 / 当前回合阶段",
        16.0,
    );

    info!("[GameBoard] Done");
}

fn bg(c: &mut Commands, x: f32, y: f32, w: f32, h: f32, col: Color, z: f32) {
    c.spawn((
        Sprite {
            color: col,
            custom_size: Some(Vec2::new(w, h)),
            ..default()
        },
        Transform::from_translation(bw(Vec2::new(x, y)).extend(z)),
        GameBoardEntity,
    ));
}

fn zb(c: &mut Commands, x: f32, y: f32, w: f32, h: f32, zt: ZoneType, owner: PlayerId) {
    let col = match zt {
        ZoneType::Deck => Color::srgba(0.22, 0.16, 0.08, 0.55),
        ZoneType::Grave => Color::srgba(0.18, 0.08, 0.08, 0.55),
        ZoneType::CostZone => Color::srgba(0.12, 0.22, 0.32, 0.55),
        ZoneType::BackField => Color::srgba(0.08, 0.18, 0.16, 0.55),
        ZoneType::FrontField => Color::srgba(0.16, 0.16, 0.26, 0.55),
        ZoneType::Hand => Color::srgba(0.1, 0.1, 0.13, 0.35),
    };
    c.spawn((
        Sprite {
            color: col,
            custom_size: Some(Vec2::new(w, h)),
            ..default()
        },
        Transform::from_translation(bw(Vec2::new(x, y)).extend(-8.0)),
        ZoneSprite {
            zone_type: zt,
            owner,
            slot_index: None,
        },
        GameBoardEntity,
    ));
}

fn lbl(c: &mut Commands, font: &Handle<Font>, x: f32, y: f32, text: &str, sz: f32) {
    let wp = bw(Vec2::new(x, y));
    c.spawn((
        Text2d::new(text.to_string()),
        TextColor(Color::srgba(0.82, 0.82, 0.88, 0.8)),
        TextFont {
            font: font.clone(),
            font_size: sz,
            ..default()
        },
        Transform::from_xyz(wp.x, wp.y, 10.0),
        GameBoardEntity,
    ));
}

pub fn spawn_card_sprite(
    c: &mut Commands,
    card: &CardInstance,
    pos: Vec2,
    zone: ZoneType,
    owner: PlayerId,
) -> Entity {
    let col = if card.current_attack.is_some() {
        Color::srgb(0.72, 0.52, 0.32)
    } else {
        Color::srgb(0.32, 0.42, 0.62)
    };
    c.spawn((
        Sprite {
            color: col,
            custom_size: Some(Vec2::new(75.0, 110.0)),
            ..default()
        },
        Transform::from_translation(bw(pos).extend(5.0)),
        CardSprite {
            instance_id: card.instance_id,
            definition_id: card.definition_id.clone(),
            zone,
            owner,
        },
        GameBoardEntity,
    ))
    .id()
}

pub fn bw(pos: Vec2) -> Vec2 {
    Vec2::new(pos.x - W / 2.0, pos.y - H / 2.0)
}

pub fn cleanup_game_board(mut commands: Commands, q: Query<Entity, With<GameBoardEntity>>) {
    info!("[GameBoard] Cleanup");
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bw_origin() {
        assert_eq!(bw(Vec2::ZERO), Vec2::new(-960.0, -540.0));
    }
    #[test]
    fn bw_center() {
        assert_eq!(bw(Vec2::new(960.0, 540.0)), Vec2::ZERO);
    }
    #[test]
    fn mirror() {
        let l = BoardLayout::default();
        let (p1, p2, cy) = (&l.player_areas[0], &l.player_areas[1], l.center_y);
        assert!((p1.deck_pos.y + p2.deck_pos.y - 2.0 * cy).abs() < 1.0);
        assert!((p1.front_field_start.y + p2.front_field_start.y - 2.0 * cy).abs() < 1.0);
    }
    #[test]
    fn turn_centered() {
        let l = BoardLayout::default();
        assert!((l.turn_info_pos.x - W / 2.0).abs() < 10.0);
        assert!((l.turn_info_pos.y - l.center_y).abs() < 10.0);
    }
    #[test]
    fn hand_zones() {
        let l = BoardLayout::default();
        let (p1, p2) = (&l.player_areas[0], &l.player_areas[1]);
        assert!(p1.hand_y > 0.0 && p1.hand_y < l.center_y);
        assert!(p2.hand_y > l.center_y && p2.hand_y < H);
    }
}
