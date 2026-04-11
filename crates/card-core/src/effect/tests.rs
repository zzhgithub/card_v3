use super::*;
use crate::types::*;

fn assert_serde_roundtrip(effect: &Effect) {
    let json = serde_json::to_string(effect).unwrap();
    let deserialized: Effect = serde_json::from_str(&json).unwrap();
    let json2 = serde_json::to_string(&deserialized).unwrap();
    assert_eq!(json, json2);
}

/// Spec example 1: On summon, summon S002-A-003 from deck/cost zone/grave without paying cost.
#[test]
fn spec_example_1_summon_specific_card_on_entry() {
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
    assert_serde_roundtrip(&effect);
    assert!(matches!(effect.trigger, Trigger::OnSummon));
    assert!(!effect.optional);
    assert!(matches!(
        effect.actions[0],
        Action::SummonFromZone { no_cost: true, .. }
    ));
}

/// Spec example 2: On attack, attack power +200.
#[test]
fn spec_example_2_attack_boost_on_attack() {
    let effect = Effect {
        trigger: Trigger::OnAttack,
        optional: false,
        activation_limit: None,
        conditions: None,
        choices: vec![],
        actions: vec![Action::ModifyAttack {
            target: CardRef::This,
            amount: 200,
        }],
        costs: None,
    };
    assert_serde_roundtrip(&effect);
    assert!(matches!(
        effect.actions[0],
        Action::ModifyAttack { amount: 200, .. }
    ));
}

/// Spec example 3: While on field, other friendly cards get +100 attack (continuous effect).
#[test]
fn spec_example_3_aura_attack_boost() {
    let effect = Effect {
        trigger: Trigger::OnSummon,
        optional: false,
        activation_limit: None,
        conditions: Some(Condition::CardIsOnField {
            card: CardRef::This,
        }),
        choices: vec![],
        actions: vec![Action::ApplyModifier {
            target: CardRef::This,
            modifier: Modifier::AttackBoost(100),
            duration: ModifierDuration::WhileSourceOnField,
        }],
        costs: None,
    };
    assert_serde_roundtrip(&effect);
    assert!(matches!(
        effect.actions[0],
        Action::ApplyModifier {
            duration: ModifierDuration::WhileSourceOnField,
            ..
        }
    ));
}

/// Spec example 4a: Immune to strategy card effects.
#[test]
fn spec_example_4a_immune_to_strategy() {
    let effect = Effect {
        trigger: Trigger::OnSummon,
        optional: false,
        activation_limit: None,
        conditions: None,
        choices: vec![],
        actions: vec![Action::ApplyModifier {
            target: CardRef::This,
            modifier: Modifier::ImmuneToCardType(CardType::Strategy),
            duration: ModifierDuration::WhileSourceOnField,
        }],
        costs: None,
    };
    assert_serde_roundtrip(&effect);
    assert!(matches!(
        effect.actions[0],
        Action::ApplyModifier {
            modifier: Modifier::ImmuneToCardType(CardType::Strategy),
            ..
        }
    ));
}

/// Spec example 4b: When opponent's RealPoint increases and this card is on field, may return to hand.
#[test]
fn spec_example_4b_return_to_hand_on_opponent_rp_change() {
    let effect = Effect {
        trigger: Trigger::OnEvent(EventTrigger::RealPointChanged {
            player: PlayerRef::Opponent,
        }),
        optional: true,
        activation_limit: None,
        conditions: Some(Condition::CardIsOnField {
            card: CardRef::This,
        }),
        choices: vec![],
        actions: vec![Action::ReturnToHand {
            target: CardRef::This,
        }],
        costs: None,
    };
    assert_serde_roundtrip(&effect);
    assert!(effect.optional);
    assert!(matches!(
        effect.trigger,
        Trigger::OnEvent(EventTrigger::RealPointChanged { .. })
    ));
    assert!(matches!(
        effect.conditions,
        Some(Condition::CardIsOnField { .. })
    ));
}

/// Spec example 5: On expose, draw 2 then discard 1.
#[test]
fn spec_example_5_draw_and_discard_on_expose() {
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
    assert_serde_roundtrip(&effect);
    assert_eq!(effect.actions.len(), 2);
    assert!(matches!(effect.actions[0], Action::Draw { count: 2, .. }));
    assert!(matches!(
        effect.actions[1],
        Action::Discard { count: 1, .. }
    ));
}

/// Spec example 6: Cost = send this card from field to grave; effect = 1 damage to opponent.
#[test]
fn spec_example_6_self_sacrifice_for_damage() {
    let effect = Effect {
        trigger: Trigger::OwnMainPhase,
        optional: true,
        activation_limit: None,
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
            filter: None,
        }),
    };
    assert_serde_roundtrip(&effect);
    assert!(effect.costs.is_some());
    assert!(matches!(
        effect.costs,
        Some(CostRequirement::SendFieldCardToGrave { count: 1, .. })
    ));
}

/// Spec example 7: On summon, heal 1 HP.
#[test]
fn spec_example_7_heal_on_summon() {
    let effect = Effect {
        trigger: Trigger::OnSummon,
        optional: false,
        activation_limit: None,
        conditions: None,
        choices: vec![],
        actions: vec![Action::HealHp {
            player: PlayerRef::Self_,
            amount: 1,
        }],
        costs: None,
    };
    assert_serde_roundtrip(&effect);
    assert!(matches!(
        effect.actions[0],
        Action::HealHp { amount: 1, .. }
    ));
}

#[test]
fn condition_ast_composition() {
    let condition = Condition::And(vec![
        Condition::Compare {
            left: ValueExpr::HandCount { player: PlayerRef::Opponent },
            op: CompareOp::Gt,
            right: ValueExpr::HandCount { player: PlayerRef::Self_ },
        },
        Condition::Compare {
            left: ValueExpr::CostZonePropertyCount {
                player: PlayerRef::Self_,
                property: Property::Rational,
            },
            op: CompareOp::Gt,
            right: ValueExpr::Literal { value: 3 },
        },
    ]);

    let json = serde_json::to_string(&condition).unwrap();
    let deserialized: Condition = serde_json::from_str(&json).unwrap();
    assert_eq!(condition, deserialized);
}

#[test]
fn applied_modifier_roundtrip() {
    let applied = AppliedModifier {
        modifier: Modifier::AttackBoost(100),
        source: InstanceId(42),
        duration: ModifierDuration::WhileSourceOnField,
    };

    let json = serde_json::to_string(&applied).unwrap();
    let deserialized: AppliedModifier = serde_json::from_str(&json).unwrap();
    assert_eq!(applied, deserialized);
}

#[test]
fn choice_with_card_filter() {
    let choice = Choice {
        choice_id: 0,
        target_type: TargetType::Card,
        filter: TargetFilter {
            player: Some(PlayerRef::Opponent),
            card_filter: Some(CardFilter::new().with_property(Property::Rational)),
            zone_filter: Some(vec![
                Zone::Front(0),
                Zone::Front(1),
                Zone::Front(2),
                Zone::Front(3),
                Zone::Front(4),
            ]),
        },
        count: ChoiceCount::Exactly(1),
    };

    let json = serde_json::to_string(&choice).unwrap();
    let deserialized: Choice = serde_json::from_str(&json).unwrap();
    let json2 = serde_json::to_string(&deserialized).unwrap();
    assert_eq!(json, json2);
}

#[test]
fn test_value_expr_literal_with_float() {
    // Test that ValueExpr::Literal can be deserialized from a float using internal tagging
    let json = r#"{"type":"Literal","value":3.0}"#;
    let result: Result<ValueExpr, _> = serde_json::from_str(json);
    println!("Deserializing {}: {:?}", json, result);
    assert!(result.is_ok(), "Failed to deserialize float literal: {:?}", result.err());
    assert_eq!(result.unwrap(), ValueExpr::Literal { value: 3 });
}

#[test]
fn test_value_expr_literal_with_int() {
    // Test that ValueExpr::Literal can be deserialized from an int using internal tagging
    let json = r#"{"type":"Literal","value":3}"#;
    let result: Result<ValueExpr, _> = serde_json::from_str(json);
    println!("Deserializing {}: {:?}", json, result);
    assert!(result.is_ok(), "Failed to deserialize int literal: {:?}", result.err());
    assert_eq!(result.unwrap(), ValueExpr::Literal { value: 3 });
}

#[test]
fn test_condition_with_literal() {
    // Test condition that has a Literal value - using internal tagging for ValueExpr and PlayerRef
    let json = r#"{"Compare":{"left":{"type":"RealPoint","player":{"type":"Self_"}},"op":"Ge","right":{"type":"Literal","value":3}}}"#;
    let result: Result<Condition, _> = serde_json::from_str(json);
    println!("Deserializing condition: {:?}", result);
    assert!(result.is_ok(), "Failed to deserialize condition: {:?}", result.err());
}

#[test]
fn test_godot_json_format_with_floats() {
    // This is the JSON format Godot produces with "type" tags on all enums
    // Both Action and ValueExpr now use internally tagged format
    let godot_json = r#"{"actions":[{"amount":1,"player":{"type":"Self_"},"type":"HealHp"}],"conditions":{"Compare":{"left":{"type":"RealPoint","player":{"type":"Self_"}},"op":"Ge","right":{"type":"Literal","value":3}}},"optional":false,"trigger":"OpponentMainPhase"}"#;

    let result: Result<Effect, _> = serde_json::from_str(godot_json);
    println!("Godot JSON result: {:?}", result);
    assert!(result.is_ok(), "Failed to deserialize Godot JSON: {:?}", result.err());
}

#[test]
fn test_godot_json_with_float_literal() {
    // Same test but with float value in Literal
    let godot_json = r#"{"actions":[{"amount":1,"player":{"type":"Self_"},"type":"HealHp"}],"conditions":{"Compare":{"left":{"type":"RealPoint","player":{"type":"Self_"}},"op":"Ge","right":{"type":"Literal","value":3.0}}},"optional":false,"trigger":"OpponentMainPhase"}"#;

    let result: Result<Effect, _> = serde_json::from_str(godot_json);
    println!("Godot JSON with float result: {:?}", result);
    assert!(result.is_ok(), "Failed to deserialize Godot JSON with float: {:?}", result.err());
}

#[test]
fn debug_serialize_condition() {
    // Let's see what JSON format Rust produces for conditions
    let condition = Condition::Compare {
        left: ValueExpr::RealPoint { player: PlayerRef::Self_ },
        op: CompareOp::Ge,
        right: ValueExpr::Literal { value: 3 },
    };

    let json = serde_json::to_string(&condition).unwrap();
    println!("Serialized condition: {}", json);

    // Verify roundtrip
    let deserialized: Condition = serde_json::from_str(&json).unwrap();
    assert_eq!(condition, deserialized);
}

#[test]
fn debug_serialize_value_expr() {
    // Let's see what JSON format Rust produces for ValueExpr
    let expr = ValueExpr::Literal { value: 3 };
    let json = serde_json::to_string(&expr).unwrap();
    println!("Serialized ValueExpr::Literal: {}", json);

    let expr2 = ValueExpr::RealPoint { player: PlayerRef::Self_ };
    let json2 = serde_json::to_string(&expr2).unwrap();
    println!("Serialized ValueExpr::RealPoint: {}", json2);
}
