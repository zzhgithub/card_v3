use crate::effect::{CardRef as EffectCardRef, CompareOp, Condition, ValueExpr};
use crate::state::{CardInstance, GameState};
use crate::types::{CardRef, PlayerId, PlayerRef, Zone};

pub fn evaluate_condition(
    cond: &Condition,
    state: &GameState,
    perspective: PlayerId,
    source: &CardInstance,
) -> bool {
    match cond {
        Condition::And(subs) => subs
            .iter()
            .all(|c| evaluate_condition(c, state, perspective, source)),
        Condition::Or(subs) => subs
            .iter()
            .any(|c| evaluate_condition(c, state, perspective, source)),
        Condition::Not(inner) => !evaluate_condition(inner, state, perspective, source),
        Condition::Compare { left, op, right } => {
            let l = resolve_value(left, state, perspective, source);
            let r = resolve_value(right, state, perspective, source);
            match op {
                CompareOp::Gt => l > r,
                CompareOp::Lt => l < r,
                CompareOp::Ge => l >= r,
                CompareOp::Le => l <= r,
                CompareOp::Eq => l == r,
                CompareOp::Ne => l != r,
            }
        }
        Condition::CardIsOnField { card } => is_card_on_field(card, state, perspective, source),
    }
}

pub fn resolve_value(
    expr: &ValueExpr,
    state: &GameState,
    perspective: PlayerId,
    source: &CardInstance,
) -> i32 {
    match expr {
        ValueExpr::Literal(n) => *n,
        ValueExpr::HandCount(player_ref) => {
            let player_id = resolve_player(player_ref, perspective);
            state.player(player_id).zones.hand.len() as i32
        }
        ValueExpr::CostZoneCount(player_ref) => {
            let player_id = resolve_player(player_ref, perspective);
            state.player(player_id).zones.cost_zone.len() as i32
        }
        ValueExpr::CostZonePropertyCount { .. } => 0,
        ValueExpr::FrontFieldCount(player_ref) => {
            let player_id = resolve_player(player_ref, perspective);
            state.player(player_id).zones.front.iter().flatten().count() as i32
        }
        ValueExpr::BackFieldCount(player_ref) => {
            let player_id = resolve_player(player_ref, perspective);
            state.player(player_id).zones.back.iter().flatten().count() as i32
        }
        ValueExpr::RealPoint(player_ref) => {
            let player_id = resolve_player(player_ref, perspective);
            state.player(player_id).real_point as i32
        }
        ValueExpr::Hp(player_ref) => {
            let player_id = resolve_player(player_ref, perspective);
            state.player(player_id).hp as i32
        }
        ValueExpr::HighestCostOnField(_) => 0,
        ValueExpr::AttackPower(card_ref) => {
            resolve_card_attack(card_ref, state, perspective, source)
        }
    }
}

fn resolve_player(player_ref: &PlayerRef, perspective: PlayerId) -> PlayerId {
    match player_ref {
        PlayerRef::Self_ => perspective,
        PlayerRef::Opponent => perspective.opponent(),
    }
}

fn resolve_card_attack(
    card_ref: &EffectCardRef,
    state: &GameState,
    perspective: PlayerId,
    source: &CardInstance,
) -> i32 {
    let card = match card_ref {
        CardRef::This => state.get_card(source.instance_id).map(|(card, _, _)| card),
        CardRef::ByInstanceId(id) => state.get_card(*id).map(|(card, _, _)| card),
        CardRef::BySlot(zone, slot) => card_from_slot(state, perspective, zone, *slot),
    };

    card.and_then(|c| c.current_attack).unwrap_or(0)
}

fn is_card_on_field(
    card_ref: &EffectCardRef,
    state: &GameState,
    perspective: PlayerId,
    source: &CardInstance,
) -> bool {
    match card_ref {
        CardRef::This => state
            .get_card(source.instance_id)
            .is_some_and(|(_, _, zone)| matches!(zone, Zone::Front(_) | Zone::Back(_))),
        CardRef::ByInstanceId(id) => state
            .get_card(*id)
            .is_some_and(|(_, _, zone)| matches!(zone, Zone::Front(_) | Zone::Back(_))),
        CardRef::BySlot(zone, slot) => {
            if card_from_slot(state, perspective, zone, *slot).is_none() {
                return false;
            }
            matches!(zone, Zone::Front(_) | Zone::Back(_))
        }
    }
}

fn card_from_slot<'a>(
    state: &'a GameState,
    player: PlayerId,
    zone: &Zone,
    slot: usize,
) -> Option<&'a CardInstance> {
    let zones = &state.player(player).zones;
    match zone {
        Zone::Deck => zones.deck.get(slot),
        Zone::Hand => zones.hand.get(slot),
        Zone::CostZone => zones.cost_zone.get(slot),
        Zone::Grave => zones.grave.get(slot),
        Zone::Front(_) => zones.front.get(slot).and_then(|card| card.as_ref()),
        Zone::Back(_) => zones.back.get(slot).and_then(|card| card.as_ref()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::GameRules;
    use crate::types::{CardId, InstanceId};

    fn make_state() -> GameState {
        GameState::new(GameRules::default(), 42)
    }

    fn source_card() -> CardInstance {
        CardInstance::new(InstanceId(0), CardId::new("S000-C-001"), Some(1000))
    }

    #[test]
    fn test_literal_compare_true() {
        let state = make_state();
        let cond = Condition::Compare {
            left: ValueExpr::Literal(5),
            op: CompareOp::Gt,
            right: ValueExpr::Literal(3),
        };
        assert!(evaluate_condition(
            &cond,
            &state,
            PlayerId::Player1,
            &source_card()
        ));
    }

    #[test]
    fn test_not_or_and_conditions() {
        let state = make_state();
        let cond = Condition::And(vec![
            Condition::Not(Box::new(Condition::Compare {
                left: ValueExpr::Literal(1),
                op: CompareOp::Gt,
                right: ValueExpr::Literal(5),
            })),
            Condition::Or(vec![
                Condition::Compare {
                    left: ValueExpr::Literal(3),
                    op: CompareOp::Eq,
                    right: ValueExpr::Literal(7),
                },
                Condition::Compare {
                    left: ValueExpr::Literal(9),
                    op: CompareOp::Eq,
                    right: ValueExpr::Literal(9),
                },
            ]),
        ]);

        assert!(evaluate_condition(
            &cond,
            &state,
            PlayerId::Player1,
            &source_card()
        ));
    }

    #[test]
    fn test_compare_operators() {
        let state = make_state();
        let source = source_card();

        let gt = Condition::Compare {
            left: ValueExpr::Literal(2),
            op: CompareOp::Gt,
            right: ValueExpr::Literal(1),
        };
        let lt = Condition::Compare {
            left: ValueExpr::Literal(1),
            op: CompareOp::Lt,
            right: ValueExpr::Literal(2),
        };
        let ge = Condition::Compare {
            left: ValueExpr::Literal(2),
            op: CompareOp::Ge,
            right: ValueExpr::Literal(2),
        };
        let le = Condition::Compare {
            left: ValueExpr::Literal(2),
            op: CompareOp::Le,
            right: ValueExpr::Literal(2),
        };
        let eq = Condition::Compare {
            left: ValueExpr::Literal(2),
            op: CompareOp::Eq,
            right: ValueExpr::Literal(2),
        };
        let ne = Condition::Compare {
            left: ValueExpr::Literal(2),
            op: CompareOp::Ne,
            right: ValueExpr::Literal(1),
        };

        assert!(evaluate_condition(&gt, &state, PlayerId::Player1, &source));
        assert!(evaluate_condition(&lt, &state, PlayerId::Player1, &source));
        assert!(evaluate_condition(&ge, &state, PlayerId::Player1, &source));
        assert!(evaluate_condition(&le, &state, PlayerId::Player1, &source));
        assert!(evaluate_condition(&eq, &state, PlayerId::Player1, &source));
        assert!(evaluate_condition(&ne, &state, PlayerId::Player1, &source));
    }

    #[test]
    fn test_hand_and_cost_zone_count() {
        let mut state = make_state();

        for i in 0..5 {
            let card = CardInstance::new(InstanceId(i + 100), CardId::new("S000-C-001"), None);
            state.players[0].zones.hand.push(card);
        }
        for i in 0..3 {
            let card = CardInstance::new(InstanceId(i + 200), CardId::new("S000-C-001"), None);
            state.players[1].zones.hand.push(card);
        }

        state.players[0].zones.cost_zone.push(CardInstance::new(
            InstanceId(300),
            CardId::new("S000-C-001"),
            None,
        ));
        state.players[0].zones.cost_zone.push(CardInstance::new(
            InstanceId(301),
            CardId::new("S000-C-001"),
            None,
        ));

        let cond = Condition::Compare {
            left: ValueExpr::HandCount(PlayerRef::Self_),
            op: CompareOp::Gt,
            right: ValueExpr::HandCount(PlayerRef::Opponent),
        };
        assert!(evaluate_condition(
            &cond,
            &state,
            PlayerId::Player1,
            &source_card()
        ));

        assert_eq!(
            resolve_value(
                &ValueExpr::CostZoneCount(PlayerRef::Self_),
                &state,
                PlayerId::Player1,
                &source_card()
            ),
            2
        );
    }

    #[test]
    fn test_field_counts_hp_and_real_point() {
        let mut state = make_state();
        state.players[0].zones.front[0] = Some(CardInstance::new(
            InstanceId(1),
            CardId::new("S000-C-001"),
            Some(500),
        ));
        state.players[0].zones.front[2] = Some(CardInstance::new(
            InstanceId(2),
            CardId::new("S000-C-001"),
            Some(500),
        ));
        state.players[0].zones.back[4] = Some(CardInstance::new(
            InstanceId(3),
            CardId::new("S000-C-001"),
            Some(500),
        ));
        state.players[0].hp = 4;
        state.players[0].real_point = 3;

        assert_eq!(
            resolve_value(
                &ValueExpr::FrontFieldCount(PlayerRef::Self_),
                &state,
                PlayerId::Player1,
                &source_card()
            ),
            2
        );
        assert_eq!(
            resolve_value(
                &ValueExpr::BackFieldCount(PlayerRef::Self_),
                &state,
                PlayerId::Player1,
                &source_card()
            ),
            1
        );
        assert_eq!(
            resolve_value(
                &ValueExpr::Hp(PlayerRef::Self_),
                &state,
                PlayerId::Player1,
                &source_card()
            ),
            4
        );
        assert_eq!(
            resolve_value(
                &ValueExpr::RealPoint(PlayerRef::Self_),
                &state,
                PlayerId::Player1,
                &source_card()
            ),
            3
        );
    }

    #[test]
    fn test_card_is_on_field_this_and_by_instance() {
        let mut state = make_state();
        let source = source_card();

        let cond_this = Condition::CardIsOnField {
            card: CardRef::This,
        };
        assert!(!evaluate_condition(
            &cond_this,
            &state,
            PlayerId::Player1,
            &source
        ));

        state.players[0].zones.front[0] = Some(source.clone());
        assert!(evaluate_condition(
            &cond_this,
            &state,
            PlayerId::Player1,
            &source
        ));

        let cond_by_id = Condition::CardIsOnField {
            card: CardRef::ByInstanceId(source.instance_id),
        };
        assert!(evaluate_condition(
            &cond_by_id,
            &state,
            PlayerId::Player1,
            &source
        ));
    }

    #[test]
    fn test_card_ref_by_slot_and_attack_power() {
        let mut state = make_state();
        state.players[0].zones.front[1] = Some(CardInstance::new(
            InstanceId(77),
            CardId::new("S000-C-001"),
            Some(777),
        ));

        let value = resolve_value(
            &ValueExpr::AttackPower(CardRef::BySlot(Zone::Front(0), 1)),
            &state,
            PlayerId::Player1,
            &source_card(),
        );
        assert_eq!(value, 777);
    }

    #[test]
    fn test_unavailable_registry_branches_fallback_to_zero() {
        let state = make_state();
        let source = source_card();

        assert_eq!(
            resolve_value(
                &ValueExpr::CostZonePropertyCount {
                    player: PlayerRef::Self_,
                    property: crate::types::Property::Rational,
                },
                &state,
                PlayerId::Player1,
                &source
            ),
            0
        );
        assert_eq!(
            resolve_value(
                &ValueExpr::HighestCostOnField(PlayerRef::Self_),
                &state,
                PlayerId::Player1,
                &source
            ),
            0
        );
    }
}
