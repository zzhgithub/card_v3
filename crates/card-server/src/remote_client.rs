//! Remote client that proxies [`ClientApi`] calls over a TCP connection.
//!
//! [`RemoteClient`] wraps a [`TcpConnection`] and converts async trait method
//! calls into network messages. If the connection is lost, all choice methods
//! return `None` and `on_event` becomes a no-op.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, warn};

use card_client::api::{CardOption, ClientApi, TargetOption, VisibleGameState};
use card_core::types::{CardRef, InstanceId, PlayerId, TargetRef};
use card_protocol::codec::TcpConnection;
use card_protocol::message::{AvailableAction, Command, GameEvent, NetworkMessage};

/// A remote player client that proxies [`ClientApi`] calls over a TCP connection.
///
/// Each method sends a [`NetworkMessage`] request and waits for the corresponding
/// response within the given timeout. On I/O error the client marks itself as
/// disconnected and all subsequent calls short-circuit.
pub struct RemoteClient {
    conn: Mutex<TcpConnection>,
    disconnected: AtomicBool,
    player_id: PlayerId,
}

impl RemoteClient {
    pub fn new(conn: TcpConnection, player_id: PlayerId) -> Self {
        Self {
            conn: Mutex::new(conn),
            disconnected: AtomicBool::new(false),
            player_id,
        }
    }

    fn is_disconnected(&self) -> bool {
        self.disconnected.load(Ordering::Acquire)
    }

    fn mark_disconnected(&self) {
        self.disconnected.store(true, Ordering::Release);
        warn!(player = ?self.player_id, "remote client disconnected");
    }

    pub fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub fn is_connected(&self) -> bool {
        !self.is_disconnected()
    }
}

#[async_trait]
impl ClientApi for RemoteClient {
    async fn on_event(&self, event: &GameEvent, _state: &VisibleGameState) {
        if self.is_disconnected() {
            return;
        }
        let msg = NetworkMessage::EventNotification {
            event: event.clone(),
        };
        let mut conn = self.conn.lock().await;
        if conn.send(&msg).await.is_err() {
            self.mark_disconnected();
        }
    }

    async fn choose_action(
        &self,
        available: &[AvailableAction],
        op_timeout: Duration,
    ) -> Option<Command> {
        if self.is_disconnected() {
            return None;
        }

        {
            let msg = NetworkMessage::RequestAction {
                player: self.player_id,
                available_actions: available.to_vec(),
                timeout_secs: op_timeout.as_secs(),
            };
            let mut conn = self.conn.lock().await;
            if conn.send(&msg).await.is_err() {
                self.mark_disconnected();
                return None;
            }
        }

        let recv_result = timeout(op_timeout, async {
            let mut conn = self.conn.lock().await;
            conn.recv().await
        })
        .await;

        match recv_result {
            Ok(Ok(NetworkMessage::CommandResponse { command })) => {
                debug!(player = ?self.player_id, "received command response");
                Some(command)
            }
            Ok(Err(e)) => {
                warn!(player = ?self.player_id, error = %e, "recv error in choose_action");
                self.mark_disconnected();
                None
            }
            Err(_) => {
                debug!(player = ?self.player_id, "choose_action timed out");
                None
            }
            Ok(Ok(other)) => {
                warn!(player = ?self.player_id, msg = ?other, "unexpected message in choose_action");
                None
            }
        }
    }

    async fn choose_targets(
        &self,
        options: &[TargetOption],
        count: usize,
        op_timeout: Duration,
    ) -> Option<Vec<InstanceId>> {
        if self.is_disconnected() {
            return None;
        }

        {
            let msg = NetworkMessage::RequestTargetSelection {
                player: self.player_id,
                prompt: String::new(),
                candidates: options.iter().map(|o| o.target.clone()).collect(),
                count,
                timeout_secs: op_timeout.as_secs(),
            };
            let mut conn = self.conn.lock().await;
            if conn.send(&msg).await.is_err() {
                self.mark_disconnected();
                return None;
            }
        }

        let recv_result = timeout(op_timeout, async {
            let mut conn = self.conn.lock().await;
            conn.recv().await
        })
        .await;

        match recv_result {
            Ok(Ok(NetworkMessage::TargetResponse { targets })) => {
                let ids: Vec<InstanceId> = targets
                    .iter()
                    .filter_map(|t| {
                        if let TargetRef::Card(CardRef::ByInstanceId(id)) = t {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .collect();
                Some(ids)
            }
            Ok(Err(e)) => {
                warn!(player = ?self.player_id, error = %e, "recv error in choose_targets");
                self.mark_disconnected();
                None
            }
            Err(_) => {
                debug!(player = ?self.player_id, "choose_targets timed out");
                None
            }
            Ok(Ok(other)) => {
                warn!(player = ?self.player_id, msg = ?other, "unexpected message in choose_targets");
                None
            }
        }
    }

    async fn choose_cards(
        &self,
        options: &[CardOption],
        count: usize,
        op_timeout: Duration,
    ) -> Option<Vec<InstanceId>> {
        if self.is_disconnected() {
            return None;
        }

        {
            let msg = NetworkMessage::RequestCardSelection {
                player: self.player_id,
                prompt: String::new(),
                candidates: options.iter().map(|o| o.instance_id).collect(),
                count,
                timeout_secs: op_timeout.as_secs(),
            };
            let mut conn = self.conn.lock().await;
            if conn.send(&msg).await.is_err() {
                self.mark_disconnected();
                return None;
            }
        }

        let recv_result = timeout(op_timeout, async {
            let mut conn = self.conn.lock().await;
            conn.recv().await
        })
        .await;

        match recv_result {
            Ok(Ok(NetworkMessage::CardResponse { instance_ids })) => Some(instance_ids),
            Ok(Err(e)) => {
                warn!(player = ?self.player_id, error = %e, "recv error in choose_cards");
                self.mark_disconnected();
                None
            }
            Err(_) => {
                debug!(player = ?self.player_id, "choose_cards timed out");
                None
            }
            Ok(Ok(other)) => {
                warn!(player = ?self.player_id, msg = ?other, "unexpected message in choose_cards");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use card_client::api::game_state_to_visible;
    use card_core::rules::GameRules;
    use card_core::state::GameState;
    use card_core::types::{CardRef, InstanceId, PlayerId, TargetRef};
    use card_protocol::codec::TcpConnection;
    use card_protocol::message::{Command, GameEvent, NetworkMessage};
    use tokio::net::TcpListener;

    fn make_visible_state() -> VisibleGameState {
        let state = GameState::new(GameRules::default(), 42);
        game_state_to_visible(&state, PlayerId::Player1)
    }

    #[tokio::test]
    async fn remote_client_choose_action_receives_command() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = TcpConnection::from_stream(stream);
            let _msg = conn.recv().await.unwrap();
            conn.send(&NetworkMessage::CommandResponse {
                command: Command::ChainPass,
            })
            .await
            .unwrap();
        });

        let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let client = RemoteClient::new(TcpConnection::from_stream(client_stream), PlayerId::Player1);
        let result = client.choose_action(&[], Duration::from_secs(5)).await;
        assert!(matches!(result, Some(Command::ChainPass)));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn remote_client_timeout_returns_none() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let _server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(10)).await;
        });

        let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let client = RemoteClient::new(TcpConnection::from_stream(client_stream), PlayerId::Player1);
        let result = client.choose_action(&[], Duration::from_millis(100)).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn remote_client_disconnected_returns_none() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            drop(stream);
        });

        let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let client = RemoteClient::new(TcpConnection::from_stream(client_stream), PlayerId::Player2);

        server.await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let result = client.choose_action(&[], Duration::from_secs(1)).await;
        assert!(result.is_none());

        assert!(!client.is_connected());
        let result2 = client.choose_action(&[], Duration::from_secs(1)).await;
        assert!(result2.is_none());
    }

    #[tokio::test]
    async fn remote_client_choose_cards_receives_ids() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = TcpConnection::from_stream(stream);
            let _msg = conn.recv().await.unwrap();
            conn.send(&NetworkMessage::CardResponse {
                instance_ids: vec![InstanceId(1), InstanceId(2)],
            })
            .await
            .unwrap();
        });

        let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let client = RemoteClient::new(TcpConnection::from_stream(client_stream), PlayerId::Player1);
        let result = client.choose_cards(&[], 2, Duration::from_secs(5)).await;
        assert_eq!(result, Some(vec![InstanceId(1), InstanceId(2)]));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn remote_client_choose_targets_converts_refs() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = TcpConnection::from_stream(stream);
            let _msg = conn.recv().await.unwrap();
            conn.send(&NetworkMessage::TargetResponse {
                targets: vec![
                    TargetRef::Card(CardRef::ByInstanceId(InstanceId(42))),
                    TargetRef::Card(CardRef::ByInstanceId(InstanceId(99))),
                ],
            })
            .await
            .unwrap();
        });

        let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let client = RemoteClient::new(TcpConnection::from_stream(client_stream), PlayerId::Player1);
        let result = client.choose_targets(&[], 2, Duration::from_secs(5)).await;
        assert_eq!(result, Some(vec![InstanceId(42), InstanceId(99)]));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn remote_client_on_event_sends_notification() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = TcpConnection::from_stream(stream);
            let msg = conn.recv().await.unwrap();
            match msg {
                NetworkMessage::EventNotification { event } => {
                    assert!(matches!(event, GameEvent::ChainComplete));
                }
                other => panic!("unexpected message: {:?}", other),
            }
        });

        let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let client = RemoteClient::new(TcpConnection::from_stream(client_stream), PlayerId::Player1);
        let visible = make_visible_state();
        client.on_event(&GameEvent::ChainComplete, &visible).await;
        server.await.unwrap();
    }
}
