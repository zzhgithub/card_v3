//! Action executor: applies game actions to the game state.
//!
//! Each action modifies the GameState and returns a list of CoreGameEvents.
//! Immunity checks are performed before execution; blocked actions return empty.

use crate::effect::{Action, Modifier, ModifierDuration};
use crate::engine::modifier::{ImmunityCheck, ModifierManager};
use crate::state::events::GameOverReason;
use crate::state::{CardInstance, CoreGameEvent, GameState};
use crate::types::{CardId, CardRef, InstanceId, PlayerId, PlayerRef, Zone, ZoneLocation};

/// Stateless action executor.
pub struct ActionExecutor;

impl ActionExecutor {
    /// Execute a single action against the game state.
    ///
    /// Returns events generated. If blocked by immunity, returns empty vec.
    pub fn execute_action(
        action: &Action,
        state: &mut GameState,
        source: InstanceId,
        perspective: PlayerId,
        targets: &[InstanceId],
    ) -> Vec<CoreGameEvent> {
        match action {
            Action::Draw { player, count } => {
                let pid = resolve_player(player, perspective);
                Self::do_draw(state, pid, *count)
            }
            Action::Damage { player, amount } => {
                let pid = resolve_player(player, perspective);
                Self::do_damage(state, pid, *amount)
            }
            Action::Destroy { target } => {
                if let Some(id) = resolve_card_ref(target, source, targets) {
                    Self::do_destroy(state, id)
                } else {
                    vec![]
                }
            }
            Action::SummonFromZone {
                card_id,
                from_zones,
                to_zone,
                no_cost: _,
            } => Self::do_summon_from_zone(state, card_id, from_zones, to_zone, perspective),
            Action::ReturnToHand { target } => {
                if let Some(id) = resolve_card_ref(target, source, targets) {
                    Self::do_return_to_hand(state, id)
                } else {
                    vec![]
                }
            }
            Action::SendToGrave { target } => {
                if let Some(id) = resolve_card_ref(target, source, targets) {
                    Self::do_send_to_grave(state, id)
                } else {
                    vec![]
                }
            }
            Action::ModifyAttack { target, amount } => {
                if let Some(id) = resolve_card_ref(target, source, targets) {
                    Self::do_modify_attack(state, id, *amount)
                } else {
                    vec![]
                }
            }
            Action::GainRealPoint { player, amount } => {
                let pid = resolve_player(player, perspective);
                Self::do_gain_real_point(state, pid, *amount)
            }
            Action::HealHp { player, amount } => {
                let pid = resolve_player(player, perspective);
                Self::do_heal_hp(state, pid, *amount)
            }
            Action::Discard { player, count } => {
                let pid = resolve_player(player, perspective);
                Self::do_discard(state, pid, *count)
            }
            Action::ApplyModifier {
                target,
                modifier,
                duration,
            } => {
                if let Some(id) = resolve_card_ref(target, source, targets) {
                    Self::do_apply_modifier(state, id, modifier.clone(), source, duration.clone())
                } else {
                    vec![]
                }
            }
            Action::RemoveModifier {
                target,
                modifier_key,
            } => {
                if let Some(id) = resolve_card_ref(target, source, targets) {
                    Self::do_remove_modifier(state, id, modifier_key)
                } else {
                    vec![]
                }
            }
            Action::Special { .. } => {
                // Special effects handled by GameEngine via registered handlers
                vec![]
            }
        }
    }

    fn do_draw(state: &mut GameState, player_id: PlayerId, count: u8) -> Vec<CoreGameEvent> {
        let mut events = Vec::new();
        let idx = player_idx(player_id);
        for _ in 0..count {
            if state.players[idx].zones.deck.is_empty() {
                events.push(CoreGameEvent::GameOver {
                    winner: Some(player_id.opponent()),
                    reason: GameOverReason::DeckOut,
                });
                return events;
            }
            let card = state.players[idx].zones.deck.remove(0);
            let iid = card.instance_id;
            if state.players[idx].zones.hand.len() < state.rules.max_hand {
                state.players[idx].zones.hand.push(card);
            } else {
                state.players[idx].zones.grave.push(card);
            }
            events.push(CoreGameEvent::DrawCard {
                player: player_id,
                instance_id: iid,
            });
        }
        events
    }

    fn do_damage(state: &mut GameState, player_id: PlayerId, amount: u8) -> Vec<CoreGameEvent> {
        let idx = player_idx(player_id);
        let old_hp = state.players[idx].hp;
        let new_hp = old_hp.saturating_sub(amount);
        state.players[idx].hp = new_hp;
        let mut events = vec![CoreGameEvent::HpChanged {
            player: player_id,
            old_hp,
            new_hp,
        }];
        if new_hp == 0 {
            events.push(CoreGameEvent::GameOver {
                winner: Some(player_id.opponent()),
                reason: GameOverReason::HpZero,
            });
        }
        events
    }

    fn do_destroy(state: &mut GameState, target_id: InstanceId) -> Vec<CoreGameEvent> {
        // Immunity check
        if let Some((card, _, _)) = state.get_card(target_id)
            && ModifierManager::has_immunity(card, &ImmunityCheck::Destruction)
        {
            return vec![];
        }
        for idx in 0..2 {
            if let Some((card, _)) = state.players[idx].zones.remove_card(target_id) {
                state.players[idx].zones.grave.push(card);
                return vec![CoreGameEvent::CardDestroyed {
                    instance_id: target_id,
                }];
            }
        }
        vec![]
    }

    fn do_summon_from_zone(
        state: &mut GameState,
        card_id: &CardId,
        from_zones: &[Zone],
        to_zone: &Zone,
        perspective: PlayerId,
    ) -> Vec<CoreGameEvent> {
        let idx = player_idx(perspective);
        let mut found_iid: Option<InstanceId> = None;
        'outer: for zone in from_zones {
            let cards: &Vec<CardInstance> = match zone {
                Zone::Deck => &state.players[idx].zones.deck,
                Zone::Hand => &state.players[idx].zones.hand,
                Zone::CostZone => &state.players[idx].zones.cost_zone,
                Zone::Grave => &state.players[idx].zones.grave,
                _ => continue,
            };
            for c in cards {
                if &c.definition_id == card_id {
                    found_iid = Some(c.instance_id);
                    break 'outer;
                }
            }
        }
        if let Some(iid) = found_iid
            && let Some((card, _)) = state.players[idx].zones.remove_card(iid)
        {
            let _ = state.players[idx]
                .zones
                .add_card_to_zone(card, *to_zone);
            let to = ZoneLocation::new(perspective, *to_zone);
            return vec![CoreGameEvent::CardSummoned {
                instance_id: iid,
                to,
            }];
        }
        vec![]
    }

    fn do_return_to_hand(state: &mut GameState, target_id: InstanceId) -> Vec<CoreGameEvent> {
        for idx in 0..2 {
            if let Some((card, _)) = state.players[idx].zones.remove_card(target_id) {
                let pid = if idx == 0 {
                    PlayerId::Player1
                } else {
                    PlayerId::Player2
                };
                let to = ZoneLocation::new(pid, Zone::Hand);
                if state.players[idx].zones.hand.len() < state.rules.max_hand {
                    state.players[idx].zones.hand.push(card);
                } else {
                    state.players[idx].zones.grave.push(card);
                }
                return vec![CoreGameEvent::CardMoved {
                    instance_id: target_id,
                    from: ZoneLocation::new(pid, Zone::Grave), // approximate
                    to,
                }];
            }
        }
        vec![]
    }

    fn do_send_to_grave(state: &mut GameState, target_id: InstanceId) -> Vec<CoreGameEvent> {
        for idx in 0..2 {
            if let Some((card, from_zone)) = state.players[idx].zones.remove_card(target_id) {
                let pid = if idx == 0 {
                    PlayerId::Player1
                } else {
                    PlayerId::Player2
                };
                let from = ZoneLocation::new(pid, from_zone);
                let to = ZoneLocation::new(pid, Zone::Grave);
                state.players[idx].zones.grave.push(card);
                return vec![CoreGameEvent::CardMoved {
                    instance_id: target_id,
                    from,
                    to,
                }];
            }
        }
        vec![]
    }

    fn do_modify_attack(
        state: &mut GameState,
        target_id: InstanceId,
        amount: i16,
    ) -> Vec<CoreGameEvent> {
        for idx in 0..2 {
            for slot in 0..5 {
                if let Some(card) = state.players[idx].zones.front[slot].as_mut()
                    && card.instance_id == target_id
                {
                    let old = card.current_attack.unwrap_or(0);
                    let new = (old + amount as i32).max(0);
                    card.current_attack = Some(new);
                    return vec![CoreGameEvent::AttackModified {
                        instance_id: target_id,
                        old_attack: old,
                        new_attack: new,
                    }];
                }
                if let Some(card) = state.players[idx].zones.back[slot].as_mut()
                    && card.instance_id == target_id
                {
                    let old = card.current_attack.unwrap_or(0);
                    let new = (old + amount as i32).max(0);
                    card.current_attack = Some(new);
                    return vec![CoreGameEvent::AttackModified {
                        instance_id: target_id,
                        old_attack: old,
                        new_attack: new,
                    }];
                }
            }
        }
        vec![]
    }

    fn do_gain_real_point(
        state: &mut GameState,
        player_id: PlayerId,
        amount: u8,
    ) -> Vec<CoreGameEvent> {
        let idx = player_idx(player_id);
        let old_rp = state.players[idx].real_point;
        let max_rp = state.rules.max_real_point;
        let new_rp = (old_rp + amount).min(max_rp);
        state.players[idx].real_point = new_rp;
        let mut events = vec![CoreGameEvent::RealPointChanged {
            player: player_id,
            old_rp,
            new_rp,
        }];
        if new_rp >= max_rp && (old_rp < max_rp || amount > 0) {
            events.push(CoreGameEvent::RealPointOverflow { player: player_id });
        }
        events
    }

    fn do_heal_hp(state: &mut GameState, player_id: PlayerId, amount: u8) -> Vec<CoreGameEvent> {
        let idx = player_idx(player_id);
        let old_hp = state.players[idx].hp;
        let new_hp = (old_hp + amount).min(state.rules.max_hp);
        state.players[idx].hp = new_hp;
        vec![CoreGameEvent::HpChanged {
            player: player_id,
            old_hp,
            new_hp,
        }]
    }

    fn do_discard(state: &mut GameState, player_id: PlayerId, count: u8) -> Vec<CoreGameEvent> {
        let idx = player_idx(player_id);
        let mut events = Vec::new();
        for _ in 0..count {
            if let Some(card) = state.players[idx].zones.hand.pop() {
                let iid = card.instance_id;
                let from = ZoneLocation::new(player_id, Zone::Hand);
                let to = ZoneLocation::new(player_id, Zone::Grave);
                state.players[idx].zones.grave.push(card);
                events.push(CoreGameEvent::CardMoved {
                    instance_id: iid,
                    from,
                    to,
                });
            }
        }
        events
    }

    fn do_apply_modifier(
        state: &mut GameState,
        target_id: InstanceId,
        modifier: Modifier,
        source: InstanceId,
        duration: ModifierDuration,
    ) -> Vec<CoreGameEvent> {
        for idx in 0..2 {
            for slot in 0..5 {
                if let Some(card) = state.players[idx].zones.front[slot].as_mut()
                    && card.instance_id == target_id
                {
                    ModifierManager::apply_modifier(card, modifier, source, duration);
                    return vec![];
                }
                if let Some(card) = state.players[idx].zones.back[slot].as_mut()
                    && card.instance_id == target_id
                {
                    ModifierManager::apply_modifier(card, modifier, source, duration);
                    return vec![];
                }
            }
        }
        vec![]
    }

    fn do_remove_modifier(
        state: &mut GameState,
        target_id: InstanceId,
        modifier_key: &str,
    ) -> Vec<CoreGameEvent> {
        for idx in 0..2 {
            for slot in 0..5 {
                if let Some(card) = state.players[idx].zones.front[slot].as_mut()
                    && card.instance_id == target_id
                {
                    if let Some(i) = card.modifiers.iter().position(|m| {
                        matches!(&m.modifier, Modifier::Special { key } if key == modifier_key)
                    }) {
                        ModifierManager::remove_modifier(card, i);
                    }
                    return vec![];
                }
            }
        }
        vec![]
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn resolve_player(player_ref: &PlayerRef, perspective: PlayerId) -> PlayerId {
    match player_ref {
        PlayerRef::Self_ => perspective,
        PlayerRef::Opponent => perspective.opponent(),
    }
}

fn resolve_card_ref(
    card_ref: &CardRef,
    source: InstanceId,
    _targets: &[InstanceId],
) -> Option<InstanceId> {
    match card_ref {
        CardRef::This => Some(source),
        CardRef::ByInstanceId(id) => Some(*id),
        CardRef::BySlot(_, _) => None, // runtime slot resolution not supported here
    }
}

fn player_idx(player_id: PlayerId) -> usize {
    match player_id {
        PlayerId::Player1 => 0,
        PlayerId::Player2 => 1,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{Action, Modifier, ModifierDuration};
    use crate::rules::GameRules;
    use crate::state::{CardInstance, GameState};
    use crate::types::{CardId, InstanceId, PlayerId, PlayerRef};

    fn make_state() -> GameState {
        GameState::new(GameRules::default(), 42)
    }

    fn test_card(id: u32, attack: i32) -> CardInstance {
        CardInstance::new(InstanceId(id), CardId::new("S000-C-001"), Some(attack))
    }

    #[test]
    fn draw_moves_card_to_hand() {
        let mut state = make_state();
        state.players[0].zones.deck.push(test_card(1, 500));
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
        assert_eq!(state.players[0].zones.hand.len(), 1);
        assert_eq!(state.players[0].zones.deck.len(), 0);
        assert!(matches!(events[0], CoreGameEvent::DrawCard { .. }));
    }

    #[test]
    fn draw_empty_deck_triggers_game_over() {
        let mut state = make_state();
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
        assert!(events.iter().any(|e| matches!(
            e,
            CoreGameEvent::GameOver {
                reason: GameOverReason::DeckOut,
                ..
            }
        )));
    }

    #[test]
    fn damage_reduces_hp() {
        let mut state = make_state();
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
        assert!(matches!(
            events[0],
            CoreGameEvent::HpChanged { new_hp: 3, .. }
        ));
    }

    #[test]
    fn damage_to_zero_triggers_game_over() {
        let mut state = make_state();
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
        assert!(events.iter().any(|e| matches!(
            e,
            CoreGameEvent::GameOver {
                reason: GameOverReason::HpZero,
                ..
            }
        )));
    }

    #[test]
    fn destroy_sends_card_to_grave() {
        let mut state = make_state();
        state.players[0].zones.front[0] = Some(test_card(1, 500));
        let events = ActionExecutor::execute_action(
            &Action::Destroy {
                target: CardRef::ByInstanceId(InstanceId(1)),
            },
            &mut state,
            InstanceId(0),
            PlayerId::Player1,
            &[],
        );
        assert!(state.players[0].zones.front[0].is_none());
        assert_eq!(state.players[0].zones.grave.len(), 1);
        assert!(matches!(events[0], CoreGameEvent::CardDestroyed { .. }));
    }

    #[test]
    fn destroy_immune_card_is_skipped() {
        let mut state = make_state();
        let mut card = test_card(1, 500);
        ModifierManager::apply_modifier(
            &mut card,
            Modifier::ImmuneToDestruction,
            InstanceId(99),
            ModifierDuration::Permanent,
        );
        state.players[0].zones.front[0] = Some(card);
        let events = ActionExecutor::execute_action(
            &Action::Destroy {
                target: CardRef::ByInstanceId(InstanceId(1)),
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
    fn gain_real_point_capped_at_max() {
        let mut state = make_state();
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
        assert!(events
            .iter()
            .any(|e| matches!(e, CoreGameEvent::RealPointOverflow { .. })));
    }

    #[test]
    fn heal_hp_capped_at_max() {
        let mut state = make_state();
        state.players[0].hp = 4;
        let events = ActionExecutor::execute_action(
            &Action::HealHp {
                player: PlayerRef::Self_,
                amount: 5,
            },
            &mut state,
            InstanceId(0),
            PlayerId::Player1,
            &[],
        );
        assert_eq!(state.players[0].hp, 6);
        assert!(matches!(
            events[0],
            CoreGameEvent::HpChanged { new_hp: 6, .. }
        ));
    }

    #[test]
    fn discard_moves_hand_to_grave() {
        let mut state = make_state();
        state.players[0].zones.hand.push(test_card(1, 0));
        state.players[0].zones.hand.push(test_card(2, 0));
        let events = ActionExecutor::execute_action(
            &Action::Discard {
                player: PlayerRef::Self_,
                count: 1,
            },
            &mut state,
            InstanceId(0),
            PlayerId::Player1,
            &[],
        );
        assert_eq!(state.players[0].zones.hand.len(), 1);
        assert_eq!(state.players[0].zones.grave.len(), 1);
        assert!(matches!(events[0], CoreGameEvent::CardMoved { .. }));
    }
}
