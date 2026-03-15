// card-core: game engine core

pub mod effect;
pub mod error;
pub mod rules;
pub mod state;
pub mod types;

pub use error::CoreError;
pub use rules::{validate_deck, CardRegistry, GameRules};
pub use state::{
    CardInstance, CardRegistryImpl, ChainLink, ChainStack, GameState, Phase, PlayerState,
    PlayerZones,
};
