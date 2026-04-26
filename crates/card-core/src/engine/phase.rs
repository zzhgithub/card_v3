use std::time::Duration;

use crate::engine::modifier::ModifierManager;
use crate::state::events::GameOverReason;
use crate::state::{CardRegistryImpl, CoreGameEvent, GameState, Phase};
use crate::types::{CardId, CardType, InstanceId, PlayerId, Zone, ZoneLocation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseAction {
    PlayCard {
        instance_id: InstanceId,
        target_zone: Zone,
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

    /// Called after a turn (and any follow-up effects) completes, with the
    /// events produced and the current game state.
    fn on_events(&self, _events: &[CoreGameEvent], _state: &crate::state::GameState) {}
}

pub struct PhaseRunner;

impl PhaseRunner {
    #[inline]
    fn notify_clients(
        state: &GameState,
        events: &[CoreGameEvent],
        client1: &dyn PhaseClient,
        client2: &dyn PhaseClient,
    ) {
        client1.on_events(events, state);
        client2.on_events(events, state);
    }

    pub fn run_turn(
        state: &mut GameState,
        registry: &CardRegistryImpl,
        client1: &dyn PhaseClient,
        client2: &dyn PhaseClient,
    ) -> Vec<CoreGameEvent> {
        let mut events = Vec::new();
        let timeout = state.rules.operation_timeout;

        // Helper: get client for current turn player
        let _current_client = |state: &GameState| -> &dyn PhaseClient {
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
        Self::notify_clients(state, &events, client1, client2);

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
                Self::notify_clients(state, &events, client1, client2);
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
        Self::notify_clients(state, &events, client1, client2);

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
        Self::notify_clients(state, &events, client1, client2);

        // ── Main1 ─────────────────────────────────────────────────────────────
        state.phase = Phase::Main1;
        events.push(CoreGameEvent::PhaseChanged {
            new_phase: Phase::Main1,
        });
        let game_over = Self::run_main_phase(state, registry, client1, client2, timeout, &mut events);
        Self::notify_clients(state, &events, client1, client2);
        if game_over {
            return events;
        }

        // ── Battle ────────────────────────────────────────────────────────────
        state.phase = Phase::Battle;
        events.push(CoreGameEvent::PhaseChanged {
            new_phase: Phase::Battle,
        });
        let game_over = Self::run_battle_phase(state, client1, client2, timeout, &mut events);
        Self::notify_clients(state, &events, client1, client2);
        if game_over {
            return events;
        }

        // ── Main2 ─────────────────────────────────────────────────────────────
        state.phase = Phase::Main2;
        events.push(CoreGameEvent::PhaseChanged {
            new_phase: Phase::Main2,
        });
        let game_over = Self::run_main_phase(state, registry, client1, client2, timeout, &mut events);
        Self::notify_clients(state, &events, client1, client2);
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
        Self::notify_clients(state, &events, client1, client2);

        events
    }

    fn run_main_phase(
        state: &mut GameState,
        registry: &CardRegistryImpl,
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
            let available = Self::build_main_actions(state, registry);
            let action = match client.choose_action(&available, timeout) {
                Some(a) => a,
                None => {
                    events.push(CoreGameEvent::GameOver {
                        winner: Some(state.turn_player.opponent()),
                        reason: GameOverReason::Timeout,
                    });
                    Self::notify_clients(state, &*events, client1, client2);
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
                    Self::notify_clients(state, &*events, client1, client2);
                    return true;
                }
                PhaseAction::PlayCard {
                    instance_id,
                    target_zone,
                } => {
                    let mut action_events = Vec::new();
                    let player_idx = player_idx(state.turn_player);

                    // Resolve the card definition to determine cost
                    let cost_info = state.players[player_idx]
                        .zones
                        .hand
                        .iter()
                        .find(|c| c.instance_id == instance_id)
                        .and_then(|c| registry.get(&c.definition_id))
                        .map(|def| def.cost as usize);

                    // Move card from hand to target zone
                    if let Some((card, _)) =
                        state.players[player_idx].zones.remove_card(instance_id)
                    {
                        let player = &mut state.players[player_idx];
                        let to = ZoneLocation::new(state.turn_player, target_zone);
                        let iid = card.instance_id;

                        // Handle Item-replace-Item: if target slot is occupied, move old card to grave
                        let old_card = match target_zone {
                            Zone::Front(slot) => player.zones.front[slot].take(),
                            Zone::Back(slot) => player.zones.back[slot].take(),
                            _ => None,
                        };
                        if let Some(old) = old_card {
                            let old_iid = old.instance_id;
                            player.zones.grave.push(old);
                            action_events.push(CoreGameEvent::CardMoved {
                                instance_id: old_iid,
                                from: to.clone(),
                                to: ZoneLocation::new(state.turn_player, Zone::Grave),
                            });
                        }

                        let _ = player.zones.add_card_to_zone(card, target_zone);
                        action_events.push(CoreGameEvent::CardSummoned { instance_id: iid, to });
                    }

                    // Auto-calculate and pay cost (RealPoint first, then hand cards)
                    if let Some(card_cost) = cost_info {
                        let player = &mut state.players[player_idx];
                        let available_rp = player.real_point as usize;
                        let rp_to_use = card_cost.min(available_rp);
                        let hand_cost = card_cost - rp_to_use;

                        // Deduct RealPoint
                        if rp_to_use > 0 {
                            let old_rp = player.real_point;
                            player.real_point -= rp_to_use as u8;
                            action_events.push(CoreGameEvent::RealPointChanged {
                                player: state.turn_player,
                                old_rp,
                                new_rp: player.real_point,
                            });
                        }

                        // Pay remaining cost from hand (take from front of hand)
                        let cost_ids: Vec<InstanceId> = player.zones.hand
                            .iter()
                            .take(hand_cost)
                            .map(|c| c.instance_id)
                            .collect();

                        for cost_iid in cost_ids {
                            if let Some((cost_card, _)) = player.zones.remove_card(cost_iid) {
                                let cid = cost_card.instance_id;
                                player.zones.cost_zone.push(cost_card);
                                action_events.push(CoreGameEvent::CardExposed { instance_id: cid });
                                action_events.push(CoreGameEvent::CardMoved {
                                    instance_id: cid,
                                    from: ZoneLocation::new(state.turn_player, Zone::Hand),
                                    to: ZoneLocation::new(state.turn_player, Zone::CostZone),
                                });
                            }
                        }
                    }

                    events.append(&mut action_events);
                    Self::notify_clients(state, events, client1, client2);
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
                    Self::notify_clients(state, &*events, client1, client2);
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
                                Self::notify_clients(state, &*events, client1, client2);
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
                    Self::notify_clients(state, &*events, client1, client2);
                }
                PhaseAction::Pass | PhaseAction::Surrender => break,
                _ => break,
            }
        }
        false
    }

    fn build_main_actions(state: &GameState, registry: &CardRegistryImpl) -> Vec<PhaseAction> {
        let mut actions = vec![PhaseAction::Pass, PhaseAction::Surrender];
        let p_idx = player_idx(state.turn_player);
        let player = &state.players[p_idx];

        for card in &player.zones.hand {
            if let Some(def) = registry.get(&card.definition_id) {
                let card_cost = def.cost as usize;
                // The card being played cannot be used as cost itself
                let available_hand = player.zones.hand.len().saturating_sub(1);
                let available_rp = player.real_point as usize;
                let max_payable = available_hand + available_rp;

                // Check if player can afford the cost
                if card_cost > max_payable {
                    continue;
                }

                // Check if cost zone has enough space for hand cards used as cost
                let hand_cost = card_cost.saturating_sub(available_rp);
                if player.zones.cost_zone.len() + hand_cost > state.rules.max_cost_zone {
                    continue;
                }

                // Generate PlayCard actions for each valid target slot based on card type
                match def.card_type {
                    CardType::Character => {
                        // Character cards go to front field
                        for slot in 0..5 {
                            if player.zones.front[slot].is_none() {
                                actions.push(PhaseAction::PlayCard {
                                    instance_id: card.instance_id,
                                    target_zone: Zone::Front(slot),
                                });
                            }
                        }
                    }
                    CardType::Strategy | CardType::Legendary => {
                        // Strategy and Legendary cards go to back field
                        for slot in 0..5 {
                            if player.zones.back[slot].is_none() {
                                actions.push(PhaseAction::PlayCard {
                                    instance_id: card.instance_id,
                                    target_zone: Zone::Back(slot),
                                });
                            }
                        }
                    }
                    CardType::Item => {
                        // Item cards go to back field.
                        // They can be placed in empty slots, or replace an existing Item card.
                        for slot in 0..5 {
                            let can_place = match &player.zones.back[slot] {
                                None => true,
                                Some(existing) => registry
                                    .get(&existing.definition_id)
                                    .map(|d| d.card_type == CardType::Item)
                                    .unwrap_or(false),
                            };
                            if can_place {
                                actions.push(PhaseAction::PlayCard {
                                    instance_id: card.instance_id,
                                    target_zone: Zone::Back(slot),
                                });
                            }
                        }
                    }
                }
            }
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
        CardDefinition, CardId, CardType, Category, InstanceId, PlayerId, Property, Zone,
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

    /// Client that records all on_events calls for verification.
    struct RecordingClient {
        actions: Mutex<VecDeque<PhaseAction>>,
        recovery: Mutex<VecDeque<Vec<InstanceId>>>,
        events_log: Mutex<Vec<Vec<CoreGameEvent>>>,
    }

    impl RecordingClient {
        fn new(actions: Vec<PhaseAction>) -> Self {
            Self {
                actions: Mutex::new(actions.into()),
                recovery: Mutex::new(VecDeque::new()),
                events_log: Mutex::new(Vec::new()),
            }
        }

        fn take_events(&self) -> Vec<Vec<CoreGameEvent>> {
            std::mem::take(&mut *self.events_log.lock().unwrap())
        }
    }

    impl PhaseClient for RecordingClient {
        fn choose_action(
            &self,
            _available: &[PhaseAction],
            _timeout: Duration,
        ) -> Option<PhaseAction> {
            self.actions.lock().unwrap().pop_front()
        }

        fn choose_recovery_cards(
            &self,
            _options: &[RecoveryCardOption],
            _count: usize,
            _timeout: Duration,
        ) -> Option<Vec<InstanceId>> {
            self.recovery.lock().unwrap().pop_front().or_else(|| Some(Vec::new()))
        }

        fn on_events(&self, events: &[CoreGameEvent], _state: &GameState) {
            self.events_log.lock().unwrap().push(events.to_vec());
        }
    }

    #[test]
    fn test_build_main_actions_filters_unaffordable_cards() {
        let mut state = make_state();
        // P1 hand: 3 cards with costs 3, 2, 2 (no RealPoint)
        state.players[0].zones.hand.push(card(1, "S000-C-001", 100)); // cost 3 in registry
        state.players[0].zones.hand.push(card(2, "S000-C-002", 100)); // cost 2 in registry
        state.players[0].zones.hand.push(card(3, "S000-S-001", 100)); // cost 2 in registry

        let mut registry = CardRegistryImpl::new();
        // Register cards with costs: 3, 2, 2
        registry.insert(CardDefinition::new(
            CardId::new("S000-C-001"), "A".into(), CardType::Character,
            Property::Rational, Category::Math, 3,
        ));
        registry.insert(CardDefinition::new(
            CardId::new("S000-C-002"), "B".into(), CardType::Character,
            Property::Rational, Category::Math, 2,
        ));
        registry.insert(CardDefinition::new(
            CardId::new("S000-S-001"), "S".into(), CardType::Strategy,
            Property::Rational, Category::Math, 2,
        ));

        let actions = PhaseRunner::build_main_actions(&state, &registry);
        let play_actions: Vec<_> = actions.iter().filter(|a| matches!(a, PhaseAction::PlayCard { .. })).collect();

        // 3-cost card cannot be played: hand has 3 cards, minus 1 for the card itself = 2 available,
        // plus 0 RP = 2 max payable. 3 > 2, so filtered out.
        // Both 2-cost cards can be played: 2 <= 2.
        // Character card -> 5 front slots; Strategy card -> 5 back slots.
        assert_eq!(play_actions.len(), 10, "Two playable cards × 5 empty slots each");
        let char_actions: Vec<_> = play_actions.iter().filter(|a| matches!(a,
            PhaseAction::PlayCard { instance_id, target_zone: Zone::Front(_) } if *instance_id == InstanceId(2)
        )).collect();
        let strat_actions: Vec<_> = play_actions.iter().filter(|a| matches!(a,
            PhaseAction::PlayCard { instance_id, target_zone: Zone::Back(_) } if *instance_id == InstanceId(3)
        )).collect();
        assert_eq!(char_actions.len(), 5, "Character card should have 5 front slot options");
        assert_eq!(strat_actions.len(), 5, "Strategy card should have 5 back slot options");
    }

    #[test]
    fn test_play_card_auto_cost_payment_broadcasts_to_both_clients() {
        let mut state = make_state();
        state.turn_number = 2; // enable draw
        state.players[0].zones.deck.push(card(99, "S000-C-001", 100));

        // P1 hand: 3 cards. Costs: 3, 2, 2. Play the 2-cost card (instance_id=2).
        state.players[0].zones.hand.push(card(1, "S000-C-001", 100)); // cost 3
        state.players[0].zones.hand.push(card(2, "S000-C-002", 100)); // cost 2
        state.players[0].zones.hand.push(card(3, "S000-S-001", 100)); // cost 2

        let mut registry = CardRegistryImpl::new();
        registry.insert(CardDefinition::new(
            CardId::new("S000-C-001"), "A".into(), CardType::Character,
            Property::Rational, Category::Math, 3,
        ));
        registry.insert(CardDefinition::new(
            CardId::new("S000-C-002"), "B".into(), CardType::Character,
            Property::Rational, Category::Math, 2,
        ));
        registry.insert(CardDefinition::new(
            CardId::new("S000-S-001"), "S".into(), CardType::Strategy,
            Property::Rational, Category::Math, 2,
        ));

        let p1 = RecordingClient::new(vec![
            PhaseAction::PlayCard {
                instance_id: InstanceId(2),
                target_zone: Zone::Front(0),
            },
            PhaseAction::Pass,
            PhaseAction::Pass,
        ]);
        let p2 = RecordingClient::new(vec![]);

        let _events = PhaseRunner::run_turn(&mut state, &registry, &p1, &p2);

        // Verify state changes
        // Hand originally had 3 cards + 1 drawn in Draw phase = 4.
        // Play 1 card (moved to front) + 2 cost cards (moved to cost_zone) = 1 card remaining in hand.
        assert_eq!(state.players[0].zones.hand.len(), 1, "Hand should have 1 card left (the drawn card)");
        assert!(state.players[0].zones.front[0].is_some(), "Card should be on front field");
        assert_eq!(state.players[0].zones.cost_zone.len(), 2, "2 hand cards paid as cost");

        // Verify both clients received on_events with cost-related events
        let p1_log = p1.take_events();
        let p2_log = p2.take_events();

        // Both clients should have received the same number of on_events calls
        assert_eq!(p1_log.len(), p2_log.len(), "Both clients should receive same number of broadcasts");
        assert!(!p1_log.is_empty(), "P1 should have received events");
        assert!(!p2_log.is_empty(), "P2 should have received events");

        // Find the on_events call that contains the PlayCard action events
        // The PlayCard happens in Main1 phase; look for CardSummoned + CardMoved events
        let p1_main1 = p1_log.iter().find(|events| {
            events.iter().any(|e| matches!(e, CoreGameEvent::CardSummoned { instance_id, .. } if *instance_id == InstanceId(2)))
        });
        let p2_main1 = p2_log.iter().find(|events| {
            events.iter().any(|e| matches!(e, CoreGameEvent::CardSummoned { instance_id, .. } if *instance_id == InstanceId(2)))
        });

        assert!(p1_main1.is_some(), "P1 should have received CardSummoned event for instance 2");
        assert!(p2_main1.is_some(), "P2 should have received CardSummoned event for instance 2");

        let p1_events = p1_main1.unwrap();
        let p2_events = p2_main1.unwrap();

        // Verify cost payment events are present
        let summon_count = p1_events.iter().filter(|e| matches!(e, CoreGameEvent::CardSummoned { .. })).count();
        let moved_count = p1_events.iter().filter(|e| matches!(e, CoreGameEvent::CardMoved { .. })).count();
        let exposed_count = p1_events.iter().filter(|e| matches!(e, CoreGameEvent::CardExposed { .. })).count();

        assert_eq!(summon_count, 1, "One card summoned");
        assert_eq!(moved_count, 2, "2 cost cards moved from hand to cost_zone");
        assert_eq!(exposed_count, 2, "2 cost cards exposed");

        // P2 should have identical events (broadcast is the same)
        assert_eq!(p1_events.len(), p2_events.len(), "Both clients should receive identical events");
    }
}
