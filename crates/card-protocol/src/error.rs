use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Protocol errors that can be serialized and sent over the network.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ProtocolError {
    #[error("serialization error: {reason}")]
    SerializationError { reason: String },

    #[error("deserialization error: {reason}")]
    DeserializationError { reason: String },

    #[error("invalid message: {reason}")]
    InvalidMessage { reason: String },

    #[error("version mismatch: expected {expected}, got {got}")]
    VersionMismatch { expected: String, got: String },
}
