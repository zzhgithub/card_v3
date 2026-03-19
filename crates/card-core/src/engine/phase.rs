use std::time::Duration;

use crate::engine::modifier::ModifierManager;
use crate::state::events::GameOverReason;
use crate::state::{CardRegistryImpl, CoreGameEvent, GameState, Phase};
use crate::types::{CardId, InstanceId, PlayerId, Zone, ZoneLocation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseAction {
    PlayCard {
        instance_id: InstanceId,
        target_zone: Zone,
        cost_payment: Vec<InstanceId>,
    },
    DeclareAttack {
        attacker: InstanceId,
        target_slot: Option<usize>,
    },
    Pass,
    Surrender,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryCardOption {
    pub instance_id: InstanceId,
    pub definition_id: CardId,
}

pub trait PhaseClient: Send + Sync {
    fn choose_action(&self, _available: &[PhaseAction], _timeout: Duration) -> Option<PhaseAction>;

    fn choose_recovery_cards(
        &self,
        _options: &[RecoveryCardOption],
        _count: usize,
        _timeout: Duration,
    ) -> Option<Vec<InstanceId>>;
}

pub struct PhaseRunner;

impl PhaseRunner {
    pub fn run_turn(
        state: &mut GameState,
        registry: &CardRegistryImpl,
        client1: &dyn PhaseClient,
        client2: &dyn PhaseClient,
    ) -> Vec<CoreGameEvent> {
        let mut events = Vec::new();
        let timeout = state.rules.operation_timeout;

        // Helper: get client for current turn player
        let current_client = |state: &GameState| -> &dyn PhaseClient {
            if state.turn_player == PlayerId::Player1 {
                client1
            } else {
                client2
            }
        };

        // ── TurnStart ────────────────────────────────────────────────────────
        state.phase = Phase::TurnStart;
        state.activated_this_turn.clear();
        events.push(CoreGameEvent::PhaseChanged {
            new_phase: Phase::TurnStart,
        });

        // ── Draw ─────────────────────────────────────────────────────────────
        state.phase = Phase::Draw;
        events.push(CoreGameEvent::PhaseChanged {
            new_phase: Phase::Draw,
        });

        let skip_draw = state.turn_number == 1 && !state.rules.first_player_draws;
        if !skip_draw {
            let player_idx = player_idx(state.turn_player);
            if state.players[player_idx].zones.deck.is_empty() {
                events.push(CoreGameEvent::GameOver {
                    winner: Some(state.turn_player.opponent()),
                    reason: GameOverReason::DeckOut,
                });
                return events;
            }
            let card = state.players[player_idx].zones.deck.remove(0);
            let iid = card.instance_id;
            state.players[player_idx].zones.hand.push(card);
            events.push(CoreGameEvent::DrawCard {
                player: state.turn_player,
                instance_id: iid,
            });
        }

        // ── Recovery ─────────────────────────────────────────────────────────
        state.phase = Phase::Recovery;
        events.push(CoreGameEvent::PhaseChanged {
            new_phase: Phase::Recovery,
        });

        let recovery_count = Self::calc_recovery_count(state, registry);
        if recovery_count > 0 {
            let player_idx = player_idx(state.turn_player);
            let options: Vec<RecoveryCardOption> = state.players[player_idx]
                .zones
                .cost_zone
                .iter()
                .map(|c| RecoveryCardOption {
                    instance_id: c.instance_id,
                    definition_id: c.definition_id.clone(),
                })
                .collect();

            let client = if state.turn_player == PlayerId::Player1 {
                client1
            } else {
                client2
            };
            let chosen = client
                .choose_recovery_cards(&options, recovery_count, timeout)
                .unwrap_or_default();

            for iid in chosen {
                if let Some((card, _)) = state.players[player_idx].zones.remove_card(iid) {
                    let to = ZoneLocation::new(state.turn_player, Zone::Hand);
                    if state.players[player_idx].zones.hand.len() < state.rules.max_hand {
                        state.players[player_idx].zones.hand.push(card);
                    }
                    events.push(CoreGameEvent::CardMoved {
                        instance_id: iid,
                        from: ZoneLocation::new(state.turn_player, Zone::CostZone),
                        to,
                    });
                }
            }
        }

        // ── Main1 ─────────────────────────────────────────────────────────────
        state.phase = Phase::Main1;
        events.push(CoreGameEvent::PhaseChanged {
            new_phase: Phase::Main1,
        });
        let game_over = Self::run_main_phase(state, client1, client2, timeout, &mut events);
        if game_over {
            return events;
        }

        // ── Battle ────────────────────────────────────────────────────────────
        state.phase = Phase::Battle;
        events.push(CoreGameEvent::PhaseChanged {
            new_phase: Phase::Battle,
        });
        let game_over = Self::run_battle_phase(state, client1, client2, timeout, &mut events);
        if game_over {
            return events;
        }

        // ── Main2 ─────────────────────────────────────────────────────────────
        state.phase = Phase::Main2;
        events.push(CoreGameEvent::PhaseChanged {
            new_phase: Phase::Main2,
        });
        let game_over = Self::run_main_phase(state, client1, client2, timeout, &mut events);
        if game_over {
            return events;
        }

        // ── TurnEnd ───────────────────────────────────────────────────────────
        state.phase = Phase::TurnEnd;
        events.push(CoreGameEvent::PhaseChanged {
            new_phase: Phase::TurnEnd,
        });

        // Cleanup UntilEndOfTurn modifiers
        ModifierManager::cleanup_expired(state);

        // Reset attacked_this_turn for all field cards
        for p_idx in 0..2 {
            for slot in 0..5 {
                if let Some(card) = state.players[p_idx].zones.front[slot].as_mut() {
                    card.attacked_this_turn = false;
                }
                if let Some(card) = state.players[p_idx].zones.back[slot].as_mut() {
                    card.attacked_this_turn = false;
                }
            }
        }

        // Switch turn player
        let next_player = state.turn_player.opponent();
        state.turn_player = next_player;
        state.turn_number += 1;
        events.push(CoreGameEvent::TurnChanged {
            new_active_player: next_player,
            turn_number: state.turn_number,
        });

        events
    }

    fn run_main_phase(
        state: &mut GameState,
        client1: &dyn PhaseClient,
        client2: &dyn PhaseClient,
        timeout: Duration,
        events: &mut Vec<CoreGameEvent>,
    ) -> bool {
        let client = if state.turn_player == PlayerId::Player1 {
            client1
        } else {
            client2
        };
        loop {
            let available = Self::build_main_actions(state);
            let action = match client.choose_action(&available, timeout) {
                Some(a) => a,
                None => {
                    events.push(CoreGameEvent::GameOver {
                        winner: Some(state.turn_player.opponent()),
                        reason: GameOverReason::Timeout,
                    });
                    return true;
                }
            };

            match action {
                PhaseAction::Pass => break,
                PhaseAction::Surrender => {
                    events.push(CoreGameEvent::GameOver {
                        winner: Some(state.turn_player.opponent()),
                        reason: GameOverReason::Surrender,
                    });
                    return true;
                }
                PhaseAction::PlayCard {
                    instance_id,
                    target_zone,
                    cost_payment,
                } => {
                    let player_idx = player_idx(state.turn_player);
                    // Pay costs: move cost cards from hand to cost_zone
                    for cost_iid in &cost_payment {
                        if let Some((card, _)) =
                            state.players[player_idx].zones.remove_card(*cost_iid)
                        {
                            let iid = card.instance_id;
                            state.players[player_idx].zones.cost_zone.push(card);
                            events.push(CoreGameEvent::CardExposed { instance_id: iid });
                        }
                    }
                    // Move card from hand to target zone
                    if let Some((card, _)) =
                        state.players[player_idx].zones.remove_card(instance_id)
                    {
                        let to = ZoneLocation::new(state.turn_player, target_zone.clone());
                        let _ = state.players[player_idx]
                            .zones
                            .add_card_to_zone(card, target_zone);
                        events.push(CoreGameEvent::CardSummoned { instance_id, to });
                    }
                }
                PhaseAction::DeclareAttack { .. } => {
                    // Attack declarations not valid in main phase — treat as Pass
                    break;
                }
            }
        }
        false
    }

    fn run_battle_phase(
        state: &mut GameState,
        client1: &dyn PhaseClient,
        client2: &dyn PhaseClient,
        timeout: Duration,
        events: &mut Vec<CoreGameEvent>,
    ) -> bool {
        let turn_player = state.turn_player;
        let client = if turn_player == PlayerId::Player1 {
            client1
        } else {
            client2
        };
        let p_idx = player_idx(turn_player);
        let opp_idx = 1 - p_idx;

        // Collect attackable cards (front field, not yet attacked)
        let attackers: Vec<InstanceId> = state.players[p_idx]
            .zones
            .front
            .iter()
            .flatten()
            .filter(|c| !c.attacked_this_turn)
            .map(|c| c.instance_id)
            .collect();

        for attacker_id in attackers {
            // Build attack options
            let mut available = Vec::new();
            let has_opp_front = state.players[opp_idx]
                .zones
                .front
                .iter()
                .any(|s| s.is_some());

            if has_opp_front {
                for slot in 0..5 {
                    if state.players[opp_idx].zones.front[slot].is_some() {
                        available.push(PhaseAction::DeclareAttack {
                            attacker: attacker_id,
                            target_slot: Some(slot),
                        });
                    }
                }
            } else {
                available.push(PhaseAction::DeclareAttack {
                    attacker: attacker_id,
                    target_slot: None,
                });
            }
            available.push(PhaseAction::Pass);

            let action = match client.choose_action(&available, timeout) {
                Some(a) => a,
                None => {
                    events.push(CoreGameEvent::GameOver {
                        winner: Some(turn_player.opponent()),
                        reason: GameOverReason::Timeout,
                    });
                    return true;
                }
            };

            match action {
                PhaseAction::DeclareAttack {
                    attacker,
                    target_slot,
                } => {
                    events.push(CoreGameEvent::AttackDeclared { attacker });

                    // Mark attacker as having attacked
                    if let Some(card) = state.players[p_idx]
                        .zones
                        .front
                        .iter_mut()
                        .flatten()
                        .find(|c| c.instance_id == attacker)
                    {
                        card.attacked_this_turn = true;
                    }

                    let attacker_power = state.players[p_idx]
                        .zones
                        .front
                        .iter()
                        .flatten()
                        .find(|c| c.instance_id == attacker)
                        .and_then(|c| c.current_attack)
                        .unwrap_or(0);

                    match target_slot {
                        None => {
                            // Direct attack
                            let old_hp = state.players[opp_idx].hp;
                            let dmg = (attacker_power as u8).min(old_hp);
                            let new_hp = old_hp - dmg;
                            state.players[opp_idx].hp = new_hp;
                            events.push(CoreGameEvent::HpChanged {
                                player: turn_player.opponent(),
                                old_hp,
                                new_hp,
                            });
                            if new_hp == 0 {
                                events.push(CoreGameEvent::GameOver {
                                    winner: Some(turn_player),
                                    reason: GameOverReason::HpZero,
                                });
                                return true;
                            }
                        }
                        Some(slot) => {
                            let defender_power = state.players[opp_idx].zones.front[slot]
                                .as_ref()
                                .and_then(|c| c.current_attack)
                                .unwrap_or(0);

                            if attacker_power > defender_power {
                                // Attacker wins: defender to grave
                                if let Some(card) = state.players[opp_idx].zones.front[slot].take()
                                {
                                    let iid = card.instance_id;
                                    state.players[opp_idx].zones.grave.push(card);
                                    events.push(CoreGameEvent::CardDestroyed { instance_id: iid });
                                }
                                // Attacker gains 1 RP
                                let old_rp = state.players[p_idx].real_point;
                                let new_rp = (old_rp + 1).min(state.rules.max_real_point);
                                state.players[p_idx].real_point = new_rp;
                                events.push(CoreGameEvent::RealPointChanged {
                                    player: turn_player,
                                    old_rp,
                                    new_rp,
                                });
                                if new_rp >= state.rules.max_real_point {
                                    events.push(CoreGameEvent::RealPointOverflow {
                                        player: turn_player,
                                    });
                                }
                            } else if attacker_power < defender_power {
                                // Defender wins: attacker to grave
                                if let Some(card) = state.players[p_idx]
                                    .zones
                                    .front
                                    .iter_mut()
                                    .find(|s| s.as_ref().map(|c| c.instance_id) == Some(attacker))
                                    .and_then(|s| s.take())
                                {
                                    let iid = card.instance_id;
                                    state.players[p_idx].zones.grave.push(card);
                                    events.push(CoreGameEvent::CardDestroyed { instance_id: iid });
                                }
                            } else {
                                // Tie: both to grave
                                if let Some(card) = state.players[opp_idx].zones.front[slot].take()
                                {
                                    let iid = card.instance_id;
                                    state.players[opp_idx].zones.grave.push(card);
                                    events.push(CoreGameEvent::CardDestroyed { instance_id: iid });
                                }
                                if let Some(card) = state.players[p_idx]
                                    .zones
                                    .front
                                    .iter_mut()
                                    .find(|s| s.as_ref().map(|c| c.instance_id) == Some(attacker))
                                    .and_then(|s| s.take())
                                {
                                    let iid = card.instance_id;
                                    state.players[p_idx].zones.grave.push(card);
                                    events.push(CoreGameEvent::CardDestroyed { instance_id: iid });
                                }
                            }
                        }
                    }
                }
                PhaseAction::Pass | PhaseAction::Surrender => break,
                _ => break,
            }
        }
        false
    }

    fn build_main_actions(state: &GameState) -> Vec<PhaseAction> {
        let mut actions = vec![PhaseAction::Pass, PhaseAction::Surrender];
        let p_idx = player_idx(state.turn_player);
        // Add PlayCard for each hand card (simplified: no cost check for now)
        for card in &state.players[p_idx].zones.hand {
            actions.push(PhaseAction::PlayCard {
                instance_id: card.instance_id,
                target_zone: Zone::Front(0),
                cost_payment: vec![],
            });
        }
        actions
    }

    fn calc_recovery_count(state: &GameState, registry: &CardRegistryImpl) -> usize {
        let opp_idx = 1 - player_idx(state.turn_player);
        state.players[opp_idx]
            .zones
            .front
            .iter()
            .flatten()
            .filter_map(|card| registry.get(&card.definition_id))
            .map(|def| def.cost as usize)
            .max()
            .unwrap_or(0)
    }
}

fn player_idx(player_id: PlayerId) -> usize {
    match player_id {
        PlayerId::Player1 => 0,
        PlayerId::Player2 => 1,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use std::time::Duration;

    use crate::rules::GameRules;
    use crate::state::{CardInstance, CardRegistryImpl, CoreGameEvent, GameState, Phase};
    use crate::types::{
        CardDefinition, CardId, CardType, Category, InstanceId, PlayerId, Property,
    };

    use super::{PhaseAction, PhaseClient, PhaseRunner, RecoveryCardOption};

    struct ScriptedClient {
        actions: Mutex<VecDeque<PhaseAction>>,
        recovery: Mutex<VecDeque<Vec<InstanceId>>>,
    }

    impl ScriptedClient {
        fn new(actions: Vec<PhaseAction>) -> Self {
            Self {
                actions: Mutex::new(actions.into()),
                recovery: Mutex::new(VecDeque::new()),
            }
        }
    }

    impl PhaseClient for ScriptedClient {
        fn choose_action(
            &self,
            _available: &[PhaseAction],
            _timeout: Duration,
        ) -> Option<PhaseAction> {
            self.actions.lock().expect("actions lock").pop_front()
        }

        fn choose_recovery_cards(
            &self,
            _options: &[RecoveryCardOption],
            _count: usize,
            _timeout: Duration,
        ) -> Option<Vec<InstanceId>> {
            self.recovery
                .lock()
                .expect("recovery lock")
                .pop_front()
                .or_else(|| Some(Vec::new()))
        }
    }

    fn make_state() -> GameState {
        GameState::new(GameRules::default(), 42)
    }

    fn card(instance: u32, def: &str, attack: i32) -> CardInstance {
        CardInstance::new(InstanceId(instance), CardId::new(def), Some(attack))
    }

    fn add_basic_definitions(registry: &mut CardRegistryImpl) {
        registry.insert(CardDefinition::new(
            CardId::new("S000-C-001"),
            "A".into(),
            CardType::Character,
            Property::Rational,
            Category::Math,
            1,
        ));
        registry.insert(CardDefinition::new(
            CardId::new("S000-C-002"),
            "B".into(),
            CardType::Character,
            Property::Rational,
            Category::Math,
            1,
        ));
    }

    #[test]
    fn test_draw_phase_first_turn_no_draw() {
        let mut state = make_state();
        state.players[0].zones.deck.push(card(1, "S000-C-001", 100));

        let mut registry = CardRegistryImpl::new();
        add_basic_definitions(&mut registry);

        let p1 = ScriptedClient::new(vec![
            PhaseAction::Pass,
            PhaseAction::Pass,
            PhaseAction::Pass,
        ]);
        let p2 = ScriptedClient::new(vec![]);
        let before_hand = state.players[0].zones.hand.len();

        let _ = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

        assert_eq!(state.players[0].zones.hand.len(), before_hand);
    }

    #[test]
    fn test_draw_phase_second_turn_draws() {
        let mut state = make_state();
        state.turn_number = 2;
        state.players[0].zones.deck.push(card(1, "S000-C-001", 100));

        let mut registry = CardRegistryImpl::new();
        add_basic_definitions(&mut registry);

        let p1 = ScriptedClient::new(vec![
            PhaseAction::Pass,
            PhaseAction::Pass,
            PhaseAction::Pass,
        ]);
        let p2 = ScriptedClient::new(vec![]);

        let _ = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

        assert_eq!(state.players[0].zones.hand.len(), 1);
    }

    #[test]
    fn test_battle_higher_attack_wins() {
        let mut state = make_state();
        state.turn_number = 2;
        state.players[0]
            .zones
            .deck
            .push(card(99, "S000-C-001", 100));
        state.players[0].zones.front[0] = Some(card(10, "S000-C-001", 500));
        state.players[1].zones.front[0] = Some(card(20, "S000-C-002", 300));

        let mut registry = CardRegistryImpl::new();
        add_basic_definitions(&mut registry);

        let p1 = ScriptedClient::new(vec![
            PhaseAction::Pass,
            PhaseAction::DeclareAttack {
                attacker: InstanceId(10),
                target_slot: Some(0),
            },
            PhaseAction::Pass,
        ]);
        let p2 = ScriptedClient::new(vec![]);

        let _ = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

        assert!(state.players[0].zones.front[0].is_some());
        assert!(state.players[1].zones.front[0].is_none());
        assert_eq!(state.players[1].zones.grave.len(), 1);
        assert_eq!(state.players[0].real_point, 1);
    }

    #[test]
    fn test_battle_tie_both_destroyed() {
        let mut state = make_state();
        state.turn_number = 2;
        state.players[0]
            .zones
            .deck
            .push(card(99, "S000-C-001", 100));
        state.players[0].zones.front[0] = Some(card(10, "S000-C-001", 500));
        state.players[1].zones.front[0] = Some(card(20, "S000-C-002", 500));

        let mut registry = CardRegistryImpl::new();
        add_basic_definitions(&mut registry);

        let p1 = ScriptedClient::new(vec![
            PhaseAction::Pass,
            PhaseAction::DeclareAttack {
                attacker: InstanceId(10),
                target_slot: Some(0),
            },
            PhaseAction::Pass,
        ]);
        let p2 = ScriptedClient::new(vec![]);

        let _ = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

        assert!(state.players[0].zones.front[0].is_none());
        assert!(state.players[1].zones.front[0].is_none());
        assert_eq!(state.players[0].zones.grave.len(), 1);
        assert_eq!(state.players[1].zones.grave.len(), 1);
    }

    #[test]
    fn test_turn_end_switches_player() {
        let mut state = make_state();
        state.turn_number = 2;
        state.players[0]
            .zones
            .deck
            .push(card(99, "S000-C-001", 100));

        let mut registry = CardRegistryImpl::new();
        add_basic_definitions(&mut registry);

        let p1 = ScriptedClient::new(vec![
            PhaseAction::Pass,
            PhaseAction::Pass,
            PhaseAction::Pass,
        ]);
        let p2 = ScriptedClient::new(vec![]);

        let _ = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

        assert_eq!(state.turn_player, PlayerId::Player2);
    }

    #[test]
    fn test_phase_changed_events_emitted() {
        let mut state = make_state();
        state.turn_number = 2;
        state.players[0]
            .zones
            .deck
            .push(card(99, "S000-C-001", 100));

        let mut registry = CardRegistryImpl::new();
        add_basic_definitions(&mut registry);

        let p1 = ScriptedClient::new(vec![
            PhaseAction::Pass,
            PhaseAction::Pass,
            PhaseAction::Pass,
        ]);
        let p2 = ScriptedClient::new(vec![]);

        let events = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);
        let phases: Vec<Phase> = events
            .iter()
            .filter_map(|ev| match ev {
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
    }
}
