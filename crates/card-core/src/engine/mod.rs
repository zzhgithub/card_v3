//! Game engine components.
//!
//! This module contains the stateless components of the game engine:
//! trigger scanning, chain management, action execution, and modifier management.

pub mod action;
pub mod chain;
pub mod modifier;
pub mod phase;
pub mod trigger;

pub use action::ActionExecutor;
pub use chain::{ChainEntry, ChainManager};
pub use modifier::{ImmunityCheck, ModifierManager};
pub use phase::PhaseRunner;
pub use trigger::{TriggerChecker, TriggeredEffect};
