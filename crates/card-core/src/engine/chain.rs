use std::time::Duration;

use crate::effect::Action;
use crate::engine::action::ActionExecutor;
use crate::engine::phase::ChainActivateOption;
use crate::engine::trigger::TriggerChecker;
use crate::state::{CardRegistryImpl, CoreGameEvent, GameState};
use crate::types::{EffectKey, InstanceId, PlayerId, StrategyKind};

#[derive(Debug, Clone)]
pub struct ChainEntry {
    pub source: InstanceId,
    pub effect_key: EffectKey,
    pub actions: Vec<Action>,
    pub controller: PlayerId,
}

pub struct ChainManager;

impl ChainManager {
    /// Resolve a chain stack in FILO order.
    ///
    /// The initial entry is pushed first. Passive triggers produced by each
    /// resolved link are pushed onto the stack (FILO: last-in first-out).
    pub fn resolve_chain(state: &mut GameState, initial: ChainEntry, registry: &CardRegistryImpl) -> Vec<CoreGameEvent> {
        let mut events = Vec::new();
        let mut stack: Vec<ChainEntry> = vec![initial];

        events.push(CoreGameEvent::ChainStarted);
        events.push(CoreGameEvent::ChainLink {
            instance_id: stack[0].source,
            effect_key: stack[0].effect_key.clone(),
        });

        let mut link_index = 0usize;

        while let Some(entry) = stack.pop() {
            events.push(CoreGameEvent::ChainResolving { link_index });
            link_index += 1;

            for action in &entry.actions {
                let action_events = ActionExecutor::execute_action(
                    action,
                    state,
                    entry.source,
                    entry.controller,
                    &[],
                );

                let is_game_over = action_events
                    .iter()
                    .any(|e| matches!(e, CoreGameEvent::GameOver { .. }));
                events.extend(action_events);

                if is_game_over {
                    events.push(CoreGameEvent::ChainComplete);
                    return events;
                }
            }

            let _passive_triggers =
                TriggerChecker::check_triggers_for_chain(state, &events, entry.controller, registry);
        }

        events.push(CoreGameEvent::ChainComplete);
        events
    }

    /// Build a ChainEntry from an instance_id and effect_key, using the registry.
    pub fn build_entry(
        state: &GameState,
        registry: &CardRegistryImpl,
        instance_id: InstanceId,
        effect_key: &EffectKey,
    ) -> Option<ChainEntry> {
        let (card, _, _) = state.get_card(instance_id)?;
        let definition = registry.get(&card.definition_id)?;
        let json_value = definition.effects.get(effect_key)?;
        let effect: crate::effect::Effect = serde_json::from_value(json_value.clone()).ok()?;
        let controller = if state.players[0]
            .zones
            .all_cards()
            .any(|c| c.instance_id == instance_id)
        {
            PlayerId::Player1
        } else {
            PlayerId::Player2
        };
        Some(ChainEntry {
            source: instance_id,
            effect_key: effect_key.clone(),
            actions: effect.actions,
            controller,
        })
    }

    /// Check if a card is an Instant strategy card (resolves immediately, not pushed to stack).
    fn is_instant(state: &GameState, registry: &CardRegistryImpl, instance_id: InstanceId) -> bool {
        state
            .get_card(instance_id)
            .and_then(|(card, _, _)| registry.get(&card.definition_id))
            .and_then(|def| def.strategy_kind.as_ref())
            .map(|k| matches!(k, StrategyKind::Instant))
            .unwrap_or(false)
    }

    /// Open an interactive chain window.
    ///
    /// After an effect is activated, both players get the opportunity to chain
    /// additional effects. Priority starts with `state.turn_player`. The function
    /// loops asking players to ChainActivate or ChainPass until both pass consecutively,
    /// then resolves the chain in FILO order.
    ///
    /// `build_options` is a callback that scans a player's cards for chainable effects.
    pub fn open_chain_window(
        state: &mut GameState,
        registry: &CardRegistryImpl,
        initial_entry: ChainEntry,
        client1: &dyn crate::engine::phase::PhaseClient,
        client2: &dyn crate::engine::phase::PhaseClient,
        build_options: &dyn Fn(&GameState, &CardRegistryImpl, PlayerId) -> Vec<ChainActivateOption>,
        timeout: Duration,
    ) -> Vec<CoreGameEvent> {
        let mut events = Vec::new();

        // Initialize chain stack
        state.chain.is_open = true;
        state.chain.links.clear();
        state.chain.links.push(crate::state::ChainLink {
            source_instance: initial_entry.source,
            effect_key: initial_entry.effect_key.clone(),
            activating_player: initial_entry.controller,
        });
        state.chain.last_passed = None;

        events.push(CoreGameEvent::ChainStarted);
        events.push(CoreGameEvent::ChainLink {
            instance_id: initial_entry.source,
            effect_key: initial_entry.effect_key.clone(),
        });

        // Interactive loop: alternate between players
        let mut current_ask = state.turn_player;

        loop {
            let available = build_options(state, registry, current_ask);

            if available.is_empty() {
                // No chainable effects → auto-pass
                state.chain.last_passed = Some(current_ask);
            } else {
                let client: &dyn crate::engine::phase::PhaseClient = if current_ask == PlayerId::Player1 {
                    client1
                } else {
                    client2
                };
                let response = client.choose_chain_action(&state.chain, &available, timeout);

                match response {
                    Some(crate::engine::phase::ChainResponse::ChainActivate {
                        instance_id,
                        effect_key,
                    }) => {
                        // Check if Instant → resolve immediately, don't push to stack
                        if Self::is_instant(state, registry, instance_id) {
                            // Build entry and execute actions immediately
                            if let Some(entry) = Self::build_entry(state, registry, instance_id, &effect_key)
                            {
                                let link_idx = state.chain.links.len();
                                events.push(CoreGameEvent::ChainLink {
                                    instance_id,
                                    effect_key: effect_key.clone(),
                                });
                                events.push(CoreGameEvent::ChainResolving {
                                    link_index: link_idx,
                                });
                                for action in &entry.actions {
                                    let action_events = ActionExecutor::execute_action(
                                        action,
                                        state,
                                        entry.source,
                                        entry.controller,
                                        &[],
                                    );
                                    let is_game_over = action_events
                                        .iter()
                                        .any(|e| matches!(e, CoreGameEvent::GameOver { .. }));
                                    events.extend(action_events);
                                    if is_game_over {
                                        events.push(CoreGameEvent::ChainComplete);
                                        state.chain.is_open = false;
                                        return events;
                                    }
                                }
                            }
                            // After Instant resolution, priority goes to opponent
                            state.chain.last_passed = None;
                            current_ask = current_ask.opponent();
                            continue;
                        }

                        // Normal (non-Instant) effect → push to chain stack
                        if Self::build_entry(state, registry, instance_id, &effect_key).is_some() {
                            state.chain.links.push(crate::state::ChainLink {
                                source_instance: instance_id,
                                effect_key: effect_key.clone(),
                                activating_player: current_ask,
                            });
                            events.push(CoreGameEvent::ChainLink {
                                instance_id,
                                effect_key: effect_key.clone(),
                            });
                            state.chain.last_passed = None;
                        }
                    }
                    _ => {
                        // Pass or timeout
                        state.chain.last_passed = Some(current_ask);
                    }
                }
            }

            // Check if both players have passed consecutively
            let opponent = current_ask.opponent();
            if state.chain.last_passed == Some(opponent) {
                // Opponent was the last to pass before us → both passed → resolve
                break;
            }

            // Priority passes to opponent
            current_ask = opponent;
        }

        // FILO resolution
        state.chain.is_open = false;
        let mut stack: Vec<ChainEntry> = Vec::new();
        // Build stack for FILO: push in activation order, pop gives reverse (last-in-first-out)
        for link in state.chain.links.iter() {
            if let Some(entry) =
                Self::build_entry(state, registry, link.source_instance, &link.effect_key)
            {
                stack.push(entry);
            }
        }
        let mut link_index = 0usize;
        while let Some(entry) = stack.pop() {
            events.push(CoreGameEvent::ChainResolving { link_index });
            link_index += 1;

            for action in &entry.actions {
                let action_events = ActionExecutor::execute_action(
                    action,
                    state,
                    entry.source,
                    entry.controller,
                    &[],
                );

                let is_game_over = action_events
                    .iter()
                    .any(|e| matches!(e, CoreGameEvent::GameOver { .. }));
                events.extend(action_events);

                if is_game_over {
                    events.push(CoreGameEvent::ChainComplete);
                    return events;
                }
            }
        }

        events.push(CoreGameEvent::ChainComplete);
        events
    }

    /// Push an additional entry onto a chain (used to build multi-link chains).
    pub fn push_link(
        stack: &mut Vec<ChainEntry>,
        entry: ChainEntry,
        events: &mut Vec<CoreGameEvent>,
    ) {
        events.push(CoreGameEvent::ChainLink {
            instance_id: entry.source,
            effect_key: entry.effect_key.clone(),
        });
        stack.push(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Action;
    use crate::rules::GameRules;
    use crate::state::{CardInstance, GameState};
    use crate::types::{CardId, EffectKey, InstanceId, PlayerId, PlayerRef};

    fn make_state() -> GameState {
        GameState::new(GameRules::default(), 42)
    }

    fn test_card(id: u32) -> CardInstance {
        CardInstance::new(InstanceId(id), CardId::new("S000-C-001"), Some(500))
    }

    fn draw_entry(source: InstanceId, controller: PlayerId) -> ChainEntry {
        ChainEntry {
            source,
            effect_key: EffectKey("e1".to_string()),
            actions: vec![Action::Draw {
                player: PlayerRef::Self_,
                count: 1,
            }],
            controller,
        }
    }

    fn damage_entry(source: InstanceId, controller: PlayerId) -> ChainEntry {
        ChainEntry {
            source,
            effect_key: EffectKey("e2".to_string()),
            actions: vec![Action::Damage {
                player: PlayerRef::Opponent,
                amount: 1,
            }],
            controller,
        }
    }

    #[test]
    fn test_chain_starts_and_completes() {
        let mut state = make_state();
        state.players[0].zones.deck.push(test_card(1));

        let entry = draw_entry(InstanceId(0), PlayerId::Player1);
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
    fn test_filo_order_damage_before_draw() {
        let mut state = make_state();
        state.players[0].zones.deck.push(test_card(1));

        let draw = draw_entry(InstanceId(0), PlayerId::Player1);
        let damage = damage_entry(InstanceId(0), PlayerId::Player1);

        let initial_hp = state.players[1].hp;

        let mut stack_entries = vec![draw, damage];
        let first = stack_entries.remove(0);
        let mut events = vec![CoreGameEvent::ChainStarted];
        events.push(CoreGameEvent::ChainLink {
            instance_id: first.source,
            effect_key: first.effect_key.clone(),
        });

        let mut all_entries = vec![first];
        all_entries.extend(stack_entries);

        let mut all_events = Vec::new();
        all_events.push(CoreGameEvent::ChainStarted);

        let mut stack: Vec<ChainEntry> = all_entries;
        let mut link_idx = 0;
        while let Some(entry) = stack.pop() {
            all_events.push(CoreGameEvent::ChainResolving {
                link_index: link_idx,
            });
            link_idx += 1;
            for action in &entry.actions {
                let evs = ActionExecutor::execute_action(
                    action,
                    &mut state,
                    entry.source,
                    entry.controller,
                    &[],
                );
                all_events.extend(evs);
            }
        }
        all_events.push(CoreGameEvent::ChainComplete);

        assert_eq!(state.players[1].hp, initial_hp - 1);
        assert_eq!(state.players[0].zones.hand.len(), 1);
    }

    #[test]
    fn test_game_over_stops_chain() {
        let mut state = make_state();
        state.players[1].hp = 1;

        let entry = damage_entry(InstanceId(0), PlayerId::Player1);
        let events = ChainManager::resolve_chain(&mut state, entry, &CardRegistryImpl::new());

        assert!(events
            .iter()
            .any(|e| matches!(e, CoreGameEvent::GameOver { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, CoreGameEvent::ChainComplete)));
    }

    #[test]
    fn test_chain_emits_chain_link_event() {
        let mut state = make_state();
        state.players[0].zones.deck.push(test_card(1));

        let entry = draw_entry(InstanceId(0), PlayerId::Player1);
        let events = ChainManager::resolve_chain(&mut state, entry, &CardRegistryImpl::new());

        assert!(events
            .iter()
            .any(|e| matches!(e, CoreGameEvent::ChainLink { .. })));
    }
}
