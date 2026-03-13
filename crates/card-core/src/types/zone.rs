use super::PlayerId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Zone {
    Deck,
    Hand,
    Front(usize),
    Back(usize),
    CostZone,
    Grave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ZoneLocation {
    pub player: PlayerId,
    pub zone: Zone,
}

impl ZoneLocation {
    pub fn new(player: PlayerId, zone: Zone) -> Self {
        Self { player, zone }
    }
}
