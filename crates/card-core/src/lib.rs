// card-core: game engine core

#![recursion_limit = "512"]

pub mod deck;
pub mod effect;
pub mod engine;
pub mod error;
pub mod rules;
pub mod state;
pub mod types;

#[cfg(test)]
mod test_godot;

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
