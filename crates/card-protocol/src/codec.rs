use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;

use crate::error::ProtocolError;
use crate::message::NetworkMessage;

/// Maximum allowed message size (1 MiB).
const MAX_MESSAGE_SIZE: u32 = 1024 * 1024;

/// A framed TCP connection that sends and receives [`NetworkMessage`] values.
///
/// Frame format: `| 4 bytes: payload length (big-endian u32) | N bytes: bincode payload |`
pub struct TcpConnection {
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
}

impl TcpConnection {
    /// Wrap a [`TcpStream`] into a framed connection.
    pub fn from_stream(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        Self { reader, writer }
    }

    /// Send a [`NetworkMessage`], length-prefixed with bincode payload.
    pub async fn send(&mut self, msg: &NetworkMessage) -> Result<(), ProtocolError> {
        let payload = bincode::serialize(msg).map_err(|e| ProtocolError::SerializationError {
            reason: e.to_string(),
        })?;

        let len = payload.len() as u32;
        if len > MAX_MESSAGE_SIZE {
            return Err(ProtocolError::InvalidMessage {
                reason: format!("outgoing message too large: {} bytes", len),
            });
        }

        self.writer
            .write_all(&len.to_be_bytes())
            .await
            .map_err(|e| ProtocolError::SerializationError {
                reason: e.to_string(),
            })?;

        self.writer
            .write_all(&payload)
            .await
            .map_err(|e| ProtocolError::SerializationError {
                reason: e.to_string(),
            })?;

        self.writer
            .flush()
            .await
            .map_err(|e| ProtocolError::SerializationError {
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Receive a [`NetworkMessage`], reading a length-prefixed bincode frame.
    pub async fn recv(&mut self) -> Result<NetworkMessage, ProtocolError> {
        let mut len_buf = [0u8; 4];
        self.reader
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| ProtocolError::DeserializationError {
                reason: e.to_string(),
            })?;

        let len = u32::from_be_bytes(len_buf);

        if len > MAX_MESSAGE_SIZE {
            return Err(ProtocolError::InvalidMessage {
                reason: format!("incoming message too large: {} bytes", len),
            });
        }

        let mut buf = vec![0u8; len as usize];
        self.reader
            .read_exact(&mut buf)
            .await
            .map_err(|e| ProtocolError::DeserializationError {
                reason: e.to_string(),
            })?;

        bincode::deserialize(&buf).map_err(|e| ProtocolError::DeserializationError {
            reason: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{GameEvent, GameOverReason, NetworkMessage};
    use card_core::types::PlayerId;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_loopback_ping_pong() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = TcpConnection::from_stream(stream);
            let msg = conn.recv().await.unwrap();
            assert!(matches!(msg, NetworkMessage::Ping));
            conn.send(&NetworkMessage::Pong).await.unwrap();
        });

        let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut client = TcpConnection::from_stream(client_stream);
        client.send(&NetworkMessage::Ping).await.unwrap();
        let reply = client.recv().await.unwrap();
        assert!(matches!(reply, NetworkMessage::Pong));

        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_complex_message_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let expected_msg = NetworkMessage::EventNotification {
            event: GameEvent::GameOver {
                winner: Some(PlayerId::Player1),
                reason: GameOverReason::HpZero,
            },
        };
        let expected_clone = expected_msg.clone();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = TcpConnection::from_stream(stream);
            let msg = conn.recv().await.unwrap();
            let json1 = serde_json::to_string(&msg).unwrap();
            let json2 = serde_json::to_string(&expected_clone).unwrap();
            assert_eq!(json1, json2);
        });

        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut client = TcpConnection::from_stream(stream);
        client.send(&expected_msg).await.unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_oversized_message_rejected() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = TcpConnection::from_stream(stream);
            use tokio::io::AsyncWriteExt;
            let huge_len: u32 = 2 * 1024 * 1024;
            conn.writer
                .write_all(&huge_len.to_be_bytes())
                .await
                .unwrap();
        });

        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut client = TcpConnection::from_stream(stream);
        let result = client.recv().await;
        assert!(result.is_err());

        server.await.unwrap();
    }
}
