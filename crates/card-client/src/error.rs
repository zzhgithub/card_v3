use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("connection error: {reason}")]
    ConnectionError { reason: String },

    #[error("timeout waiting for response")]
    Timeout,

    #[error("server rejected command: {reason}")]
    CommandRejected { reason: String },

    #[error("protocol error: {0}")]
    Protocol(#[from] card_protocol::ProtocolError),
}
