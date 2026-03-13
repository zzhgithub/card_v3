use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerId {
    Player1,
    Player2,
}

impl PlayerId {
    pub fn opponent(self) -> Self {
        match self {
            PlayerId::Player1 => PlayerId::Player2,
            PlayerId::Player2 => PlayerId::Player1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerRef {
    Self_,
    Opponent,
}

impl PlayerRef {
    pub fn resolve(self, current_player: PlayerId) -> PlayerId {
        match self {
            PlayerRef::Self_ => current_player,
            PlayerRef::Opponent => current_player.opponent(),
        }
    }
}
