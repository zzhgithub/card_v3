use std::time::Instant;

use crate::effect::Effect;
use crate::engine::phase::PhaseClient;
use crate::engine::{ChainEntry, ChainManager, ModifierManager, PhaseRunner, TriggerChecker};
use crate::state::events::GameOverReason;
use crate::state::{CardRegistryImpl, CoreGameEvent, GameState};
use crate::types::PlayerId;

const MAX_TURNS: u32 = 500;

#[derive(Debug, Clone)]
pub struct GameResult {
    pub winner: Option<PlayerId>,
    pub reason: GameOverReason,
    pub final_state: GameState,
    pub event_log: Vec<CoreGameEvent>,
}

pub struct GameEngine {
    state: GameState,
    registry: CardRegistryImpl,
    clients: [Box<dyn PhaseClient>; 2],
}

impl GameEngine {
    pub fn new(
        state: GameState,
        registry: CardRegistryImpl,
        client1: Box<dyn PhaseClient>,
        client2: Box<dyn PhaseClient>,
    ) -> Self {
        Self {
            state,
            registry,
            clients: [client1, client2],
        }
    }

    pub fn run(mut self) -> GameResult {
        let mut event_log = Vec::new();
        let started_at = Instant::now();

        let initial_hand_size = self.state.rules.initial_hand_size;
        for player_idx in 0..2 {
            let player_id = if player_idx == 0 {
                PlayerId::Player1
            } else {
                PlayerId::Player2
            };
            for _ in 0..initial_hand_size {
                if self.state.players[player_idx].zones.deck.is_empty() {
                    let winner = Some(player_id.opponent());
                    return GameResult {
                        winner,
                        reason: GameOverReason::DeckOut,
                        final_state: self.state,
                        event_log,
                    };
                }
                let card = self.state.players[player_idx].zones.deck.remove(0);
                let iid = card.instance_id;
                self.state.players[player_idx].zones.hand.push(card);
                event_log.push(CoreGameEvent::DrawCard {
                    player: player_id,
                    instance_id: iid,
                });
            }
        }

        loop {
            ModifierManager::decrement_turn_counts(&mut self.state);

            let mut turn_events = PhaseRunner::run_turn(
                &mut self.state,
                &self.registry,
                self.clients[0].as_ref(),
                self.clients[1].as_ref(),
            );

            if Self::extract_game_over(&turn_events).is_none() {
                let follow_up_events = self.resolve_follow_up_effects(&turn_events);
                turn_events.extend(follow_up_events);
            }

            let game_over = Self::extract_game_over(&turn_events);

            // Notify clients of state changes after each turn
            self.clients[0].on_events(&turn_events, &self.state);
            self.clients[1].on_events(&turn_events, &self.state);

            event_log.extend(turn_events);

            if let Some((winner, reason)) = game_over {
                return GameResult {
                    winner,
                    reason,
                    final_state: self.state,
                    event_log,
                };
            }

            if self.state.turn_number > MAX_TURNS
                || started_at.elapsed() >= self.state.rules.total_game_timeout
            {
                return GameResult {
                    winner: None,
                    reason: GameOverReason::Timeout,
                    final_state: self.state,
                    event_log,
                };
            }
        }
    }

    fn resolve_follow_up_effects(&mut self, source_events: &[CoreGameEvent]) -> Vec<CoreGameEvent> {
        let mut resolved_events = Vec::new();

        for event in source_events {
            if matches!(event, CoreGameEvent::GameOver { .. }) {
                break;
            }

            let triggered =
                TriggerChecker::check_triggers(&self.state, event, self.state.turn_player);
            if triggered.is_empty() {
                continue;
            }

            let chain_events = self.resolve_triggered_effects(triggered);
            let is_game_over = Self::extract_game_over(&chain_events).is_some();
            resolved_events.extend(chain_events);

            if is_game_over {
                break;
            }
        }

        resolved_events
    }

    fn resolve_triggered_effects(
        &mut self,
        triggered_effects: Vec<crate::engine::TriggeredEffect>,
    ) -> Vec<CoreGameEvent> {
        let mut events = Vec::new();

        for triggered in triggered_effects {
            let Some(entry) = self.build_chain_entry(&triggered) else {
                continue;
            };

            self.state
                .activated_this_turn
                .insert((triggered.source_instance, triggered.effect_key.clone()));
            events.push(CoreGameEvent::EffectActivated {
                instance_id: triggered.source_instance,
                effect_key: triggered.effect_key.clone(),
            });

            let chain_events = ChainManager::resolve_chain(&mut self.state, entry);
            let is_game_over = Self::extract_game_over(&chain_events).is_some();
            events.extend(chain_events);

            if is_game_over {
                break;
            }
        }

        events
    }

    fn build_chain_entry(&self, triggered: &crate::engine::TriggeredEffect) -> Option<ChainEntry> {
        let (card, _, _) = self.state.get_card(triggered.source_instance)?;
        let definition = self.registry.get(&card.definition_id)?;
        let effect_json = definition.effects.get(&triggered.effect_key)?;

        let effect: Effect = serde_json::from_value(effect_json.clone()).ok()?;

        Some(ChainEntry {
            source: triggered.source_instance,
            effect_key: triggered.effect_key.clone(),
            actions: effect.actions,
            controller: triggered.controller,
        })
    }

    fn extract_game_over(events: &[CoreGameEvent]) -> Option<(Option<PlayerId>, GameOverReason)> {
        events.iter().find_map(|event| {
            if let CoreGameEvent::GameOver { winner, reason } = event {
                Some((*winner, reason.clone()))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::phase::{PhaseAction, PhaseClient, RecoveryCardOption};
    use crate::rules::GameRules;
    use crate::state::events::GameOverReason;
    use crate::state::{CardInstance, CardRegistryImpl, CoreGameEvent, GameState};
    use crate::types::{CardDefinition, CardId, CardType, Category, InstanceId, Property};
    use std::time::Duration;

    struct AutoSurrenderClient;

    impl PhaseClient for AutoSurrenderClient {
        fn choose_action(
            &self,
            _available: &[PhaseAction],
            _timeout: Duration,
        ) -> Option<PhaseAction> {
            Some(PhaseAction::Surrender)
        }

        fn choose_recovery_cards(
            &self,
            _options: &[RecoveryCardOption],
            _count: usize,
            _timeout: Duration,
        ) -> Option<Vec<InstanceId>> {
            Some(vec![])
        }
    }

    struct PassAllClient;

    impl PassAllClient {
        fn new() -> Self {
            Self
        }
    }

    impl PhaseClient for PassAllClient {
        fn choose_action(
            &self,
            _available: &[PhaseAction],
            _timeout: Duration,
        ) -> Option<PhaseAction> {
            Some(PhaseAction::Pass)
        }

        fn choose_recovery_cards(
            &self,
            _options: &[RecoveryCardOption],
            _count: usize,
            _timeout: Duration,
        ) -> Option<Vec<InstanceId>> {
            Some(vec![])
        }
    }

    fn minimal_state() -> (GameState, CardRegistryImpl) {
        let rules = GameRules::default();
        let mut state = GameState::new(rules, 42);

        for i in 0..5 {
            state.players[0].zones.deck.push(CardInstance::new(
                InstanceId(i + 100),
                CardId::new("S000-C-001"),
                Some(500),
            ));
            state.players[1].zones.deck.push(CardInstance::new(
                InstanceId(i + 200),
                CardId::new("S000-C-001"),
                Some(500),
            ));
        }

        let mut registry = CardRegistryImpl::new();
        registry.insert(CardDefinition::new(
            CardId::new("S000-C-001"),
            "Test Card".to_string(),
            CardType::Character,
            Property::Rational,
            Category::Math,
            1,
        ));

        (state, registry)
    }

    #[test]
    fn test_game_ends_on_surrender() {
        let (state, registry) = minimal_state();
        let engine = GameEngine::new(
            state,
            registry,
            Box::new(AutoSurrenderClient),
            Box::new(AutoSurrenderClient),
        );

        let result = engine.run();

        assert!(matches!(result.reason, GameOverReason::Surrender));
        assert!(!result.event_log.is_empty());
    }

    #[test]
    fn test_game_ends_on_deck_out() {
        let rules = GameRules::default();
        let mut state = GameState::new(rules, 42);
        state.players[0].zones.deck.push(CardInstance::new(
            InstanceId(100),
            CardId::new("S000-C-001"),
            Some(500),
        ));
        let mut registry = CardRegistryImpl::new();
        registry.insert(CardDefinition::new(
            CardId::new("S000-C-001"),
            "Test".to_string(),
            CardType::Character,
            Property::Rational,
            Category::Math,
            1,
        ));
        let engine = GameEngine::new(
            state,
            registry,
            Box::new(PassAllClient::new()),
            Box::new(PassAllClient::new()),
        );

        let result = engine.run();

        assert!(matches!(
            result.reason,
            GameOverReason::DeckOut | GameOverReason::Surrender | GameOverReason::Timeout
        ));
        assert!(!result.event_log.is_empty());
    }

    #[test]
    fn test_event_log_contains_phase_events() {
        let (state, registry) = minimal_state();
        let engine = GameEngine::new(
            state,
            registry,
            Box::new(AutoSurrenderClient),
            Box::new(AutoSurrenderClient),
        );

        let result = engine.run();

        assert!(result
            .event_log
            .iter()
            .any(|event| matches!(event, CoreGameEvent::PhaseChanged { .. })));
    }

    #[test]
    fn test_game_result_has_final_state() {
        let (state, registry) = minimal_state();
        let engine = GameEngine::new(
            state,
            registry,
            Box::new(AutoSurrenderClient),
            Box::new(AutoSurrenderClient),
        );

        let result = engine.run();

        assert!(result.final_state.turn_number >= 1);
    }

    #[test]
    fn test_initial_hand_drawn_before_first_turn() {
        let rules = GameRules::default();
        let hand_size = rules.initial_hand_size;
        assert!(hand_size > 0, "initial_hand_size must be > 0 for this test");

        let mut state = GameState::new(rules, 42);
        let mut registry = CardRegistryImpl::new();
        registry.insert(CardDefinition::new(
            CardId::new("S000-C-001"),
            "Test".to_string(),
            CardType::Character,
            Property::Rational,
            Category::Math,
            1,
        ));
        for i in 0..(hand_size + 5) {
            state.players[0].zones.deck.push(CardInstance::new(
                InstanceId(i as u32 + 1),
                CardId::new("S000-C-001"),
                Some(100),
            ));
            state.players[1].zones.deck.push(CardInstance::new(
                InstanceId(i as u32 + 100),
                CardId::new("S000-C-001"),
                Some(100),
            ));
        }

        let engine = GameEngine::new(
            state,
            registry,
            Box::new(AutoSurrenderClient),
            Box::new(AutoSurrenderClient),
        );

        let result = engine.run();
        assert!(
            result
                .event_log
                .iter()
                .filter(|e| matches!(e, CoreGameEvent::DrawCard { .. }))
                .count()
                >= hand_size * 2
        );
    }

    #[test]
    fn test_initial_hand_zero_draws_nothing() {
        let mut rules = GameRules::default();
        rules.initial_hand_size = 0;
        let mut state = GameState::new(rules, 42);
        let mut registry = CardRegistryImpl::new();
        registry.insert(CardDefinition::new(
            CardId::new("S000-C-001"),
            "Test".to_string(),
            CardType::Character,
            Property::Rational,
            Category::Math,
            1,
        ));
        for i in 0..5 {
            state.players[0].zones.deck.push(CardInstance::new(
                InstanceId(i + 1),
                CardId::new("S000-C-001"),
                Some(100),
            ));
            state.players[1].zones.deck.push(CardInstance::new(
                InstanceId(i + 100),
                CardId::new("S000-C-001"),
                Some(100),
            ));
        }

        let engine = GameEngine::new(
            state,
            registry,
            Box::new(AutoSurrenderClient),
            Box::new(AutoSurrenderClient),
        );

        let result = engine.run();
        let draw_events_before_first_phase: Vec<_> = result
            .event_log
            .iter()
            .take_while(|e| !matches!(e, CoreGameEvent::PhaseChanged { .. }))
            .filter(|e| matches!(e, CoreGameEvent::DrawCard { .. }))
            .collect();
        assert_eq!(draw_events_before_first_phase.len(), 0);
    }

    #[test]
    fn test_initial_hand_deck_out_returns_deckout() {
        let mut rules = GameRules::default();
        rules.initial_hand_size = 10;
        let mut state = GameState::new(rules, 42);
        let mut registry = CardRegistryImpl::new();
        registry.insert(CardDefinition::new(
            CardId::new("S000-C-001"),
            "Test".to_string(),
            CardType::Character,
            Property::Rational,
            Category::Math,
            1,
        ));
        for i in 0..3 {
            state.players[0].zones.deck.push(CardInstance::new(
                InstanceId(i + 1),
                CardId::new("S000-C-001"),
                Some(100),
            ));
            state.players[1].zones.deck.push(CardInstance::new(
                InstanceId(i + 100),
                CardId::new("S000-C-001"),
                Some(100),
            ));
        }

        let engine = GameEngine::new(
            state,
            registry,
            Box::new(PassAllClient::new()),
            Box::new(PassAllClient::new()),
        );

        let result = engine.run();
        assert!(matches!(result.reason, GameOverReason::DeckOut));
    }
}
