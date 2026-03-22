use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use card_client::api::{CardOption, ClientApi};
use card_core::engine::phase::{PhaseAction, PhaseClient, RecoveryCardOption};
use card_core::engine::GameResult;
use card_core::rules::{validate_deck, GameRules};
use card_core::types::{InstanceId, PlayerId};
use card_protocol::codec::TcpConnection;
use card_protocol::message::{AttackTarget, AvailableAction, Command, NetworkMessage};
use card_script::loader::{ScriptIndex, ScriptLoader};
use tokio::net::TcpListener;
use tokio::runtime::Handle;
use tracing::{error, info, warn};

use crate::remote_client::RemoteClient;
use crate::session::GameSession;

pub struct GameServer {
    bind_addr: SocketAddr,
    script_index: ScriptIndex,
    rules: GameRules,
    version: String,
    rng_seed: u64,
}

impl GameServer {
    pub fn new(
        bind_addr: SocketAddr,
        script_index: ScriptIndex,
        rules: GameRules,
        version: String,
        rng_seed: u64,
    ) -> Self {
        Self {
            bind_addr,
            script_index,
            rules,
            version,
            rng_seed,
        }
    }

    pub async fn start(&self) -> Result<GameResult> {
        let listener = TcpListener::bind(self.bind_addr)
            .await
            .with_context(|| format!("failed to bind {}", self.bind_addr))?;
        info!(addr = %self.bind_addr, "game server listening");

        let (mut conn1, name1) = accept_player(&listener, &self.version, PlayerId::Player1).await?;
        let (mut conn2, name2) = accept_player(&listener, &self.version, PlayerId::Player2).await?;
        info!(player1 = %name1, player2 = %name2, "both players connected");

        let deck1 = recv_deck_submit(&mut conn1, PlayerId::Player1).await?;
        let deck2 = recv_deck_submit(&mut conn2, PlayerId::Player2).await?;

        let validation_index = self.rebuild_script_index()?;
        let loader = ScriptLoader::new(validation_index);
        let registry = loader
            .load_for_game(&deck1, &deck2)
            .map_err(|e| anyhow!("failed to load scripts for deck validation: {}", e))?;

        if let Err(reason) = validate_deck(&deck1, &self.rules, &registry) {
            warn!(player = ?PlayerId::Player1, reason = %reason, "deck rejected");
            conn1
                .send(&NetworkMessage::DeckRejected {
                    reason: reason.clone(),
                })
                .await
                .context("failed to send deck rejection to player1")?;
            conn2
                .send(&NetworkMessage::Disconnect {
                    reason: "opponent deck invalid".to_string(),
                })
                .await
                .context("failed to notify player2 deck rejection")?;
            bail!("player1 deck invalid: {}", reason);
        }

        if let Err(reason) = validate_deck(&deck2, &self.rules, &registry) {
            warn!(player = ?PlayerId::Player2, reason = %reason, "deck rejected");
            conn2
                .send(&NetworkMessage::DeckRejected {
                    reason: reason.clone(),
                })
                .await
                .context("failed to send deck rejection to player2")?;
            conn1
                .send(&NetworkMessage::Disconnect {
                    reason: "opponent deck invalid".to_string(),
                })
                .await
                .context("failed to notify player1 deck rejection")?;
            bail!("player2 deck invalid: {}", reason);
        }

        conn1
            .send(&NetworkMessage::DeckAccepted)
            .await
            .context("failed to send deck accepted to player1")?;
        conn2
            .send(&NetworkMessage::DeckAccepted)
            .await
            .context("failed to send deck accepted to player2")?;

        let rules_json = serde_json::to_string(&self.rules).context("failed to serialize rules")?;
        conn1
            .send(&NetworkMessage::GameStart {
                rules_json: rules_json.clone(),
            })
            .await
            .context("failed to send GameStart to player1")?;
        conn2
            .send(&NetworkMessage::GameStart { rules_json })
            .await
            .context("failed to send GameStart to player2")?;

        let runtime = Handle::current();
        let client1 = Arc::new(RemoteClient::new(conn1, PlayerId::Player1));
        let client2 = Arc::new(RemoteClient::new(conn2, PlayerId::Player2));

        let mut session = GameSession::new(
            self.rules.clone(),
            self.rebuild_script_index()?,
            self.rng_seed,
        );
        session
            .add_player(
                Box::new(RemotePhaseClient::new(Arc::clone(&client1), runtime.clone())),
                deck1,
            )
            .context("failed to add player1 to session")?;
        session
            .add_player(
                Box::new(RemotePhaseClient::new(Arc::clone(&client2), runtime)),
                deck2,
            )
            .context("failed to add player2 to session")?;

        info!("starting game session");
        let result = tokio::task::spawn_blocking(move || session.start())
            .await
            .context("session task join failed")??;
        info!(winner = ?result.winner, reason = ?result.reason, "game finished");

        Ok(result)
    }

    fn rebuild_script_index(&self) -> Result<ScriptIndex> {
        let mut paths: Vec<PathBuf> = self
            .script_index
            .iter()
            .map(|(_, p)| p.clone())
            .collect();

        if paths.is_empty() {
            return ScriptIndex::scan(Path::new("/tmp/card_server_empty_index"))
                .map_err(|e| anyhow!("failed to create empty script index: {}", e));
        }

        paths.sort();
        let mut common = paths[0]
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow!("invalid script path"))?;

        for path in paths.iter().skip(1) {
            let parent = path
                .parent()
                .ok_or_else(|| anyhow!("invalid script path"))?;
            while !parent.starts_with(&common) {
                if !common.pop() {
                    break;
                }
            }
        }

        ScriptIndex::scan(&common).map_err(|e| anyhow!("failed to rebuild script index: {}", e))
    }
}

pub struct RemotePhaseClient {
    inner: Arc<RemoteClient>,
    runtime: Handle,
}

impl RemotePhaseClient {
    pub fn new(inner: Arc<RemoteClient>, runtime: Handle) -> Self {
        Self { inner, runtime }
    }

    fn block_on<F, T>(&self, fut: F) -> T
    where
        F: Future<Output = T>,
    {
        if Handle::try_current().is_ok() {
            tokio::task::block_in_place(|| self.runtime.block_on(fut))
        } else {
            self.runtime.block_on(fut)
        }
    }
}

impl PhaseClient for RemotePhaseClient {
    fn choose_action(&self, available: &[PhaseAction], timeout: Duration) -> Option<PhaseAction> {
        let network_actions = phase_actions_to_network(available);
        let command = self.block_on(self.inner.choose_action(&network_actions, timeout))?;
        command_to_phase_action(command)
    }

    fn choose_recovery_cards(
        &self,
        options: &[RecoveryCardOption],
        count: usize,
        timeout: Duration,
    ) -> Option<Vec<InstanceId>> {
        let card_options: Vec<CardOption> = options
            .iter()
            .map(|opt| CardOption {
                instance_id: opt.instance_id,
                definition_id: opt.definition_id.clone(),
                description: format!("{}", opt.definition_id),
            })
            .collect();

        self.block_on(self.inner.choose_cards(&card_options, count, timeout))
    }
}

async fn accept_player(
    listener: &TcpListener,
    server_version: &str,
    player_id: PlayerId,
) -> Result<(TcpConnection, String)> {
    let (stream, addr) = listener.accept().await.context("failed to accept connection")?;
    info!(player = ?player_id, peer = %addr, "accepted TCP connection");
    let mut conn = TcpConnection::from_stream(stream);
    let player_name = perform_handshake(&mut conn, player_id, server_version).await?;
    Ok((conn, player_name))
}

async fn perform_handshake(
    conn: &mut TcpConnection,
    assigned_id: PlayerId,
    server_version: &str,
) -> Result<String> {
    conn.send(&NetworkMessage::Hello {
        version: server_version.to_string(),
        player_name: "Server".to_string(),
    })
    .await
    .context("failed to send hello")?;

    let reply = conn.recv().await.context("failed to receive hello")?;
    let (client_version, player_name) = match reply {
        NetworkMessage::Hello {
            version,
            player_name,
        } => (version, player_name),
        other => {
            conn.send(&NetworkMessage::Disconnect {
                reason: format!("expected Hello, got {:?}", other),
            })
            .await
            .ok();
            bail!("invalid handshake message: {:?}", other);
        }
    };

    if client_version != server_version {
        let reason = format!(
            "version mismatch: server={}, client={}",
            server_version, client_version
        );
        warn!(%reason, "disconnecting mismatched client");
        conn.send(&NetworkMessage::Disconnect {
            reason: reason.clone(),
        })
        .await
        .context("failed to send version mismatch disconnect")?;
        bail!(reason);
    }

    conn.send(&NetworkMessage::HelloAck {
        player_id: assigned_id,
    })
    .await
    .context("failed to send hello ack")?;

    Ok(player_name)
}

async fn recv_deck_submit(conn: &mut TcpConnection, player_id: PlayerId) -> Result<Vec<card_core::types::CardId>> {
    let message = conn
        .recv()
        .await
        .with_context(|| format!("failed to recv DeckSubmit from {:?}", player_id))?;
    match message {
        NetworkMessage::DeckSubmit { card_ids } => {
            info!(player = ?player_id, size = card_ids.len(), "received deck submit");
            Ok(card_ids)
        }
        other => {
            let reason = format!("expected DeckSubmit, got {:?}", other);
            error!(player = ?player_id, %reason, "protocol error");
            conn.send(&NetworkMessage::Disconnect {
                reason: reason.clone(),
            })
            .await
            .ok();
            bail!(reason)
        }
    }
}

fn phase_actions_to_network(actions: &[PhaseAction]) -> Vec<AvailableAction> {
    actions
        .iter()
        .map(|action| match action {
            PhaseAction::PlayCard { instance_id, .. } => AvailableAction::PlayCard {
                instance_id: *instance_id,
            },
            PhaseAction::DeclareAttack {
                attacker,
                target_slot,
            } => {
                let possible_targets = match target_slot {
                    Some(slot) => vec![AttackTarget::FrontSlot(*slot)],
                    None => vec![AttackTarget::DirectAttack],
                };
                AvailableAction::DeclareAttack {
                    attacker: *attacker,
                    possible_targets,
                }
            }
            PhaseAction::Pass => AvailableAction::ChainPass,
            PhaseAction::Surrender => AvailableAction::Surrender,
        })
        .collect()
}

fn command_to_phase_action(command: Command) -> Option<PhaseAction> {
    match command {
        Command::PlayCard {
            instance_id,
            target_zone,
            cost_payment,
        } => Some(PhaseAction::PlayCard {
            instance_id,
            target_zone,
            cost_payment: cost_payment.hand_cards,
        }),
        Command::DeclareAttack { attacker, target } => {
            let target_slot = match target {
                AttackTarget::FrontSlot(slot) => Some(slot),
                AttackTarget::DirectAttack => None,
            };
            Some(PhaseAction::DeclareAttack {
                attacker,
                target_slot,
            })
        }
        Command::ChainPass => Some(PhaseAction::Pass),
        Command::Surrender => Some(PhaseAction::Surrender),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpStream;

    #[tokio::test]
    async fn handshake_success_sends_hello_ack() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = TcpConnection::from_stream(stream);
            let name = perform_handshake(&mut conn, PlayerId::Player1, "1.0.0")
                .await
                .unwrap();
            assert_eq!(name, "Client-A");
        });

        let stream = TcpStream::connect(addr).await.unwrap();
        let mut conn = TcpConnection::from_stream(stream);

        let first = conn.recv().await.unwrap();
        match first {
            NetworkMessage::Hello {
                version,
                player_name,
            } => {
                assert_eq!(version, "1.0.0");
                assert_eq!(player_name, "Server");
            }
            other => panic!("unexpected message: {:?}", other),
        }

        conn.send(&NetworkMessage::Hello {
            version: "1.0.0".to_string(),
            player_name: "Client-A".to_string(),
        })
        .await
        .unwrap();

        let ack = conn.recv().await.unwrap();
        match ack {
            NetworkMessage::HelloAck { player_id } => {
                assert_eq!(player_id, PlayerId::Player1);
            }
            other => panic!("unexpected message: {:?}", other),
        }

        server.await.unwrap();
    }

    #[tokio::test]
    async fn handshake_version_mismatch_sends_disconnect() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = TcpConnection::from_stream(stream);
            let result = perform_handshake(&mut conn, PlayerId::Player2, "2.0.0").await;
            assert!(result.is_err());
        });

        let stream = TcpStream::connect(addr).await.unwrap();
        let mut conn = TcpConnection::from_stream(stream);

        let _hello = conn.recv().await.unwrap();
        conn.send(&NetworkMessage::Hello {
            version: "1.0.0".to_string(),
            player_name: "OldClient".to_string(),
        })
        .await
        .unwrap();

        let msg = conn.recv().await.unwrap();
        match msg {
            NetworkMessage::Disconnect { reason } => {
                assert!(reason.contains("version mismatch"));
            }
            other => panic!("unexpected message: {:?}", other),
        }

        server.await.unwrap();
    }
}
