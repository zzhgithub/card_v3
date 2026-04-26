//! Network protocol message types for the card game.
//!
//! Defines core message types used by the client API and engine:
//! - [`Command`]: Player action commands (client → host)
//! - [`GameEvent`]: Game state change events (host → client)
//!
//! Note: The legacy TCP transport layer (`NetworkMessage` + `TcpConnection`) has been
//! removed. The active server uses WebSocket + JSON (see `card-server::websocket_server`).

use card_core::types::{CardId, EffectKey, InstanceId, PlayerId, TargetRef, Zone, ZoneLocation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostPayment {
    pub hand_cards: Vec<InstanceId>,
    pub real_point: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttackTarget {
    FrontSlot(usize),
    DirectAttack,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Phase {
    TurnStart,
    Draw,
    Recovery,
    Main1,
    Battle,
    Main2,
    TurnEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameOverReason {
    HpZero,
    DeckOut,
    Surrender,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    PlayCard {
        instance_id: InstanceId,
        target_zone: Zone,
    },
    ActivateEffect {
        instance_id: InstanceId,
        effect_key: EffectKey,
        cost_payment: CostPayment,
    },
    DeclareAttack {
        attacker: InstanceId,
        target: AttackTarget,
    },
    DirectAttackRealPoint {
        real_point_used: u8,
    },
    ChainActivate {
        instance_id: InstanceId,
        effect_key: EffectKey,
        cost_payment: CostPayment,
    },
    ChainPass,
    SelectRecoveryCards {
        instance_ids: Vec<InstanceId>,
    },
    SelectTargets {
        targets: Vec<TargetRef>,
    },
    SelectCards {
        instance_ids: Vec<InstanceId>,
    },
    Surrender,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AvailableAction {
    PlayCard {
        instance_id: InstanceId,
    },
    ActivateEffect {
        instance_id: InstanceId,
        effect_key: EffectKey,
    },
    DeclareAttack {
        attacker: InstanceId,
        possible_targets: Vec<AttackTarget>,
    },
    ChainActivate {
        instance_id: InstanceId,
        effect_key: EffectKey,
    },
    ChainPass,
    SelectRecoveryCards {
        required_count: usize,
        candidates: Vec<InstanceId>,
    },
    Surrender,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    CardMoved {
        instance_id: InstanceId,
        from: ZoneLocation,
        to: ZoneLocation,
    },
    CardSummoned {
        instance_id: InstanceId,
        to: ZoneLocation,
    },
    CardDestroyed {
        instance_id: InstanceId,
    },
    HpChanged {
        player: PlayerId,
        old_hp: u8,
        new_hp: u8,
    },
    RealPointChanged {
        player: PlayerId,
        old_rp: u8,
        new_rp: u8,
    },
    RealPointOverflow {
        player: PlayerId,
    },
    AttackModified {
        instance_id: InstanceId,
        old_attack: i32,
        new_attack: i32,
    },
    PhaseChanged {
        new_phase: Phase,
    },
    TurnChanged {
        new_active_player: PlayerId,
        turn_number: u32,
    },
    DrawCard {
        player: PlayerId,
        instance_id: InstanceId,
    },
    EffectActivated {
        instance_id: InstanceId,
        effect_key: EffectKey,
    },
    ChainStarted,
    ChainLink {
        instance_id: InstanceId,
        effect_key: EffectKey,
    },
    ChainResolving {
        link_index: usize,
    },
    ChainComplete,
    RequestAction {
        player: PlayerId,
        available_actions: Vec<AvailableAction>,
        timeout_secs: u64,
    },
    RequestTargetSelection {
        player: PlayerId,
        prompt: String,
        candidates: Vec<TargetRef>,
        count: usize,
        timeout_secs: u64,
    },
    RequestCardSelection {
        player: PlayerId,
        prompt: String,
        candidates: Vec<InstanceId>,
        count: usize,
        timeout_secs: u64,
    },
    GameOver {
        winner: Option<PlayerId>,
        reason: GameOverReason,
    },
}


#[cfg(test)]
mod tests {
    use super::*;
    use card_core::types::{InstanceId, PlayerId};

    #[test]
    fn command_surrender_bincode_roundtrip() {
        let cmd = Command::Surrender;
        let bytes = bincode::serialize(&cmd).unwrap();
        let decoded: Command = bincode::deserialize(&bytes).unwrap();
        assert!(matches!(decoded, Command::Surrender));
    }

    #[test]
    fn game_event_game_over_serde_json() {
        let event = GameEvent::GameOver {
            winner: Some(PlayerId::Player1),
            reason: GameOverReason::HpZero,
        };
        let json = serde_json::to_string_pretty(&event).unwrap();
        assert!(json.contains("GameOver"));
        assert!(json.contains("HpZero"));
        let decoded: GameEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            decoded,
            GameEvent::GameOver {
                reason: GameOverReason::HpZero,
                ..
            }
        ));
    }

    #[test]
    fn command_declare_attack_roundtrip() {
        let cmd = Command::DeclareAttack {
            attacker: InstanceId(1),
            target: AttackTarget::FrontSlot(0),
        };
        let bytes = bincode::serialize(&cmd).unwrap();
        let decoded: Command = bincode::deserialize(&bytes).unwrap();
        assert!(matches!(decoded, Command::DeclareAttack { .. }));
    }
}
