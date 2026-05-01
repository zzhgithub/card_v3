//! Effect trigger scanning system.
//!
//! Scans all cards on the field for effects that should trigger in response
//! to a game event, returning a list of effects ready to be activated.

use std::collections::HashSet;

use crate::effect::evaluator::evaluate_condition;
use crate::effect::{ActivationLimit, Effect, EventTrigger, Trigger};
use crate::state::{CardInstance, CardRegistryImpl, CoreGameEvent, GameState, Phase};
use crate::types::{EffectKey, InstanceId, PlayerId, PlayerRef};

/// An effect that has been identified as ready to trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggeredEffect {
    /// The card instance that holds this effect.
    pub source_instance: InstanceId,
    /// The effect's key within the card's effects map.
    pub effect_key: EffectKey,
    /// Whether the player can choose not to activate (optional effects).
    pub optional: bool,
    /// The player who controls the source card.
    pub controller: PlayerId,
}

/// Stateless trigger scanner.
pub struct TriggerChecker;

impl TriggerChecker {
    /// Scan all cards on the field for effects triggered by `event`.
    pub fn check_triggers(
        state: &GameState,
        event: &CoreGameEvent,
        perspective: PlayerId,
        registry: &CardRegistryImpl,
    ) -> Vec<TriggeredEffect> {
        let mut results = Vec::new();
        let mut seen: HashSet<(InstanceId, EffectKey)> = HashSet::new();

        for player_id in [PlayerId::Player1, PlayerId::Player2] {
            let player_state = state.player(player_id);

            for card in player_state
                .zones
                .front
                .iter()
                .flatten()
                .chain(player_state.zones.back.iter().flatten())
            {
                for (effect_key, effect) in Self::effects_for_instance(card, registry) {
                    if !Self::trigger_matches(&effect.trigger, event, card.instance_id) {
                        continue;
                    }

                    if !Self::is_activation_allowed(
                        &effect.activation_limit,
                        state,
                        card.instance_id,
                        &effect_key,
                    ) {
                        continue;
                    }

                    if let Some(cond) = &effect.conditions
                        && !evaluate_condition(cond, state, perspective, card)
                    {
                        continue;
                    }

                    if seen.insert((card.instance_id, effect_key.clone())) {
                        results.push(TriggeredEffect {
                            source_instance: card.instance_id,
                            effect_key,
                            optional: effect.optional,
                            controller: player_id,
                        });
                    }
                }
            }
        }

        results
    }

    /// Check for passive triggers that should be inserted into the chain stack.
    /// Called after each chain link resolves.
    pub fn check_triggers_for_chain(
        state: &GameState,
        recent_events: &[CoreGameEvent],
        perspective: PlayerId,
        registry: &CardRegistryImpl,
    ) -> Vec<TriggeredEffect> {
        let mut results = Vec::new();
        for event in recent_events.iter().rev().take(5) {
            let triggered = Self::check_triggers(state, event, perspective, registry);
            for t in triggered {
                if !t.optional {
                    results.push(t);
                }
            }
        }
        results
    }

    /// Check if a specific effect's trigger matches the given event.
    pub fn trigger_matches(trigger: &Trigger, event: &CoreGameEvent, source: InstanceId) -> bool {
        match trigger {
            Trigger::OnSummon => matches!(
                event,
                CoreGameEvent::CardSummoned { instance_id, .. } if *instance_id == source
            ),
            Trigger::OnDestroy => matches!(
                event,
                CoreGameEvent::CardDestroyed { instance_id } if *instance_id == source
            ),
            Trigger::OnAttack => matches!(
                event,
                CoreGameEvent::AttackDeclared { attacker } if *attacker == source
            ),
            Trigger::OnExpose => matches!(
                event,
                CoreGameEvent::CardExposed { instance_id } if *instance_id == source
            ),
            Trigger::TurnStart => matches!(
                event,
                CoreGameEvent::PhaseChanged {
                    new_phase: Phase::TurnStart
                }
            ),
            Trigger::DrawPhase => matches!(
                event,
                CoreGameEvent::PhaseChanged {
                    new_phase: Phase::Draw
                }
            ),
            Trigger::RecoveryPhase => matches!(
                event,
                CoreGameEvent::PhaseChanged {
                    new_phase: Phase::Recovery
                }
            ),
            Trigger::OwnMainPhase | Trigger::OpponentMainPhase | Trigger::BothMainPhase => {
                matches!(
                    event,
                    CoreGameEvent::PhaseChanged {
                        new_phase: Phase::Main1 | Phase::Main2
                    }
                )
            }
            Trigger::BattlePhase => matches!(
                event,
                CoreGameEvent::PhaseChanged {
                    new_phase: Phase::Battle
                }
            ),
            Trigger::TurnEnd => matches!(
                event,
                CoreGameEvent::PhaseChanged {
                    new_phase: Phase::TurnEnd
                }
            ),
            Trigger::OnEvent(et) => event_trigger_matches(et, event),
        }
    }

    /// Check if an effect has exceeded its activation limit this turn.
    pub fn is_activation_allowed(
        limit: &Option<ActivationLimit>,
        state: &GameState,
        source: InstanceId,
        effect_key: &EffectKey,
    ) -> bool {
        match limit {
            None => true,
            Some(ActivationLimit::OncePerTurn) => {
                !state
                    .activated_this_turn
                    .contains(&(source, effect_key.clone()))
            }
            Some(ActivationLimit::OncePerTurnSameName) => {
                let source_def = state.get_card(source).map(|(card, _, _)| card.definition_id.clone());

                if let Some(def_id) = source_def {
                    for player in &state.players {
                        for card in player.zones.all_cards() {
                            if card.definition_id == def_id
                                && state
                                    .activated_this_turn
                                    .contains(&(card.instance_id, effect_key.clone()))
                            {
                                return false;
                            }
                        }
                    }
                }

                true
            }
        }
    }

    fn effects_for_instance(card: &CardInstance, registry: &CardRegistryImpl) -> Vec<(EffectKey, Effect)> {
        let Some(definition) = registry.get(&card.definition_id) else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        for (effect_key, json_value) in &definition.effects {
            if let Ok(effect) = serde_json::from_value::<Effect>(json_value.clone()) {
                effects.push((effect_key.clone(), effect));
            }
        }
        effects
    }
}

fn event_trigger_matches(et: &EventTrigger, event: &CoreGameEvent) -> bool {
    match (et, event) {
        (
            EventTrigger::RealPointChanged { player },
            CoreGameEvent::RealPointChanged {
                player: event_player, ..
            },
        ) => player_ref_matches(player, *event_player),
        (
            EventTrigger::RealPointOverflow { player },
            CoreGameEvent::RealPointOverflow {
                player: event_player,
            },
        ) => player_ref_matches(player, *event_player),
        (
            EventTrigger::HpChanged { player },
            CoreGameEvent::HpChanged {
                player: event_player, ..
            },
        ) => player_ref_matches(player, *event_player),
        (EventTrigger::CardDestroyed { .. }, CoreGameEvent::CardDestroyed { .. }) => true,
        (EventTrigger::CardSummoned { .. }, CoreGameEvent::CardSummoned { .. }) => true,
        (EventTrigger::EffectActivated { .. }, CoreGameEvent::EffectActivated { .. }) => true,
        (EventTrigger::CardExposed { .. }, CoreGameEvent::CardExposed { .. }) => true,
        (
            EventTrigger::CardDrawn { player },
            CoreGameEvent::DrawCard {
                player: event_player, ..
            },
        ) => player_ref_matches(player, *event_player),
        _ => false,
    }
}

fn player_ref_matches(player_ref: &PlayerRef, player_id: PlayerId) -> bool {
    match player_ref {
        PlayerRef::Self_ | PlayerRef::Opponent => {
            let _ = player_id;
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::GameRules;
    use crate::types::{CardId, Zone, ZoneLocation};

    #[test]
    fn on_summon_matches_same_source() {
        let source = InstanceId(1);
        let event = CoreGameEvent::CardSummoned {
            instance_id: source,
            to: ZoneLocation::new(PlayerId::Player1, Zone::Front(0)),
        };
        assert!(TriggerChecker::trigger_matches(&Trigger::OnSummon, &event, source));
    }

    #[test]
    fn on_summon_does_not_match_other_source() {
        let source = InstanceId(1);
        let event = CoreGameEvent::CardSummoned {
            instance_id: InstanceId(2),
            to: ZoneLocation::new(PlayerId::Player1, Zone::Front(0)),
        };
        assert!(!TriggerChecker::trigger_matches(&Trigger::OnSummon, &event, source));
    }

    #[test]
    fn phase_trigger_matches_turn_start() {
        let event = CoreGameEvent::PhaseChanged {
            new_phase: Phase::TurnStart,
        };
        assert!(TriggerChecker::trigger_matches(
            &Trigger::TurnStart,
            &event,
            InstanceId(1),
        ));
    }

    #[test]
    fn once_per_turn_limit_blocks_second_activation() {
        let mut state = GameState::new(GameRules::default(), 42);
        let source = InstanceId(1);
        let effect_key = EffectKey("e1".to_string());

        assert!(TriggerChecker::is_activation_allowed(
            &Some(ActivationLimit::OncePerTurn),
            &state,
            source,
            &effect_key,
        ));

        state
            .activated_this_turn
            .insert((source, effect_key.clone()));

        assert!(!TriggerChecker::is_activation_allowed(
            &Some(ActivationLimit::OncePerTurn),
            &state,
            source,
            &effect_key,
        ));
    }

    #[test]
    fn once_per_turn_same_name_checks_other_instance() {
        let mut state = GameState::new(GameRules::default(), 42);
        let effect_key = EffectKey("e1".to_string());

        let id1 = InstanceId(10);
        let id2 = InstanceId(11);
        let def_id = CardId::new("S001-C-001");

        state.players[0].zones.hand.push(CardInstance::new(id1, def_id.clone(), Some(1000)));
        state.players[1].zones.hand.push(CardInstance::new(id2, def_id, Some(1000)));

        state
            .activated_this_turn
            .insert((id1, effect_key.clone()));

        assert!(!TriggerChecker::is_activation_allowed(
            &Some(ActivationLimit::OncePerTurnSameName),
            &state,
            id2,
            &effect_key,
        ));
    }
}
