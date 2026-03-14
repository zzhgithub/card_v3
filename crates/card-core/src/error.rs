use thiserror::Error;

use crate::types::CardId;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid card ID format: {0}")]
    InvalidCardId(String),

    #[error("card not found: {0}")]
    CardNotFound(CardId),

    #[error("invalid zone operation: {reason}")]
    InvalidZone { reason: String },

    #[error("rule violation: {reason}")]
    RuleViolation { reason: String },

    #[error("invalid command: {reason}")]
    InvalidCommand { reason: String },

    #[error("invalid target: {reason}")]
    InvalidTarget { reason: String },

    #[error("effect error: {reason}")]
    EffectError { reason: String },

    #[error("game over")]
    GameOver,
}
