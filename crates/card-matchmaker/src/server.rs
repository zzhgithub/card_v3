use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, oneshot};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{error, info, warn};

const MATCH_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MatchMessage {
    MatchRequest { player_name: String, version: String },
    MatchFound {
        opponent_name: String,
        host_addr: String,
        is_host: bool,
    },
    MatchCancel,
    VersionMismatch { required: String },
    Error { message: String },
    WaitingForMatch,
}

pub struct MatchmakerServer {
    version: String,
    port: u16,
    waiting_queue: Arc<Mutex<VecDeque<WaitingPlayer>>>,
    player_id_seq: Arc<AtomicU64>,
}

struct WaitingPlayer {
    id: u64,
    player_name: String,
    host_addr: String,
    notifier: oneshot::Sender<MatchAssignment>,
}

#[derive(Debug)]
struct MatchAssignment {
    opponent_name: String,
    host_addr: String,
    is_host: bool,
}

impl MatchmakerServer {
    pub fn new(version: String, port: u16) -> Self {
        Self {
            version,
            port,
            waiting_queue: Arc::new(Mutex::new(VecDeque::new())),
            player_id_seq: Arc::new(AtomicU64::new(1)),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let bind_addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&bind_addr).await?;
        info!(addr = %bind_addr, required_version = %self.version, "matchmaker server started");

        loop {
            let (stream, addr) = listener.accept().await?;
            info!(peer = %addr, "incoming websocket connection");

            let queue = Arc::clone(&self.waiting_queue);
            let required_version = self.version.clone();
            let player_id_seq = Arc::clone(&self.player_id_seq);

            tokio::spawn(async move {
                if let Err(err) =
                    handle_connection(stream, queue, required_version, player_id_seq).await
                {
                    error!(peer = %addr, error = %err, "connection handling failed");
                }
            });
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    queue: Arc<Mutex<VecDeque<WaitingPlayer>>>,
    required_version: String,
    player_id_seq: Arc<AtomicU64>,
) -> Result<()> {
    let peer_addr = stream.peer_addr()?.to_string();
    let mut ws = accept_async(stream).await?;

    let request = match read_match_request(&mut ws).await {
        Ok(req) => req,
        Err(err) => {
            let _ = send_message(
                &mut ws,
                &MatchMessage::Error {
                    message: err.to_string(),
                },
            )
            .await;
            let _ = ws.close(None).await;
            return Ok(());
        }
    };

    let (player_name, client_version) = match request {
        MatchMessage::MatchRequest {
            player_name,
            version,
        } => (player_name, version),
        _ => {
            send_message(
                &mut ws,
                &MatchMessage::Error {
                    message: "first message must be MatchRequest".to_string(),
                },
            )
            .await?;
            ws.close(None).await?;
            return Ok(());
        }
    };

    if client_version != required_version {
        warn!(
            player = %player_name,
            client_version = %client_version,
            required_version = %required_version,
            "version mismatch"
        );
        send_message(
            &mut ws,
            &MatchMessage::VersionMismatch {
                required: required_version,
            },
        )
        .await?;
        ws.close(None).await?;
        return Ok(());
    }

    let player_id = player_id_seq.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = oneshot::channel();

    info!(player = %player_name, player_id, "player entered waiting queue");

    let pair_to_match = {
        let mut guard = queue.lock().await;
        guard.push_back(WaitingPlayer {
            id: player_id,
            player_name: player_name.clone(),
            host_addr: peer_addr.clone(),
            notifier: tx,
        });

        if guard.len() >= 2 {
            let first = guard.pop_front();
            let second = guard.pop_front();
            first.zip(second)
        } else {
            None
        }
    };

    if let Some((host, guest)) = pair_to_match {
        info!(host = %host.player_name, guest = %guest.player_name, "match found");

        let _ = host.notifier.send(MatchAssignment {
            opponent_name: guest.player_name.clone(),
            host_addr: host.host_addr.clone(),
            is_host: true,
        });

        let _ = guest.notifier.send(MatchAssignment {
            opponent_name: host.player_name,
            host_addr: host.host_addr,
            is_host: false,
        });
    } else {
        send_message(&mut ws, &MatchMessage::WaitingForMatch).await?;
    }

    match tokio::time::timeout(MATCH_TIMEOUT, rx).await {
        Ok(Ok(assignment)) => {
            send_message(
                &mut ws,
                &MatchMessage::MatchFound {
                    opponent_name: assignment.opponent_name,
                    host_addr: assignment.host_addr,
                    is_host: assignment.is_host,
                },
            )
            .await?;
            Ok(())
        }
        Ok(Err(_)) => {
            send_message(
                &mut ws,
                &MatchMessage::Error {
                    message: "match cancelled".to_string(),
                },
            )
            .await?;
            ws.close(None).await?;
            Ok(())
        }
        Err(_) => {
            {
                let mut guard = queue.lock().await;
                if let Some(idx) = guard.iter().position(|p| p.id == player_id) {
                    guard.remove(idx);
                }
            }

            warn!(player = %player_name, "match timeout");
            send_message(
                &mut ws,
                &MatchMessage::Error {
                    message: "match timeout".to_string(),
                },
            )
            .await?;
            Ok(())
        }
    }
}

async fn read_match_request(
    ws: &mut tokio_tungstenite::WebSocketStream<TcpStream>,
) -> Result<MatchMessage> {
    let msg = tokio::time::timeout(MATCH_TIMEOUT, ws.next())
        .await?
        .ok_or_else(|| anyhow::anyhow!("connection closed before request"))??;

    match msg {
        Message::Text(text) => Ok(serde_json::from_str::<MatchMessage>(text.as_ref())?),
        _ => Err(anyhow::anyhow!("first message must be text json")),
    }
}

async fn send_message(
    ws: &mut tokio_tungstenite::WebSocketStream<TcpStream>,
    msg: &MatchMessage,
) -> Result<()> {
    let payload = serde_json::to_string(msg)?;
    ws.send(Message::Text(payload.into())).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MatchMessage, MatchmakerServer};
    use anyhow::{Result, anyhow};
    use futures_util::{SinkExt, StreamExt};
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::time::Duration;
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    static NEXT_PORT: AtomicU16 = AtomicU16::new(19090);

    fn next_port() -> u16 {
        NEXT_PORT.fetch_add(1, Ordering::Relaxed)
    }

    async fn recv_message(
        ws: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ) -> Result<MatchMessage> {
        let msg = tokio::time::timeout(Duration::from_secs(3), ws.next())
            .await?
            .ok_or_else(|| anyhow!("connection closed"))??;

        match msg {
            Message::Text(text) => Ok(serde_json::from_str(text.as_ref())?),
            other => Err(anyhow!("unexpected ws message: {other:?}")),
        }
    }

    async fn recv_match_found(
        ws: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ) -> Result<(String, String, bool)> {
        for _ in 0..3 {
            match recv_message(ws).await? {
                MatchMessage::WaitingForMatch => continue,
                MatchMessage::MatchFound {
                    opponent_name,
                    host_addr,
                    is_host,
                } => return Ok((opponent_name, host_addr, is_host)),
                other => return Err(anyhow!("unexpected response: {other:?}")),
            }
        }
        Err(anyhow!("did not receive MatchFound in time"))
    }

    async fn wait_for_ready(port: u16) -> Result<()> {
        for _ in 0..40 {
            if tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
                .await
                .is_ok()
            {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        Err(anyhow!("server did not become ready"))
    }

    #[tokio::test]
    async fn two_clients_are_matched() -> Result<()> {
        let port = next_port();
        let server = MatchmakerServer::new("0.1.0".to_string(), port);
        let server_task = tokio::spawn(async move { server.run().await });

        wait_for_ready(port).await?;

        let (mut ws1, _) = connect_async(format!("ws://127.0.0.1:{port}")).await?;
        let (mut ws2, _) = connect_async(format!("ws://127.0.0.1:{port}")).await?;

        let req1 = serde_json::to_string(&MatchMessage::MatchRequest {
            player_name: "alice".to_string(),
            version: "0.1.0".to_string(),
        })?;
        let req2 = serde_json::to_string(&MatchMessage::MatchRequest {
            player_name: "bob".to_string(),
            version: "0.1.0".to_string(),
        })?;

        ws1.send(Message::Text(req1.into())).await?;
        ws2.send(Message::Text(req2.into())).await?;

        let found1 = recv_match_found(&mut ws1).await?;
        let found2 = recv_match_found(&mut ws2).await?;
        let found = [found1, found2];

        assert!(found.iter().any(|(_, _, is_host)| *is_host));
        assert!(found.iter().any(|(_, _, is_host)| !*is_host));
        assert!(found.iter().all(|(_, host_addr, _)| !host_addr.is_empty()));

        server_task.abort();
        Ok(())
    }

    #[tokio::test]
    async fn version_mismatch_is_rejected() -> Result<()> {
        let port = next_port();
        let server = MatchmakerServer::new("0.1.0".to_string(), port);
        let server_task = tokio::spawn(async move { server.run().await });

        wait_for_ready(port).await?;

        let (mut ws, _) = connect_async(format!("ws://127.0.0.1:{port}")).await?;
        let req = serde_json::to_string(&MatchMessage::MatchRequest {
            player_name: "alice".to_string(),
            version: "9.9.9".to_string(),
        })?;
        ws.send(Message::Text(req.into())).await?;

        let incoming = recv_message(&mut ws).await?;
        match incoming {
            MatchMessage::VersionMismatch { required } => assert_eq!(required, "0.1.0"),
            other => return Err(anyhow!("unexpected response: {other:?}")),
        }

        server_task.abort();
        Ok(())
    }
}
