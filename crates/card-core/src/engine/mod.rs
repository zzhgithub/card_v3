//! Game engine components.
//!
//! This module contains the stateless components of the game engine:
//! trigger scanning, chain management, action execution, and modifier management.

pub mod action;
pub mod modifier;
pub mod trigger;

pub use action::ActionExecutor;
pub use modifier::{ImmunityCheck, ModifierManager};
pub use trigger::{TriggerChecker, TriggeredEffect};
