use thiserror::Error;

use card_core::types::CardId;

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("Lua error: {0}")]
    LuaError(#[from] mlua::Error),

    #[error("parse error: {reason}")]
    ParseError { reason: String },

    #[error("card not found in scripts: {0}")]
    CardNotFound(CardId),

    #[error("sandbox violation: {reason}")]
    SandboxViolation { reason: String },

    #[error("script timeout")]
    ScriptTimeout,

    #[error("invalid card definition: {reason}")]
    InvalidCardDefinition { reason: String },
}
