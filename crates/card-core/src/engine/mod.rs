//! Game engine components.
//!
//! This module contains the stateless components of the game engine:
//! trigger scanning, chain management, action execution, and modifier management.

pub mod modifier;
pub mod trigger;

pub use modifier::{ImmunityCheck, ModifierManager};
