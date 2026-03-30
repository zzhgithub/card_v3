use card_client::api::{CardPublicInfo, VisibleGameState};
use card_core::effect::text::card_effects_text;
use card_core::state::CardInstance;
use card_core::types::{CardDefinition, CardType, Category, Property};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::cursor::{FieldCursor, FieldZone};

pub fn render_info_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    cursor: &FieldCursor,
    state: &VisibleGameState,
    card_defs: &[CardDefinition],
) {
    let block = Block::default().borders(Borders::ALL).title("卡牌信息");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = if cursor.active {
        render_cursor_content(cursor, state, card_defs)
    } else {
        render_status_overview(state)
    };

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn render_status_overview(state: &VisibleGameState) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from("【当前状态】").style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
        Line::from(vec![
            Span::raw("回合: "),
            Span::styled(
                state.turn_number.to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("阶段: "),
            Span::styled(
                phase_display_name(state.phase),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("连锁: "),
            Span::styled(
                state.chain_size.to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("当前行动方: "),
            if state.is_my_turn {
                Span::styled("己方", Style::default().fg(Color::Green))
            } else {
                Span::styled("对手", Style::default().fg(Color::Red))
            },
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("己方卡组: "),
            Span::raw(format!("{}张", state.my_state.zones.deck.len())),
        ]),
        Line::from(vec![
            Span::raw("己方墓地: "),
            Span::raw(format!("{}张", state.my_state.zones.grave.len())),
        ]),
        Line::from(vec![
            Span::raw("己方费用区: "),
            Span::raw(format!("{}张", state.my_state.zones.cost_zone.len())),
        ]),
        Line::from(vec![
            Span::raw("对手费用区: "),
            Span::raw(format!("{}张", state.opponent_public.cost_zone.len())),
        ]),
    ];

    lines.push(Line::from(""));
    lines.push(Line::from("按 Tab 进入场地浏览模式").style(Style::default().fg(Color::DarkGray)));

    lines
}

fn render_cursor_content(
    cursor: &FieldCursor,
    state: &VisibleGameState,
    card_defs: &[CardDefinition],
) -> Vec<Line<'static>> {
    let zone_name = zone_display_name(&cursor.zone);

    match &cursor.zone {
        FieldZone::OpponentBack(idx) => state.opponent_public.back[*idx]
            .as_ref()
            .map(|public_info| render_opponent_card_info(zone_name.clone(), public_info, card_defs))
            .unwrap_or_else(|| empty_zone_lines(zone_name.clone(), cursor)),
        FieldZone::OpponentFront(idx) => state.opponent_public.front[*idx]
            .as_ref()
            .map(|public_info| render_opponent_card_info(zone_name.clone(), public_info, card_defs))
            .unwrap_or_else(|| empty_zone_lines(zone_name.clone(), cursor)),
        FieldZone::OpponentCostZone(idx) => state
            .opponent_public
            .cost_zone
            .get(*idx)
            .map(|public_info| render_opponent_card_info(zone_name.clone(), public_info, card_defs))
            .unwrap_or_else(|| empty_zone_lines(zone_name.clone(), cursor)),
        FieldZone::MyFront(idx) => state.my_state.zones.front[*idx]
            .as_ref()
            .map(|instance| render_my_card_info(zone_name.clone(), instance, card_defs))
            .unwrap_or_else(|| empty_zone_lines(zone_name.clone(), cursor)),
        FieldZone::MyBack(idx) => state.my_state.zones.back[*idx]
            .as_ref()
            .map(|instance| render_my_card_info(zone_name.clone(), instance, card_defs))
            .unwrap_or_else(|| empty_zone_lines(zone_name.clone(), cursor)),
        FieldZone::MyCostZone(idx) => state
            .my_state
            .zones
            .cost_zone
            .get(*idx)
            .map(|instance| render_my_card_info(zone_name.clone(), instance, card_defs))
            .unwrap_or_else(|| empty_zone_lines(zone_name.clone(), cursor)),
        FieldZone::MyHand(idx) => state
            .my_state
            .zones
            .hand
            .get(*idx)
            .map(|instance| render_my_card_info(zone_name.clone(), instance, card_defs))
            .unwrap_or_else(|| empty_zone_lines(zone_name.clone(), cursor)),
    }
}

fn empty_zone_lines(zone_name: String, cursor: &FieldCursor) -> Vec<Line<'static>> {
    vec![
        Line::from(format!("【{}】", zone_name)).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
        Line::from(format!("槽位 {} (空)", cursor.slot_index())),
        Line::from(""),
        Line::from("此位置无卡片").style(Style::default().fg(Color::DarkGray)),
    ]
}

fn render_my_card_info(
    zone_name: String,
    instance: &CardInstance,
    card_defs: &[CardDefinition],
) -> Vec<Line<'static>> {
    let def = card_defs
        .iter()
        .find(|d| d.id.0 == instance.definition_id.0);

    let mut lines = vec![
        Line::from(format!("【{}】", zone_name)).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
    ];

    if let Some(def) = def {
        lines.push(
            Line::from(def.name.clone()).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        );

        lines.push(Line::from(format!("ID: {}", def.id.0.clone())));

        lines.push(Line::from(""));

        let type_suffix = if let Some(kind) = def.strategy_kind {
            format!(" [{}]", strategy_kind_display_name(kind))
        } else if let Some(kind) = def.item_kind {
            format!(" [{}]", item_kind_display_name(kind))
        } else {
            String::new()
        };
        lines.push(Line::from(format!(
            "类型: {}{}",
            card_type_display_name(def.card_type),
            type_suffix
        )));

        lines.push(Line::from(format!(
            "属性: {}",
            property_display_name(def.property)
        )));

        lines.push(Line::from(format!(
            "范畴: {}",
            category_display_name(def.category)
        )));

        lines.push(
            Line::from(format!("费用: {}", def.cost)).style(Style::default().fg(Color::Cyan)),
        );

        if def.card_type == CardType::Character {
            let base_attack = def.attack.unwrap_or(0);
            let current_attack = instance.current_attack.unwrap_or(base_attack as i32);
            lines.push(
                Line::from(format!(
                    "攻击力: {} (基础: {})",
                    current_attack, base_attack
                ))
                .style(Style::default().fg(Color::Red)),
            );
        }

        lines.push(Line::from(""));
        lines.push(Line::from("效果:").style(Style::default().fg(Color::Cyan)));

        let effect_texts = card_effects_text(&def.effects);
        if effect_texts.is_empty() {
            lines.push(Line::from("  无效果").style(Style::default().fg(Color::DarkGray)));
        } else {
            for (idx, text) in effect_texts.into_iter().enumerate() {
                lines.push(Line::from(format!("  {}. {}", idx + 1, text)));
            }
        }
    } else {
        lines.push(Line::from(format!(
            "卡片ID: {}",
            instance.definition_id.0.clone()
        )));
        lines.push(Line::from(""));
        lines.push(Line::from("未找到卡片定义").style(Style::default().fg(Color::Red)));
    }

    lines
}

fn render_opponent_card_info(
    zone_name: String,
    public_info: &CardPublicInfo,
    card_defs: &[CardDefinition],
) -> Vec<Line<'static>> {
    let def = card_defs
        .iter()
        .find(|d| d.id.0 == public_info.definition_id.0);

    let mut lines = vec![
        Line::from(format!("【{}】", zone_name)).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
    ];

    lines.push(Line::from("【对手卡片】").style(Style::default().fg(Color::Magenta)));

    if let Some(def) = def {
        lines.push(Line::from(format!("ID: {}", def.id.0.clone())));

        lines.push(Line::from(""));

        lines.push(Line::from(format!(
            "类型: {}",
            card_type_display_name(def.card_type)
        )));

        lines.push(Line::from(vec![
            Span::raw("属性: "),
            Span::styled("???", Style::default().fg(Color::DarkGray)),
        ]));

        if def.card_type == CardType::Character {
            let current_attack = public_info.current_attack.unwrap_or(0);
            lines.push(Line::from(vec![
                Span::raw("攻击力: "),
                Span::styled(current_attack.to_string(), Style::default().fg(Color::Red)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from("效果: ").style(Style::default().fg(Color::DarkGray)));
        lines.push(Line::from("  [不可见]").style(Style::default().fg(Color::DarkGray)));
    } else {
        lines.push(Line::from(format!(
            "卡片ID: {}",
            public_info.definition_id.0.clone()
        )));
        lines.push(Line::from(""));
        lines.push(Line::from("未找到卡片定义").style(Style::default().fg(Color::Red)));
    }

    lines
}

fn zone_display_name(zone: &FieldZone) -> String {
    match zone {
        FieldZone::OpponentBack(_) => "对手后场",
        FieldZone::OpponentFront(_) => "对手前场",
        FieldZone::MyFront(_) => "己方前场",
        FieldZone::MyBack(_) => "己方后场",
        FieldZone::MyCostZone(_) => "己方费用区",
        FieldZone::MyHand(_) => "己方手卡",
        FieldZone::OpponentCostZone(_) => "对手费用区",
    }
    .to_string()
}

fn card_type_display_name(ct: CardType) -> &'static str {
    match ct {
        CardType::Character => "人物卡",
        CardType::Strategy => "策略卡",
        CardType::Item => "物品卡",
        CardType::Legendary => "传奇卡",
    }
}

fn property_display_name(p: Property) -> &'static str {
    match p {
        Property::Rational => "理性",
        Property::Divine => "神性",
        Property::Spiritual => "灵性",
    }
}

fn category_display_name(c: Category) -> &'static str {
    match c {
        Category::Math => "数学",
        Category::Science => "科学",
        Category::Literature => "文艺",
        Category::Philosophy => "哲学",
        Category::Mystery => "神秘",
    }
}

fn strategy_kind_display_name(kind: card_core::types::StrategyKind) -> &'static str {
    match kind {
        card_core::types::StrategyKind::Normal => "通常",
        card_core::types::StrategyKind::Trick => "诡计",
        card_core::types::StrategyKind::Instant => "瞬时",
    }
}

fn item_kind_display_name(kind: card_core::types::ItemKind) -> &'static str {
    match kind {
        card_core::types::ItemKind::Normal => "通常",
        card_core::types::ItemKind::Persistent => "存留",
    }
}

fn phase_display_name(phase: card_core::state::Phase) -> &'static str {
    match phase {
        card_core::state::Phase::TurnStart => "回合开始",
        card_core::state::Phase::Draw => "抽牌",
        card_core::state::Phase::Recovery => "回收",
        card_core::state::Phase::Main1 => "主阶段1",
        card_core::state::Phase::Battle => "战斗",
        card_core::state::Phase::Main2 => "主阶段2",
        card_core::state::Phase::TurnEnd => "回合结束",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use card_core::types::{CardType, Category, Property, StrategyKind};

    #[test]
    fn test_card_type_display_name() {
        assert_eq!(card_type_display_name(CardType::Character), "人物卡");
        assert_eq!(card_type_display_name(CardType::Strategy), "策略卡");
        assert_eq!(card_type_display_name(CardType::Item), "物品卡");
        assert_eq!(card_type_display_name(CardType::Legendary), "传奇卡");
    }

    #[test]
    fn test_property_display_name() {
        assert_eq!(property_display_name(Property::Rational), "理性");
        assert_eq!(property_display_name(Property::Divine), "神性");
        assert_eq!(property_display_name(Property::Spiritual), "灵性");
    }

    #[test]
    fn test_category_display_name() {
        assert_eq!(category_display_name(Category::Math), "数学");
        assert_eq!(category_display_name(Category::Science), "科学");
        assert_eq!(category_display_name(Category::Literature), "文艺");
        assert_eq!(category_display_name(Category::Philosophy), "哲学");
        assert_eq!(category_display_name(Category::Mystery), "神秘");
    }

    #[test]
    fn test_zone_display_name() {
        let zones = vec![
            (FieldZone::MyFront(0), "己方前场"),
            (FieldZone::MyBack(0), "己方后场"),
            (FieldZone::MyCostZone(0), "己方费用区"),
            (FieldZone::MyHand(0), "己方手卡"),
            (FieldZone::OpponentFront(0), "对手前场"),
            (FieldZone::OpponentBack(0), "对手后场"),
            (FieldZone::OpponentCostZone(0), "对手费用区"),
        ];

        for (zone, expected) in zones {
            assert_eq!(zone_display_name(&zone), expected);
        }
    }

    #[test]
    fn test_strategy_kind_display_name() {
        assert_eq!(strategy_kind_display_name(StrategyKind::Normal), "通常");
        assert_eq!(strategy_kind_display_name(StrategyKind::Trick), "诡计");
        assert_eq!(strategy_kind_display_name(StrategyKind::Instant), "瞬时");
    }
}
