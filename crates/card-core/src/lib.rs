// card-core: game engine core

pub mod effect;
pub mod engine;
pub mod error;
pub mod rules;
pub mod state;
pub mod types;

pub use error::CoreError;
pub use engine::modifier::{ImmunityCheck, ModifierManager};
pub use engine::trigger::{TriggerChecker, TriggeredEffect};
pub use rules::{validate_deck, CardRegistry, GameRules};
pub use state::{
    CardInstance, CardRegistryImpl, ChainLink, ChainStack, CoreGameEvent, GameState, Phase,
    PlayerState, PlayerZones,
};
