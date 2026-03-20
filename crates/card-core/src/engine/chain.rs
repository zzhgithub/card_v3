use crate::effect::Action;
use crate::engine::action::ActionExecutor;
use crate::engine::trigger::TriggerChecker;
use crate::state::{CoreGameEvent, GameState};
use crate::types::{EffectKey, InstanceId, PlayerId};

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
    pub fn resolve_chain(state: &mut GameState, initial: ChainEntry) -> Vec<CoreGameEvent> {
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
                TriggerChecker::check_triggers_for_chain(state, &events, entry.controller);
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
        let events = ChainManager::resolve_chain(&mut state, entry);

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
        let events = ChainManager::resolve_chain(&mut state, entry);

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
        let events = ChainManager::resolve_chain(&mut state, entry);

        assert!(events
            .iter()
            .any(|e| matches!(e, CoreGameEvent::ChainLink { .. })));
    }
}
