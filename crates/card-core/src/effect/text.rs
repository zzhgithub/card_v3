use std::collections::HashMap;

use serde_json::Value;

use crate::types::{CardFilter, CardRef, CardType, Category, EffectKey, PlayerRef, Property, Zone};

use super::{
    Action, ActivationLimit, Choice, ChoiceCount, CompareOp, Condition, CostRequirement, Effect,
    EventTrigger, Modifier, ModifierDuration, TargetType, Trigger, ValueExpr,
};

pub fn effect_to_chinese(effect: &Effect) -> String {
    let mut trigger = trigger_text(&effect.trigger);
    if let Some(limit) = &effect.activation_limit {
        let limit_text = match limit {
            ActivationLimit::OncePerTurn => "一回合一次",
            ActivationLimit::OncePerTurnSameName => "同名卡一回合一次",
        };
        trigger = format!("{}（{}）", trigger, limit_text);
    }

    let mut clauses = Vec::new();
    if let Some(condition) = &effect.conditions {
        clauses.push(format!("当{}时", condition_text(condition)));
    }

    let action_body = {
        let base_actions = actions_text(&effect.actions);
        if let Some(choice_text) = choices_text(&effect.choices) {
            format!("{}，{}", choice_text, base_actions)
        } else {
            base_actions
        }
    };

    let optional_prefix = if effect.optional { "可以" } else { "" };
    let content = if let Some(cost) = &effect.costs {
        format!(
            "{}将{}才能发动，{}",
            optional_prefix,
            cost_text(cost),
            action_body
        )
    } else {
        format!("{}{}", optional_prefix, action_body)
    };
    clauses.push(content);

    format!("{}，{}。", trigger, clauses.join("，"))
}

pub fn effects_from_json(
    effects: &HashMap<EffectKey, Value>,
) -> Vec<(EffectKey, Result<Effect, String>)> {
    let mut items: Vec<(EffectKey, Result<Effect, String>)> = effects
        .iter()
        .map(|(k, v)| {
            let parsed = serde_json::from_value::<Effect>(v.clone())
                .map_err(|err| format!("效果{}解析失败: {}", k.0, err));
            (k.clone(), parsed)
        })
        .collect();
    items.sort_by(|a, b| a.0 .0.cmp(&b.0 .0));
    items
}

pub fn card_effects_text(effects: &HashMap<EffectKey, Value>) -> Vec<String> {
    effects_from_json(effects)
        .into_iter()
        .map(|(_, item)| match item {
            Ok(effect) => effect_to_chinese(&effect),
            Err(_) => "（效果数据异常）".to_string(),
        })
        .collect()
}

pub fn trigger_text(trigger: &Trigger) -> String {
    match trigger {
        Trigger::TurnStart => "回合开始时".to_string(),
        Trigger::DrawPhase => "抽牌阶段".to_string(),
        Trigger::RecoveryPhase => "回收阶段".to_string(),
        Trigger::OwnMainPhase => "自己主要阶段".to_string(),
        Trigger::OpponentMainPhase => "对方主要阶段".to_string(),
        Trigger::BothMainPhase => "主要阶段".to_string(),
        Trigger::BattlePhase => "战斗阶段".to_string(),
        Trigger::TurnEnd => "回合结束时".to_string(),
        Trigger::OnSummon => "登场时".to_string(),
        Trigger::OnAttack => "攻击时".to_string(),
        Trigger::OnExpose => "暴露时".to_string(),
        Trigger::OnDestroy => "破坏时".to_string(),
        Trigger::OnEvent(event) => event_trigger_text(event),
    }
}

pub fn condition_text(condition: &Condition) -> String {
    match condition {
        Condition::And(items) => items
            .iter()
            .map(condition_text)
            .collect::<Vec<_>>()
            .join("并且"),
        Condition::Or(items) => items
            .iter()
            .map(condition_text)
            .collect::<Vec<_>>()
            .join("或者"),
        Condition::Not(inner) => format!("不是（{}）", condition_text(inner)),
        Condition::Compare { left, op, right } => {
            format!(
                "{}{}{}",
                value_expr_text(left),
                compare_op_text(*op),
                value_expr_text(right)
            )
        }
        Condition::CardIsOnField { card } => format!("{}在场上", card_ref_text(card)),
    }
}

pub fn action_text(action: &Action) -> String {
    match action {
        Action::Draw { player, count } => format!("{}抽{}张卡", player_ref_text(*player), count),
        Action::Damage { player, amount } => {
            format!("{}受到{}点伤害", player_ref_text(*player), amount)
        }
        Action::Destroy { target } => format!("将{}破坏", card_ref_text(target)),
        Action::SummonFromZone {
            card_id,
            from_zones,
            to_zone,
            no_cost,
        } => {
            let zones = from_zones
                .iter()
                .map(|z| zone_text(*z))
                .collect::<Vec<_>>()
                .join("、");
            let no_cost_text = if *no_cost {
                "不支付费用"
            } else {
                "支付费用"
            };
            format!(
                "将{}中的【{}】{}登场到{}",
                zones,
                card_id,
                no_cost_text,
                zone_text(*to_zone)
            )
        }
        Action::ReturnToHand { target } => format!("将{}返回手牌", card_ref_text(target)),
        Action::SendToGrave { target } => format!("将{}送入墓地", card_ref_text(target)),
        Action::ModifyAttack { target, amount } => {
            if *amount >= 0 {
                format!("{}的攻击力增加{}", card_ref_text(target), amount)
            } else {
                format!("{}的攻击力减少{}", card_ref_text(target), amount.abs())
            }
        }
        Action::GainRealPoint { player, amount } => {
            format!("{}获得{}点RealPoint", player_ref_text(*player), amount)
        }
        Action::HealHp { player, amount } => {
            format!("{}恢复{}点生命值", player_ref_text(*player), amount)
        }
        Action::Discard { player, count } => {
            format!("{}丢弃{}张手牌", player_ref_text(*player), count)
        }
        Action::ApplyModifier {
            target,
            modifier,
            duration,
        } => format!(
            "使{}获得“{}”（{}）",
            card_ref_text(target),
            modifier_text(modifier),
            modifier_duration_text(duration)
        ),
        Action::RemoveModifier {
            target,
            modifier_key,
        } => format!("移除{}上的修饰“{}”", card_ref_text(target), modifier_key),
        Action::Special { key } => format!("执行特殊效果“{}”", key.0),
    }
}

pub fn cost_text(cost: &CostRequirement) -> String {
    match cost {
        CostRequirement::SendFieldCardToGrave { count, filter } => {
            let card_text = if let Some(filter) = filter {
                card_filter_text(filter)
            } else {
                "卡".to_string()
            };
            format!("场上{}张{}送入墓地", count, card_text)
        }
        CostRequirement::DiscardHand { count } => format!("从手牌丢弃{}张卡", count),
        CostRequirement::SendCostZoneToGrave { count, filter } => {
            if let Some(filter) = filter {
                format!("费用区{}张{}送入墓地", count, card_filter_text(filter))
            } else {
                format!("费用区{}张卡送入墓地", count)
            }
        }
    }
}

pub fn player_ref_text(player: PlayerRef) -> &'static str {
    match player {
        PlayerRef::Self_ => "自己",
        PlayerRef::Opponent => "对手",
    }
}

pub fn modifier_text(modifier: &Modifier) -> String {
    match modifier {
        Modifier::AttackBoost(amount) => {
            if *amount >= 0 {
                format!("攻击力增加{}", amount)
            } else {
                format!("攻击力减少{}", amount.abs())
            }
        }
        Modifier::ImmuneToCardType(card_type) => {
            format!("不受{}效果影响", card_type_text(*card_type))
        }
        Modifier::ImmuneToProperty(property) => {
            format!("不受{}属性卡效果影响", property_text(*property))
        }
        Modifier::ImmuneToDestruction => "不会被效果破坏".to_string(),
        Modifier::ImmuneToTargeting => "不能成为效果对象".to_string(),
        Modifier::CanAttackDirectly => "可以直接攻击玩家".to_string(),
        Modifier::ExtraAttackPerTurn(count) => format!("每回合额外攻击{}次", count),
        Modifier::CannotAttack => "不能攻击".to_string(),
        Modifier::Special { key } => format!("特殊修饰“{}”", key),
    }
}

fn event_trigger_text(event: &EventTrigger) -> String {
    match event {
        EventTrigger::RealPointChanged { player } => {
            format!("{}的RealPoint发生变化时", player_ref_text(*player))
        }
        EventTrigger::RealPointOverflow { player } => {
            format!("{}的RealPoint溢出时", player_ref_text(*player))
        }
        EventTrigger::HpChanged { player } => {
            format!("{}的生命值发生变化时", player_ref_text(*player))
        }
        EventTrigger::CardDestroyed { filter } => format!("{}被破坏时", card_filter_text(filter)),
        EventTrigger::CardSummoned { filter } => format!("{}登场时", card_filter_text(filter)),
        EventTrigger::EffectActivated { filter } => {
            format!("{}的效果发动时", card_filter_text(filter))
        }
        EventTrigger::CardExposed { filter } => format!("{}暴露时", card_filter_text(filter)),
        EventTrigger::CardDrawn { player } => format!("{}抽卡时", player_ref_text(*player)),
    }
}

fn compare_op_text(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Gt => ">",
        CompareOp::Lt => "<",
        CompareOp::Ge => ">=",
        CompareOp::Le => "<=",
        CompareOp::Eq => "==",
        CompareOp::Ne => "!=",
    }
}

fn value_expr_text(expr: &ValueExpr) -> String {
    match expr {
        ValueExpr::Literal { value } => value.to_string(),
        ValueExpr::HandCount { player } => format!("{}手牌数量", player_ref_text(*player)),
        ValueExpr::CostZoneCount { player } => format!("{}费用区数量", player_ref_text(*player)),
        ValueExpr::CostZonePropertyCount { player, property } => format!(
            "{}费用区{}属性卡数量",
            player_ref_text(*player),
            property_text(*property)
        ),
        ValueExpr::FrontFieldCount { player } => format!("{}前场数量", player_ref_text(*player)),
        ValueExpr::BackFieldCount { player } => format!("{}后场数量", player_ref_text(*player)),
        ValueExpr::RealPoint { player } => format!("{}的RealPoint", player_possessive_text(*player)),
        ValueExpr::Hp { player } => format!("{}的生命值", player_possessive_text(*player)),
        ValueExpr::HighestCostOnField { player } => {
            format!("{}场上最高费用", player_ref_text(*player))
        }
        ValueExpr::AttackPower { card } => format!("{}的攻击力", card_ref_text(card)),
    }
}

fn zone_text(zone: Zone) -> String {
    match zone {
        Zone::Deck => "卡组".to_string(),
        Zone::Hand => "手牌".to_string(),
        Zone::Front(slot) => format!("前场第{}格", slot + 1),
        Zone::Back(slot) => format!("后场第{}格", slot + 1),
        Zone::CostZone => "费用区".to_string(),
        Zone::Grave => "墓地".to_string(),
    }
}

fn card_ref_text(card: &CardRef) -> String {
    match card {
        CardRef::This => "这张卡".to_string(),
        CardRef::ByInstanceId(id) => format!("实例ID={}的卡", id.0),
        CardRef::BySlot(zone, index) => format!("{}的第{}张卡", zone_text(*zone), index + 1),
    }
}

fn card_filter_text(filter: &CardFilter) -> String {
    let mut parts = Vec::new();
    if let Some(property) = filter.property {
        parts.push(format!("{}属性", property_text(property)));
    }
    if let Some(card_type) = filter.card_type {
        parts.push(card_type_text(card_type).to_string());
    }
    if let Some(category) = filter.category {
        parts.push(format!("{}范畴卡", category_text(category)));
    }
    if let Some(tags) = &filter.tags
        && !tags.is_empty()
    {
        parts.push(format!("带有标签{}的卡", tags.join("/")));
    }

    let cost_desc = match (filter.min_cost, filter.max_cost) {
        (Some(min), Some(max)) => Some(format!("费用{}-{}的卡", min, max)),
        (Some(min), None) => Some(format!("费用至少{}的卡", min)),
        (None, Some(max)) => Some(format!("费用至多{}的卡", max)),
        (None, None) => None,
    };

    if let Some(cost_desc) = cost_desc {
        if parts.is_empty() {
            return cost_desc;
        }
        return format!("{}且{}", parts.join(""), cost_desc);
    }

    if parts.is_empty() {
        "卡".to_string()
    } else if parts.len() == 1 {
        let only = &parts[0];
        if only.ends_with("卡") {
            only.clone()
        } else {
            format!("{}的卡", only)
        }
    } else {
        format!("{}的卡", parts.join("且"))
    }
}

fn card_type_text(card_type: CardType) -> &'static str {
    match card_type {
        CardType::Character => "人物卡",
        CardType::Strategy => "策略卡",
        CardType::Item => "物品卡",
        CardType::Legendary => "传奇卡",
    }
}

fn property_text(property: Property) -> &'static str {
    match property {
        Property::Rational => "理性",
        Property::Divine => "神性",
        Property::Spiritual => "灵性",
    }
}

fn modifier_duration_text(duration: &ModifierDuration) -> String {
    match duration {
        ModifierDuration::WhileSourceOnField => "持续至来源离场".to_string(),
        ModifierDuration::UntilEndOfTurn => "持续到回合结束".to_string(),
        ModifierDuration::Permanent => "永久持续".to_string(),
        ModifierDuration::TurnCount(turns) => format!("持续{}回合", turns),
    }
}

fn category_text(category: Category) -> &'static str {
    match category {
        Category::Math => "数学",
        Category::Science => "科学",
        Category::Literature => "文艺",
        Category::Philosophy => "哲学",
        Category::Mystery => "神秘",
    }
}

fn player_possessive_text(player: PlayerRef) -> &'static str {
    match player {
        PlayerRef::Self_ => "自己",
        PlayerRef::Opponent => "对手",
    }
}

fn actions_text(actions: &[Action]) -> String {
    let mut iter = actions.iter().map(action_text);
    if let Some(first) = iter.next() {
        let rest = iter.collect::<Vec<_>>();
        if rest.is_empty() {
            first
        } else {
            format!("{}，然后{}", first, rest.join("，然后"))
        }
    } else {
        "无效果".to_string()
    }
}

fn choices_text(choices: &[Choice]) -> Option<String> {
    if choices.is_empty() {
        return None;
    }
    Some(
        choices
            .iter()
            .map(choice_text)
            .collect::<Vec<_>>()
            .join("，"),
    )
}

fn choice_text(choice: &Choice) -> String {
    let scope = choice
        .filter
        .player
        .map(player_ref_text)
        .unwrap_or("任意玩家");

    let zone_desc = choice
        .filter
        .zone_filter
        .as_ref()
        .map(|zones| {
            if zones
                .iter()
                .all(|z| matches!(z, Zone::Front(_) | Zone::Back(_)))
            {
                "场上".to_string()
            } else {
                zones
                    .iter()
                    .map(|z| zone_text(*z))
                    .collect::<Vec<_>>()
                    .join("、")
            }
        })
        .unwrap_or_else(|| "区域".to_string());

    let count = match choice.count {
        ChoiceCount::Exactly(n) => n.to_string(),
        ChoiceCount::UpTo(n) => format!("至多{}", n),
        ChoiceCount::All => "全部".to_string(),
    };

    match choice.target_type {
        TargetType::Card => {
            let filter_desc = choice
                .filter
                .card_filter
                .as_ref()
                .map(card_filter_text)
                .unwrap_or_else(|| "卡".to_string());
            if matches!(choice.count, ChoiceCount::All) {
                format!("选择{}{}的全部{}", scope, zone_desc, filter_desc)
            } else {
                format!("选择{}{}的{}张{}", scope, zone_desc, count, filter_desc)
            }
        }
        TargetType::Player => {
            if matches!(choice.count, ChoiceCount::All) {
                "选择全部玩家".to_string()
            } else {
                format!("选择{}{}名玩家", scope, count)
            }
        }
        TargetType::Zone => {
            if matches!(choice.count, ChoiceCount::All) {
                format!("选择{}的全部区域", scope)
            } else {
                format!("选择{}{}个区域", scope, count)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use super::*;
    use crate::types::{CardId, Category};

    fn simple_effect(trigger: Trigger, action: Action) -> Effect {
        Effect {
            trigger,
            optional: false,
            activation_limit: None,
            conditions: None,
            choices: vec![],
            actions: vec![action],
            costs: None,
        }
    }

    #[test]
    fn trigger_text_covers_all_phase_and_self_action_triggers() {
        let cases = vec![
            (Trigger::TurnStart, "回合开始时"),
            (Trigger::DrawPhase, "抽牌阶段"),
            (Trigger::RecoveryPhase, "回收阶段"),
            (Trigger::OwnMainPhase, "自己主要阶段"),
            (Trigger::OpponentMainPhase, "对方主要阶段"),
            (Trigger::BothMainPhase, "主要阶段"),
            (Trigger::BattlePhase, "战斗阶段"),
            (Trigger::TurnEnd, "回合结束时"),
            (Trigger::OnSummon, "登场时"),
            (Trigger::OnAttack, "攻击时"),
            (Trigger::OnExpose, "暴露时"),
            (Trigger::OnDestroy, "破坏时"),
        ];

        for (trigger, expected) in cases {
            assert_eq!(trigger_text(&trigger), expected);
        }
    }

    #[test]
    fn trigger_text_covers_event_trigger_variants() {
        let event_cases = vec![
            (
                EventTrigger::RealPointChanged {
                    player: PlayerRef::Opponent,
                },
                "对手的RealPoint发生变化时",
            ),
            (
                EventTrigger::RealPointOverflow {
                    player: PlayerRef::Self_,
                },
                "自己的RealPoint溢出时",
            ),
            (
                EventTrigger::HpChanged {
                    player: PlayerRef::Self_,
                },
                "自己的生命值发生变化时",
            ),
            (
                EventTrigger::CardDestroyed {
                    filter: CardFilter::new().with_property(Property::Rational),
                },
                "理性属性的卡被破坏时",
            ),
            (
                EventTrigger::CardSummoned {
                    filter: CardFilter::new().with_card_type(CardType::Character),
                },
                "人物卡登场时",
            ),
            (
                EventTrigger::EffectActivated {
                    filter: CardFilter::new().with_category(Category::Mystery),
                },
                "神秘范畴卡的效果发动时",
            ),
            (
                EventTrigger::CardExposed {
                    filter: CardFilter::new().with_cost_range(2, 5),
                },
                "费用2-5的卡暴露时",
            ),
            (
                EventTrigger::CardDrawn {
                    player: PlayerRef::Opponent,
                },
                "对手抽卡时",
            ),
        ];

        for (event, expected) in event_cases {
            assert_eq!(trigger_text(&Trigger::OnEvent(event)), expected);
        }
    }

    #[test]
    fn condition_text_handles_and_or_not_compare_and_on_field() {
        let condition = Condition::And(vec![
            Condition::CardIsOnField {
                card: CardRef::This,
            },
            Condition::Or(vec![
                Condition::Compare {
                    left: ValueExpr::HandCount { player: PlayerRef::Opponent },
                    op: CompareOp::Gt,
                    right: ValueExpr::HandCount { player: PlayerRef::Self_ },
                },
                Condition::Not(Box::new(Condition::Compare {
                    left: ValueExpr::RealPoint { player: PlayerRef::Self_ },
                    op: CompareOp::Le,
                    right: ValueExpr::Literal { value: 1 },
                })),
            ]),
        ]);

        let text = condition_text(&condition);
        assert!(text.contains("这张卡在场上"));
        assert!(text.contains("并且"));
        assert!(text.contains("或者"));
        assert!(text.contains("不是"));
        assert!(text.contains("对手手牌数量"));
        assert!(text.contains("自己手牌数量"));
        assert!(text.contains("自己的RealPoint"));
    }

    #[test]
    fn action_text_covers_all_actions() {
        let actions = vec![
            (
                Action::Draw {
                    player: PlayerRef::Self_,
                    count: 2,
                },
                "自己抽2张卡",
            ),
            (
                Action::Damage {
                    player: PlayerRef::Opponent,
                    amount: 1,
                },
                "对手受到1点伤害",
            ),
            (
                Action::Destroy {
                    target: CardRef::This,
                },
                "将这张卡破坏",
            ),
            (
                Action::SummonFromZone {
                    card_id: CardId::new("S002-A-003"),
                    from_zones: vec![Zone::Deck, Zone::CostZone, Zone::Grave],
                    to_zone: Zone::Front(0),
                    no_cost: true,
                },
                "将卡组、费用区、墓地中的【S002-A-003】不支付费用登场到前场第1格",
            ),
            (
                Action::ReturnToHand {
                    target: CardRef::ByInstanceId(crate::types::InstanceId(3)),
                },
                "将实例ID=3的卡返回手牌",
            ),
            (
                Action::SendToGrave {
                    target: CardRef::BySlot(Zone::Back(1), 0),
                },
                "将后场第2格的第1张卡送入墓地",
            ),
            (
                Action::ModifyAttack {
                    target: CardRef::This,
                    amount: -150,
                },
                "这张卡的攻击力减少150",
            ),
            (
                Action::GainRealPoint {
                    player: PlayerRef::Self_,
                    amount: 2,
                },
                "自己获得2点RealPoint",
            ),
            (
                Action::HealHp {
                    player: PlayerRef::Self_,
                    amount: 1,
                },
                "自己恢复1点生命值",
            ),
            (
                Action::Discard {
                    player: PlayerRef::Opponent,
                    count: 1,
                },
                "对手丢弃1张手牌",
            ),
            (
                Action::ApplyModifier {
                    target: CardRef::This,
                    modifier: Modifier::ImmuneToCardType(CardType::Strategy),
                    duration: ModifierDuration::WhileSourceOnField,
                },
                "使这张卡获得“不受策略卡效果影响”（持续至来源离场）",
            ),
            (
                Action::RemoveModifier {
                    target: CardRef::This,
                    modifier_key: "anti_strategy".to_string(),
                },
                "移除这张卡上的修饰“anti_strategy”",
            ),
            (
                Action::Special {
                    key: EffectKey("custom_key".to_string()),
                },
                "执行特殊效果“custom_key”",
            ),
        ];

        for (action, expected) in actions {
            assert_eq!(action_text(&action), expected);
        }
    }

    #[test]
    fn cost_text_covers_all_cost_requirement_variants() {
        let c1 = CostRequirement::SendFieldCardToGrave {
            count: 1,
            filter: Some(CardFilter::new().with_property(Property::Rational)),
        };
        assert_eq!(cost_text(&c1), "场上1张理性属性的卡送入墓地");

        let c2 = CostRequirement::DiscardHand { count: 2 };
        assert_eq!(cost_text(&c2), "从手牌丢弃2张卡");

        let c3 = CostRequirement::SendCostZoneToGrave {
            count: 3,
            filter: None,
        };
        assert_eq!(cost_text(&c3), "费用区3张卡送入墓地");
    }

    #[test]
    fn modifier_text_covers_all_modifier_variants() {
        let cases = vec![
            (Modifier::AttackBoost(100), "攻击力增加100"),
            (
                Modifier::ImmuneToCardType(CardType::Strategy),
                "不受策略卡效果影响",
            ),
            (
                Modifier::ImmuneToProperty(Property::Divine),
                "不受神性属性卡效果影响",
            ),
            (Modifier::ImmuneToDestruction, "不会被效果破坏"),
            (Modifier::ImmuneToTargeting, "不能成为效果对象"),
            (Modifier::CanAttackDirectly, "可以直接攻击玩家"),
            (Modifier::ExtraAttackPerTurn(2), "每回合额外攻击2次"),
            (Modifier::CannotAttack, "不能攻击"),
            (
                Modifier::Special {
                    key: "mystic".to_string(),
                },
                "特殊修饰“mystic”",
            ),
        ];

        for (modifier, expected) in cases {
            assert_eq!(modifier_text(&modifier), expected);
        }
    }

    #[test]
    fn effect_to_chinese_end_to_end_without_condition_or_cost() {
        let effect = simple_effect(
            Trigger::OnAttack,
            Action::ModifyAttack {
                target: CardRef::This,
                amount: 200,
            },
        );

        assert_eq!(
            effect_to_chinese(&effect),
            "攻击时，这张卡的攻击力增加200。"
        );
    }

    #[test]
    fn effect_to_chinese_end_to_end_with_optional_limit_condition_and_cost() {
        let effect = Effect {
            trigger: Trigger::OwnMainPhase,
            optional: true,
            activation_limit: Some(ActivationLimit::OncePerTurn),
            conditions: Some(Condition::CardIsOnField {
                card: CardRef::This,
            }),
            choices: vec![],
            actions: vec![Action::Damage {
                player: PlayerRef::Opponent,
                amount: 1,
            }],
            costs: Some(CostRequirement::SendFieldCardToGrave {
                count: 1,
                filter: Some(CardFilter::new().with_card_type(CardType::Character)),
            }),
        };

        assert_eq!(
            effect_to_chinese(&effect),
            "自己主要阶段（一回合一次），当这张卡在场上时，可以将场上1张人物卡送入墓地才能发动，对手受到1点伤害。"
        );
    }

    #[test]
    fn effect_to_chinese_end_to_end_with_activation_limit_same_name() {
        let effect = Effect {
            trigger: Trigger::OnSummon,
            optional: false,
            activation_limit: Some(ActivationLimit::OncePerTurnSameName),
            conditions: None,
            choices: vec![],
            actions: vec![Action::HealHp {
                player: PlayerRef::Self_,
                amount: 1,
            }],
            costs: None,
        };

        assert_eq!(
            effect_to_chinese(&effect),
            "登场时（同名卡一回合一次），自己恢复1点生命值。"
        );
    }

    #[test]
    fn readme_example_1_summon_from_three_zones_text() {
        let effect = Effect {
            trigger: Trigger::OnSummon,
            optional: false,
            activation_limit: None,
            conditions: None,
            choices: vec![],
            actions: vec![Action::SummonFromZone {
                card_id: CardId::new("S002-A-003"),
                from_zones: vec![Zone::Deck, Zone::CostZone, Zone::Grave],
                to_zone: Zone::Front(0),
                no_cost: true,
            }],
            costs: None,
        };

        assert_eq!(
            effect_to_chinese(&effect),
            "登场时，将卡组、费用区、墓地中的【S002-A-003】不支付费用登场到前场第1格。"
        );
    }

    #[test]
    fn readme_example_5_draw_then_discard_text() {
        let effect = Effect {
            trigger: Trigger::OnExpose,
            optional: false,
            activation_limit: None,
            conditions: None,
            choices: vec![],
            actions: vec![
                Action::Draw {
                    player: PlayerRef::Self_,
                    count: 2,
                },
                Action::Discard {
                    player: PlayerRef::Self_,
                    count: 1,
                },
            ],
            costs: None,
        };

        assert_eq!(
            effect_to_chinese(&effect),
            "暴露时，自己抽2张卡，然后自己丢弃1张手牌。"
        );
    }

    #[test]
    fn readme_example_7_heal_text() {
        let effect = simple_effect(
            Trigger::OnSummon,
            Action::HealHp {
                player: PlayerRef::Self_,
                amount: 1,
            },
        );
        assert_eq!(effect_to_chinese(&effect), "登场时，自己恢复1点生命值。");
    }

    #[test]
    fn effects_from_json_deserializes_each_entry() {
        let valid_effect = serde_json::to_value(simple_effect(
            Trigger::OnAttack,
            Action::ModifyAttack {
                target: CardRef::This,
                amount: 200,
            },
        ))
        .expect("serialize effect to json");

        let mut effects = HashMap::new();
        effects.insert(EffectKey("e2".to_string()), json!({"bad": "shape"}));
        effects.insert(EffectKey("e1".to_string()), valid_effect);

        let list = effects_from_json(&effects);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].0 .0, "e1");
        assert!(list[0].1.is_ok());
        assert_eq!(list[1].0 .0, "e2");
        assert!(list[1].1.is_err());
    }

    #[test]
    fn card_effects_text_converts_success_and_masks_errors() {
        let mut effects = HashMap::new();
        effects.insert(
            EffectKey("e1".to_string()),
            serde_json::to_value(simple_effect(
                Trigger::OnSummon,
                Action::HealHp {
                    player: PlayerRef::Self_,
                    amount: 1,
                },
            ))
            .expect("serialize effect"),
        );
        effects.insert(EffectKey("e2".to_string()), json!({"broken": true}));

        let texts = card_effects_text(&effects);
        assert_eq!(texts.len(), 2);
        assert_eq!(texts[0], "登场时，自己恢复1点生命值。");
        assert_eq!(texts[1], "（效果数据异常）");
    }
}
