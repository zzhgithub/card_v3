use serde::{Deserialize, Serialize};

use crate::state::Phase;
use crate::types::{EffectKey, InstanceId, PlayerId, ZoneLocation};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CoreGameEvent {
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
    CardExposed {
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
    AttackDeclared {
        attacker: InstanceId,
    },
    EffectActivated {
        instance_id: InstanceId,
        effect_key: EffectKey,
    },
    GameOver {
        winner: Option<PlayerId>,
        reason: GameOverReason,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameOverReason {
    HpZero,
    DeckOut,
    Surrender,
    Timeout,
}
