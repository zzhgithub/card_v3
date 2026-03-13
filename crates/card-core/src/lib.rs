// card-core: game engine core

pub mod effect;
pub mod rules;
pub mod types;

pub use rules::{validate_deck, CardRegistry, GameRules};
