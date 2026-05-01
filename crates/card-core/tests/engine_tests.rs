use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use card_core::effect::{
    Action, ActivationLimit, EventTrigger, Modifier, ModifierDuration, Trigger,
};
use card_core::engine::chain::{ChainEntry, ChainManager};
use card_core::engine::phase::{PhaseAction, PhaseClient, PhaseRunner, RecoveryCardOption};
use card_core::engine::{ActionExecutor, ModifierManager, TriggerChecker};
use card_core::rules::GameRules;
use card_core::state::events::GameOverReason;
use card_core::state::{CardInstance, CardRegistryImpl, CoreGameEvent, GameState, Phase};
use card_core::types::{
    CardDefinition, CardId, CardRef, CardType, Category, EffectKey, InstanceId, PlayerId,
    PlayerRef, Property, Zone,
};
use card_core::GameEngine;

struct PassAllClient;

impl PhaseClient for PassAllClient {
    fn choose_action(&self, available: &[PhaseAction], _timeout: Duration) -> Option<PhaseAction> {
        available
            .iter()
            .find(|a| matches!(a, PhaseAction::Pass))
            .cloned()
            .or_else(|| available.first().cloned())
    }

    fn choose_recovery_cards(
        &self,
        options: &[RecoveryCardOption],
        count: usize,
        _timeout: Duration,
    ) -> Option<Vec<InstanceId>> {
        Some(options.iter().take(count).map(|o| o.instance_id).collect())
    }
}

struct ScriptedClient {
    actions: Mutex<VecDeque<PhaseAction>>,
    recovery: Mutex<VecDeque<Vec<InstanceId>>>,
}

impl ScriptedClient {
    fn new(actions: Vec<PhaseAction>, recovery: Vec<Vec<InstanceId>>) -> Self {
        Self {
            actions: Mutex::new(actions.into()),
            recovery: Mutex::new(recovery.into()),
        }
    }
}

impl PhaseClient for ScriptedClient {
    fn choose_action(&self, available: &[PhaseAction], _timeout: Duration) -> Option<PhaseAction> {
        self.actions
            .lock()
            .expect("actions lock")
            .pop_front()
            .or_else(|| available.first().cloned())
    }

    fn choose_recovery_cards(
        &self,
        options: &[RecoveryCardOption],
        count: usize,
        _timeout: Duration,
    ) -> Option<Vec<InstanceId>> {
        self.recovery
            .lock()
            .expect("recovery lock")
            .pop_front()
            .or_else(|| Some(options.iter().take(count).map(|o| o.instance_id).collect()))
    }
}

fn make_test_registry() -> CardRegistryImpl {
    let mut registry = CardRegistryImpl::new();
    registry.insert(CardDefinition::new(
        CardId::new("S999-C-001"),
        "Scout".to_string(),
        CardType::Character,
        Property::Rational,
        Category::Math,
        1,
    ));
    registry.insert(CardDefinition::new(
        CardId::new("S999-C-002"),
        "Guard".to_string(),
        CardType::Character,
        Property::Divine,
        Category::Science,
        2,
    ));
    registry.insert(CardDefinition::new(
        CardId::new("S999-C-003"),
        "Giant".to_string(),
        CardType::Character,
        Property::Spiritual,
        Category::Philosophy,
        4,
    ));
    registry
}

fn make_test_state(rules: GameRules, deck_size: usize) -> GameState {
    let mut state = GameState::new(rules, 42);
    for i in 0..deck_size {
        state.players[0].zones.deck.push(CardInstance::new(
            InstanceId(1000 + i as u32),
            CardId::new("S999-C-001"),
            Some(300 + i as i32),
        ));
        state.players[1].zones.deck.push(CardInstance::new(
            InstanceId(2000 + i as u32),
            CardId::new("S999-C-002"),
            Some(280 + i as i32),
        ));
    }
    state
}

fn test_card(instance: u32, definition: &str, attack: i32) -> CardInstance {
    CardInstance::new(InstanceId(instance), CardId::new(definition), Some(attack))
}

fn has_game_over(events: &[CoreGameEvent], reason: GameOverReason) -> bool {
    events.iter().any(|e| {
        matches!(
            e,
            CoreGameEvent::GameOver {
                reason: r,
                ..
            } if *r == reason
        )
    })
}

#[test]
fn phase_flow_complete_turn_rotation_emits_7_phases_in_order() {
    let mut rules = GameRules::default();
    rules.first_player_draws = true;
    let mut state = make_test_state(rules, 3);
    let registry = make_test_registry();

    let p1 = ScriptedClient::new(
        vec![PhaseAction::Pass, PhaseAction::Pass, PhaseAction::Pass],
        vec![],
    );
    let p2 = ScriptedClient::new(vec![], vec![]);

    let events = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);
    let phases: Vec<Phase> = events
        .iter()
        .filter_map(|event| match event {
            CoreGameEvent::PhaseChanged { new_phase } => Some(*new_phase),
            _ => None,
        })
        .collect();

    assert_eq!(
        phases,
        vec![
            Phase::TurnStart,
            Phase::Draw,
            Phase::Recovery,
            Phase::Main1,
            Phase::Battle,
            Phase::Main2,
            Phase::TurnEnd,
        ]
    );
    assert_eq!(state.turn_player, PlayerId::Player2);
    assert_eq!(state.turn_number, 2);
}

#[test]
fn phase_flow_first_turn_no_draw_when_rule_disabled() {
    let rules = GameRules::default();
    let mut state = make_test_state(rules, 2);
    let registry = make_test_registry();
    let hand_before = state.players[0].zones.hand.len();

    let p1 = ScriptedClient::new(
        vec![PhaseAction::Pass, PhaseAction::Pass, PhaseAction::Pass],
        vec![],
    );
    let p2 = ScriptedClient::new(vec![], vec![]);

    let events = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

    assert_eq!(state.players[0].zones.hand.len(), hand_before);
    assert!(!events.iter().any(|e| {
        matches!(
            e,
            CoreGameEvent::DrawCard {
                player: PlayerId::Player1,
                ..
            }
        )
    }));
}

#[test]
fn phase_flow_first_turn_draw_when_rule_enabled() {
    let mut rules = GameRules::default();
    rules.first_player_draws = true;
    let mut state = make_test_state(rules, 2);
    let registry = make_test_registry();

    let p1 = ScriptedClient::new(
        vec![PhaseAction::Pass, PhaseAction::Pass, PhaseAction::Pass],
        vec![],
    );
    let p2 = ScriptedClient::new(vec![], vec![]);

    let events = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

    assert_eq!(state.players[0].zones.hand.len(), 1);
    assert!(events.iter().any(|e| {
        matches!(
            e,
            CoreGameEvent::DrawCard {
                player: PlayerId::Player1,
                ..
            }
        )
    }));
}

#[test]
fn phase_flow_deck_out_triggers_game_over() {
    let mut rules = GameRules::default();
    rules.first_player_draws = true;
    let mut state = make_test_state(rules, 0);
    let registry = make_test_registry();

    let p1 = ScriptedClient::new(vec![PhaseAction::Pass], vec![]);
    let p2 = ScriptedClient::new(vec![], vec![]);

    let events = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

    assert!(has_game_over(&events, GameOverReason::DeckOut));
}

#[test]
fn phase_flow_hp_zero_triggers_game_over() {
    let mut rules = GameRules::default();
    rules.first_player_draws = true;
    let mut state = make_test_state(rules, 2);
    let registry = make_test_registry();

    state.turn_number = 2;
    state.players[0].zones.front[0] = Some(test_card(10, "S999-C-001", 5));
    state.players[1].hp = 1;

    let p1 = ScriptedClient::new(
        vec![
            PhaseAction::Pass,
            PhaseAction::DeclareAttack {
                attacker: InstanceId(10),
                target_slot: None,
            },
        ],
        vec![],
    );
    let p2 = ScriptedClient::new(vec![], vec![]);

    let events = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

    assert!(has_game_over(&events, GameOverReason::HpZero));
    assert_eq!(state.players[1].hp, 0);
}

#[test]
fn phase_flow_recovery_phase_count_uses_opponent_highest_cost() {
    let rules = GameRules::default();
    let mut state = make_test_state(rules, 0);
    let registry = make_test_registry();

    state.players[0]
        .zones
        .cost_zone
        .push(test_card(31, "S999-C-001", 1));
    state.players[0]
        .zones
        .cost_zone
        .push(test_card(32, "S999-C-001", 1));
    state.players[0]
        .zones
        .cost_zone
        .push(test_card(33, "S999-C-001", 1));

    state.players[1].zones.front[0] = Some(test_card(41, "S999-C-002", 2));
    state.players[1].zones.front[1] = Some(test_card(42, "S999-C-003", 4));

    let p1 = PassAllClient;
    let p2 = PassAllClient;

    let events = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

    let moved_from_cost = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                CoreGameEvent::CardMoved {
                    from,
                    to,
                    ..
                } if from.zone == Zone::CostZone && to.zone == Zone::Hand
            )
        })
        .count();

    assert_eq!(moved_from_cost, 3);
    assert_eq!(state.players[0].zones.cost_zone.len(), 0);
    assert_eq!(state.players[0].zones.hand.len(), 3);
}

#[test]
fn phase_flow_recovery_skips_when_opponent_has_no_front_cards() {
    let rules = GameRules::default();
    let mut state = make_test_state(rules, 0);
    let registry = make_test_registry();
    state.players[0]
        .zones
        .cost_zone
        .push(test_card(51, "S999-C-001", 1));

    let p1 = PassAllClient;
    let p2 = PassAllClient;

    let _events = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

    assert_eq!(state.players[0].zones.cost_zone.len(), 1);
    assert_eq!(state.players[0].zones.hand.len(), 0);
}

#[test]
fn phase_flow_battle_attacker_higher_attack_destroys_defender_and_gains_rp() {
    let mut rules = GameRules::default();
    rules.first_player_draws = true;
    let mut state = make_test_state(rules, 2);
    let registry = make_test_registry();

    state.turn_number = 2;
    state.players[0].zones.front[0] = Some(test_card(60, "S999-C-001", 7));
    state.players[1].zones.front[0] = Some(test_card(61, "S999-C-002", 3));

    let p1 = ScriptedClient::new(
        vec![
            PhaseAction::Pass,
            PhaseAction::DeclareAttack {
                attacker: InstanceId(60),
                target_slot: Some(0),
            },
            PhaseAction::Pass,
        ],
        vec![],
    );
    let p2 = ScriptedClient::new(vec![], vec![]);

    let events = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

    assert!(events
        .iter()
        .any(|e| matches!(e, CoreGameEvent::CardDestroyed { instance_id } if *instance_id == InstanceId(61))));
    assert_eq!(state.players[0].real_point, 1);
}

#[test]
fn phase_flow_battle_tie_destroys_both_cards() {
    let mut rules = GameRules::default();
    rules.first_player_draws = true;
    let mut state = make_test_state(rules, 2);
    let registry = make_test_registry();

    state.turn_number = 2;
    state.players[0].zones.front[0] = Some(test_card(70, "S999-C-001", 4));
    state.players[1].zones.front[0] = Some(test_card(71, "S999-C-002", 4));

    let p1 = ScriptedClient::new(
        vec![
            PhaseAction::Pass,
            PhaseAction::DeclareAttack {
                attacker: InstanceId(70),
                target_slot: Some(0),
            },
            PhaseAction::Pass,
        ],
        vec![],
    );
    let p2 = ScriptedClient::new(vec![], vec![]);

    let _events = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

    assert_eq!(state.players[0].zones.grave.len(), 1);
    assert_eq!(state.players[1].zones.grave.len(), 1);
}

#[test]
fn phase_flow_battle_defender_higher_attack_destroys_attacker() {
    let mut rules = GameRules::default();
    rules.first_player_draws = true;
    let mut state = make_test_state(rules, 2);
    let registry = make_test_registry();

    state.turn_number = 2;
    state.players[0].zones.front[0] = Some(test_card(80, "S999-C-001", 2));
    state.players[1].zones.front[0] = Some(test_card(81, "S999-C-002", 5));

    let p1 = ScriptedClient::new(
        vec![
            PhaseAction::Pass,
            PhaseAction::DeclareAttack {
                attacker: InstanceId(80),
                target_slot: Some(0),
            },
            PhaseAction::Pass,
        ],
        vec![],
    );
    let p2 = ScriptedClient::new(vec![], vec![]);

    let _events = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

    assert!(state.players[0].zones.front[0].is_none());
    assert_eq!(state.players[0].zones.grave.len(), 1);
}

#[test]
fn action_draw_adds_cards_to_hand() {
    let rules = GameRules::default();
    let mut state = make_test_state(rules, 3);

    let events = ActionExecutor::execute_action(
        &Action::Draw {
            player: PlayerRef::Self_,
            count: 2,
        },
        &mut state,
        InstanceId(0),
        PlayerId::Player1,
        &[],
    );

    assert_eq!(state.players[0].zones.hand.len(), 2);
    assert_eq!(state.players[0].zones.deck.len(), 1);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, CoreGameEvent::DrawCard { .. }))
            .count(),
        2
    );
}

#[test]
fn action_draw_empty_deck_triggers_game_over() {
    let rules = GameRules::default();
    let mut state = make_test_state(rules, 0);

    let events = ActionExecutor::execute_action(
        &Action::Draw {
            player: PlayerRef::Self_,
            count: 1,
        },
        &mut state,
        InstanceId(0),
        PlayerId::Player1,
        &[],
    );

    assert!(has_game_over(&events, GameOverReason::DeckOut));
}

#[test]
fn action_damage_reduces_hp() {
    let rules = GameRules::default();
    let mut state = make_test_state(rules, 0);

    let events = ActionExecutor::execute_action(
        &Action::Damage {
            player: PlayerRef::Opponent,
            amount: 2,
        },
        &mut state,
        InstanceId(0),
        PlayerId::Player1,
        &[],
    );

    assert_eq!(state.players[1].hp, 3);
    assert!(events
        .iter()
        .any(|e| matches!(e, CoreGameEvent::HpChanged { new_hp: 3, .. })));
}

#[test]
fn action_damage_to_zero_triggers_game_over() {
    let rules = GameRules::default();
    let mut state = make_test_state(rules, 0);

    let events = ActionExecutor::execute_action(
        &Action::Damage {
            player: PlayerRef::Opponent,
            amount: 5,
        },
        &mut state,
        InstanceId(0),
        PlayerId::Player1,
        &[],
    );

    assert_eq!(state.players[1].hp, 0);
    assert!(has_game_over(&events, GameOverReason::HpZero));
}

#[test]
fn action_gain_real_point_overflow_emits_event() {
    let rules = GameRules::default();
    let mut state = make_test_state(rules, 0);
    state.players[0].real_point = 5;

    let events = ActionExecutor::execute_action(
        &Action::GainRealPoint {
            player: PlayerRef::Self_,
            amount: 3,
        },
        &mut state,
        InstanceId(0),
        PlayerId::Player1,
        &[],
    );

    assert_eq!(state.players[0].real_point, 6);
    assert!(events.iter().any(|e| matches!(
        e,
        CoreGameEvent::RealPointOverflow {
            player: PlayerId::Player1
        }
    )));
}

#[test]
fn action_heal_hp_capped_at_max() {
    let rules = GameRules::default();
    let mut state = make_test_state(rules, 0);
    state.players[0].hp = 5;

    let _events = ActionExecutor::execute_action(
        &Action::HealHp {
            player: PlayerRef::Self_,
            amount: 9,
        },
        &mut state,
        InstanceId(0),
        PlayerId::Player1,
        &[],
    );

    assert_eq!(state.players[0].hp, 6);
}

#[test]
fn action_destroy_moves_card_to_grave() {
    let rules = GameRules::default();
    let mut state = make_test_state(rules, 0);
    state.players[0].zones.front[0] = Some(test_card(91, "S999-C-001", 3));

    let events = ActionExecutor::execute_action(
        &Action::Destroy {
            target: CardRef::ByInstanceId(InstanceId(91)),
        },
        &mut state,
        InstanceId(0),
        PlayerId::Player1,
        &[],
    );

    assert!(state.players[0].zones.front[0].is_none());
    assert_eq!(state.players[0].zones.grave.len(), 1);
    assert!(events
        .iter()
        .any(|e| matches!(e, CoreGameEvent::CardDestroyed { instance_id } if *instance_id == InstanceId(91))));
}

#[test]
fn action_destroy_respects_immunity_to_destruction() {
    let rules = GameRules::default();
    let mut state = make_test_state(rules, 0);
    let mut immune_card = test_card(92, "S999-C-001", 3);
    ModifierManager::apply_modifier(
        &mut immune_card,
        Modifier::ImmuneToDestruction,
        InstanceId(999),
        ModifierDuration::Permanent,
    );
    state.players[0].zones.front[0] = Some(immune_card);

    let events = ActionExecutor::execute_action(
        &Action::Destroy {
            target: CardRef::ByInstanceId(InstanceId(92)),
        },
        &mut state,
        InstanceId(0),
        PlayerId::Player1,
        &[],
    );

    assert!(state.players[0].zones.front[0].is_some());
    assert!(events.is_empty());
}

#[test]
fn modifier_attack_boost_modifies_current_attack() {
    let mut card = test_card(101, "S999-C-001", 5);
    ModifierManager::apply_modifier(
        &mut card,
        Modifier::AttackBoost(3),
        InstanceId(888),
        ModifierDuration::UntilEndOfTurn,
    );

    assert_eq!(card.current_attack, Some(8));
}

#[test]
fn modifier_until_end_of_turn_cleanup_removes_modifier() {
    let mut state = make_test_state(GameRules::default(), 0);
    let mut card = test_card(102, "S999-C-001", 5);
    ModifierManager::apply_modifier(
        &mut card,
        Modifier::AttackBoost(2),
        InstanceId(888),
        ModifierDuration::UntilEndOfTurn,
    );
    state.players[0].zones.front[0] = Some(card);

    let _events = ModifierManager::cleanup_expired(&mut state);

    let card_after = state.players[0].zones.front[0]
        .as_ref()
        .expect("card should remain on field");
    assert_eq!(card_after.current_attack, Some(5));
    assert_eq!(card_after.modifiers.len(), 0);
}

#[test]
fn modifier_while_source_on_field_cleans_when_source_leaves() {
    let mut state = make_test_state(GameRules::default(), 0);
    let source_id = InstanceId(300);

    let mut target = test_card(103, "S999-C-001", 5);
    ModifierManager::apply_modifier(
        &mut target,
        Modifier::AttackBoost(2),
        source_id,
        ModifierDuration::WhileSourceOnField,
    );
    state.players[0].zones.front[0] = Some(target);
    state.players[0].zones.front[1] = Some(test_card(300, "S999-C-002", 1));

    let _ = ModifierManager::cleanup_expired(&mut state);
    assert_eq!(
        state.players[0].zones.front[0]
            .as_ref()
            .expect("target exists")
            .modifiers
            .len(),
        1
    );

    let removed = state.players[0].zones.front[1]
        .take()
        .expect("source exists");
    state.players[0].zones.grave.push(removed);
    let _ = ModifierManager::cleanup_expired(&mut state);

    let card_after = state.players[0].zones.front[0]
        .as_ref()
        .expect("target exists after cleanup");
    assert_eq!(card_after.modifiers.len(), 0);
    assert_eq!(card_after.current_attack, Some(5));
}

#[test]
fn modifier_immunity_check_blocks_targeting() {
    let mut card = test_card(104, "S999-C-001", 5);
    ModifierManager::apply_modifier(
        &mut card,
        Modifier::ImmuneToTargeting,
        InstanceId(777),
        ModifierDuration::Permanent,
    );

    assert!(ModifierManager::has_immunity(
        &card,
        &card_core::ImmunityCheck::Targeting
    ));
}

#[test]
fn modifier_permanent_modifier_never_expires_on_cleanup() {
    let mut state = make_test_state(GameRules::default(), 0);
    let mut card = test_card(105, "S999-C-001", 5);
    ModifierManager::apply_modifier(
        &mut card,
        Modifier::AttackBoost(4),
        InstanceId(777),
        ModifierDuration::Permanent,
    );
    state.players[0].zones.front[0] = Some(card);

    let _ = ModifierManager::cleanup_expired(&mut state);

    let card_after = state.players[0].zones.front[0]
        .as_ref()
        .expect("card should stay on field");
    assert_eq!(card_after.modifiers.len(), 1);
    assert_eq!(card_after.current_attack, Some(9));
}

#[test]
fn modifier_turn_count_decrements_and_expires() {
    let mut state = make_test_state(GameRules::default(), 0);
    let mut card = test_card(106, "S999-C-001", 5);
    ModifierManager::apply_modifier(
        &mut card,
        Modifier::AttackBoost(1),
        InstanceId(700),
        ModifierDuration::TurnCount(1),
    );
    state.players[0].zones.front[0] = Some(card);

    ModifierManager::decrement_turn_counts(&mut state);
    ModifierManager::cleanup_expired(&mut state);

    let card_after = state.players[0].zones.front[0]
        .as_ref()
        .expect("card should remain on field");
    assert_eq!(card_after.modifiers.len(), 0);
    assert_eq!(card_after.current_attack, Some(5));
}

#[test]
fn trigger_matches_on_attack_event() {
    let event = CoreGameEvent::AttackDeclared {
        attacker: InstanceId(120),
    };
    assert!(TriggerChecker::trigger_matches(
        &Trigger::OnAttack,
        &event,
        InstanceId(120)
    ));
}

#[test]
fn trigger_matches_draw_phase_event() {
    let event = CoreGameEvent::PhaseChanged {
        new_phase: Phase::Draw,
    };
    assert!(TriggerChecker::trigger_matches(
        &Trigger::DrawPhase,
        &event,
        InstanceId(1)
    ));
}

#[test]
fn trigger_matches_real_point_overflow_event() {
    let event = CoreGameEvent::RealPointOverflow {
        player: PlayerId::Player1,
    };
    assert!(TriggerChecker::trigger_matches(
        &Trigger::OnEvent(EventTrigger::RealPointOverflow {
            player: PlayerRef::Self_,
        }),
        &event,
        InstanceId(1)
    ));
}

#[test]
fn trigger_once_per_turn_limit_blocks_reactivation() {
    let mut state = make_test_state(GameRules::default(), 0);
    let source = InstanceId(130);
    let key = EffectKey("once".to_string());

    assert!(TriggerChecker::is_activation_allowed(
        &Some(ActivationLimit::OncePerTurn),
        &state,
        source,
        &key
    ));

    state.activated_this_turn.insert((source, key.clone()));

    assert!(!TriggerChecker::is_activation_allowed(
        &Some(ActivationLimit::OncePerTurn),
        &state,
        source,
        &key
    ));
}

#[test]
fn trigger_once_per_turn_same_name_blocks_other_instance() {
    let mut state = make_test_state(GameRules::default(), 0);
    let key = EffectKey("same_name".to_string());

    let id1 = InstanceId(140);
    let id2 = InstanceId(141);
    let def = CardId::new("S999-C-001");

    state.players[0]
        .zones
        .hand
        .push(CardInstance::new(id1, def.clone(), Some(1)));
    state.players[1]
        .zones
        .hand
        .push(CardInstance::new(id2, def, Some(1)));
    state.activated_this_turn.insert((id1, key.clone()));

    assert!(!TriggerChecker::is_activation_allowed(
        &Some(ActivationLimit::OncePerTurnSameName),
        &state,
        id2,
        &key
    ));
}

#[test]
fn chain_resolution_executes_actions_and_completes() {
    let mut state = make_test_state(GameRules::default(), 1);
    let entry = ChainEntry {
        source: InstanceId(500),
        effect_key: EffectKey("draw".to_string()),
        actions: vec![Action::Draw {
            player: PlayerRef::Self_,
            count: 1,
        }],
        controller: PlayerId::Player1,
    };

    let events = ChainManager::resolve_chain(&mut state, entry, &CardRegistryImpl::new());

    assert!(events
        .iter()
        .any(|e| matches!(e, CoreGameEvent::ChainStarted)));
    assert!(events
        .iter()
        .any(|e| matches!(e, CoreGameEvent::ChainComplete)));
    assert_eq!(state.players[0].zones.hand.len(), 1);
}

#[test]
fn chain_resolution_stops_after_game_over() {
    let mut state = make_test_state(GameRules::default(), 0);
    state.players[1].hp = 1;
    let entry = ChainEntry {
        source: InstanceId(501),
        effect_key: EffectKey("burn".to_string()),
        actions: vec![Action::Damage {
            player: PlayerRef::Opponent,
            amount: 1,
        }],
        controller: PlayerId::Player1,
    };

    let events = ChainManager::resolve_chain(&mut state, entry, &CardRegistryImpl::new());

    assert!(has_game_over(&events, GameOverReason::HpZero));
    assert!(events
        .iter()
        .any(|e| matches!(e, CoreGameEvent::ChainComplete)));
}

#[test]
fn chain_push_link_adds_entry_and_emits_chain_link_event() {
    let mut stack = Vec::new();
    let mut events = Vec::new();
    let entry = ChainEntry {
        source: InstanceId(502),
        effect_key: EffectKey("link".to_string()),
        actions: vec![],
        controller: PlayerId::Player1,
    };

    ChainManager::push_link(&mut stack, entry.clone(), &mut events);

    assert_eq!(stack.len(), 1);
    assert_eq!(stack[0].source, entry.source);
    assert!(events
        .iter()
        .any(|e| matches!(e, CoreGameEvent::ChainLink { instance_id, .. } if *instance_id == entry.source)));
}

#[test]
fn chain_resolution_emits_resolving_index_zero() {
    let mut state = make_test_state(GameRules::default(), 1);
    let entry = ChainEntry {
        source: InstanceId(503),
        effect_key: EffectKey("draw2".to_string()),
        actions: vec![Action::Draw {
            player: PlayerRef::Self_,
            count: 1,
        }],
        controller: PlayerId::Player1,
    };

    let events = ChainManager::resolve_chain(&mut state, entry, &CardRegistryImpl::new());

    assert!(events
        .iter()
        .any(|e| matches!(e, CoreGameEvent::ChainResolving { link_index: 0 })));
}

#[test]
fn chain_resolution_draw_from_empty_deck_still_completes_chain() {
    let mut state = make_test_state(GameRules::default(), 0);
    let entry = ChainEntry {
        source: InstanceId(504),
        effect_key: EffectKey("deckout".to_string()),
        actions: vec![Action::Draw {
            player: PlayerRef::Self_,
            count: 1,
        }],
        controller: PlayerId::Player1,
    };

    let events = ChainManager::resolve_chain(&mut state, entry, &CardRegistryImpl::new());

    assert!(has_game_over(&events, GameOverReason::DeckOut));
    assert!(events
        .iter()
        .any(|e| matches!(e, CoreGameEvent::ChainComplete)));
}

#[test]
fn engine_pass_all_client_game_completes() {
    let mut rules = GameRules::default();
    rules.first_player_draws = true;
    let state = make_test_state(rules, 1);
    let registry = make_test_registry();
    let engine = GameEngine::new(
        state,
        registry,
        Box::new(PassAllClient),
        Box::new(PassAllClient),
    );

    let result = engine.run();

    assert!(matches!(result.reason, GameOverReason::DeckOut));
}

#[test]
fn engine_game_ends_within_max_turns() {
    let mut rules = GameRules::default();
    rules.first_player_draws = true;
    let state = make_test_state(rules, 2);
    let registry = make_test_registry();
    let engine = GameEngine::new(
        state,
        registry,
        Box::new(PassAllClient),
        Box::new(PassAllClient),
    );

    let result = engine.run();

    assert!(result.final_state.turn_number <= 500);
}

#[test]
fn engine_event_log_non_empty_after_game_completion() {
    let mut rules = GameRules::default();
    rules.first_player_draws = true;
    let state = make_test_state(rules, 1);
    let registry = make_test_registry();
    let engine = GameEngine::new(
        state,
        registry,
        Box::new(PassAllClient),
        Box::new(PassAllClient),
    );

    let result = engine.run();

    assert!(!result.event_log.is_empty());
}

#[test]
fn engine_winner_is_set_on_completion() {
    let mut rules = GameRules::default();
    rules.first_player_draws = true;
    let state = make_test_state(rules, 1);
    let registry = make_test_registry();
    let engine = GameEngine::new(
        state,
        registry,
        Box::new(PassAllClient),
        Box::new(PassAllClient),
    );

    let result = engine.run();

    assert!(result.winner.is_some());
}

#[test]
fn engine_same_seed_produces_same_result() {
    let mut rules = GameRules::default();
    rules.first_player_draws = true;

    let state_a = make_test_state(rules.clone(), 2);
    let state_b = make_test_state(rules, 2);
    let registry_a = make_test_registry();
    let registry_b = make_test_registry();

    let result_a = GameEngine::new(
        state_a,
        registry_a,
        Box::new(PassAllClient),
        Box::new(PassAllClient),
    )
    .run();
    let result_b = GameEngine::new(
        state_b,
        registry_b,
        Box::new(PassAllClient),
        Box::new(PassAllClient),
    )
    .run();

    assert_eq!(result_a.reason, result_b.reason);
    assert_eq!(result_a.winner, result_b.winner);
    assert_eq!(
        result_a.final_state.turn_number,
        result_b.final_state.turn_number
    );
    assert_eq!(result_a.event_log, result_b.event_log);
}
