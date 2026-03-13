use super::{InstanceId, PlayerId, PlayerRef, Zone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardRef {
    This,
    ByInstanceId(InstanceId),
    BySlot(Zone, usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetRef {
    Player(PlayerRef),
    Card(CardRef),
    Zone(PlayerRef, Zone),
    SlotInZone(PlayerRef, Zone, usize),
}

impl TargetRef {
    pub fn resolve_player(self, current_player: PlayerId) -> Option<PlayerId> {
        match self {
            TargetRef::Player(player_ref) => Some(player_ref.resolve(current_player)),
            _ => None,
        }
    }
}
