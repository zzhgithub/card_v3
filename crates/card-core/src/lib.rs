// card-core: game engine core

pub mod deck;
pub mod effect;
pub mod engine;
pub mod error;
pub mod rules;
pub mod state;
pub mod types;

pub use error::CoreError;
pub use engine::action::ActionExecutor;
pub use engine::game_engine::{GameEngine, GameResult};
pub use engine::modifier::{ImmunityCheck, ModifierManager};
pub use engine::trigger::{TriggerChecker, TriggeredEffect};
pub use rules::{validate_deck, CardRegistry, GameRules};
pub use state::{
    CardInstance, CardRegistryImpl, ChainLink, ChainStack, CoreGameEvent, GameState, Phase,
    PlayerState, PlayerZones,
};
