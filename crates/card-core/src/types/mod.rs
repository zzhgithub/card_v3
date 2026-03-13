//! Core domain types for the card game engine.
//!
//! This module defines all fundamental types used throughout the game,
//! including card identifiers, player references, zones, and card definitions.

mod card_definition;
mod card_id;
mod card_types;
mod player;
mod references;
mod zone;

#[cfg(test)]
mod tests;

pub use card_definition::{CardDefinition, CardFilter};
pub use card_id::{CardId, CardIdParseError};
pub use card_types::{CardType, Category, EffectKey, InstanceId, ItemKind, Property, StrategyKind};
pub use player::{PlayerId, PlayerRef};
pub use references::{CardRef, TargetRef};
pub use zone::{Zone, ZoneLocation};

pub use serde::{Deserialize, Serialize};
