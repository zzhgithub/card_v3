//! WebSocket server for handling client connections.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use card_core::types::{CardId, InstanceId, PlayerId};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message, tungstenite::protocol::frame::Utf8Bytes};
use tracing::{debug, error, info};

use crate::room::{PlayerAction, RoomManager, RoomMessage};

/// WebSocket server that handles client connections.
pub struct WebSocketServer {
    room_manager: Arc<RoomManager>,
    bind_addr: SocketAddr,
}

impl WebSocketServer {
    pub fn new(room_manager: Arc<RoomManager>, bind_addr: SocketAddr) -> Self {
        Self {
            room_manager,
            bind_addr,
        }
    }

    /// Start the WebSocket server.
    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!("WebSocket server listening on {}", self.bind_addr);

        while let Ok((stream, addr)) = listener.accept().await {
            let room_manager = self.room_manager.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(stream, addr, room_manager).await {
                    error!("Client {} error: {}", addr, e);
                }
            });
        }

        Ok(())
    }
}

/// Client protocol messages (JSON over WebSocket).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "join_room")]
    JoinRoom { room_id: String, player_name: String },
    #[serde(rename = "leave_room")]
    LeaveRoom,
    #[serde(rename = "submit_deck")]
    SubmitDeck { deck_id: String, cards: Vec<String> }, // Card IDs as strings
    #[serde(rename = "unready")]
    Unready,
    #[serde(rename = "action")]
    Action { action: ClientAction },
    #[serde(rename = "recovery")]
    Recovery { cards: Vec<u32> }, // Instance IDs
    #[serde(rename = "query_room_state")]
    QueryRoomState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action_type")]
pub enum ClientAction {
    #[serde(rename = "pass")]
    Pass,
    #[serde(rename = "surrender")]
    Surrender,
    #[serde(rename = "play_card")]
    PlayCard {
        instance_id: u32,
        target_zone: ClientZone,
    },
    #[serde(rename = "declare_attack")]
    DeclareAttack {
        attacker_id: u32,
        target: AttackTarget,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "target_type")]
pub enum AttackTarget {
    #[serde(rename = "direct")]
    Direct,
    #[serde(rename = "slot")]
    Slot { slot_index: usize },
}

/// Player info for room state query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub name: String,
    pub is_ready: bool,
    pub deck_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "zone_type")]
pub enum ClientZone {
    #[serde(rename = "front")]
    Front { slot: usize },
    #[serde(rename = "back")]
    Back { slot: usize },
}

/// Server protocol messages (JSON over WebSocket).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "joined")]
    Joined {
        player_id: String,
        room_state: String,
    },
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "player_ready")]
    PlayerReady {
        player_name: String,
        deck_id: String,
    },
    #[serde(rename = "player_unready")]
    PlayerUnready {
        player_name: String,
    },
    #[serde(rename = "room_state")]
    RoomState {
        players: Vec<PlayerInfo>,
        all_ready: bool,
    },
    #[serde(rename = "waiting_for_deck")]
    WaitingForDeck,
    #[serde(rename = "game_starting")]
    GameStarting,
    #[serde(rename = "game_started")]
    GameStarted {
        state: serde_json::Value,
    },
    #[serde(rename = "state_update")]
    StateUpdate {
        state: serde_json::Value,
    },
    #[serde(rename = "action_request")]
    ActionRequest {
        available_actions: Vec<String>,
        timeout_secs: u64,
    },
    #[serde(rename = "recovery_request")]
    RecoveryRequest {
        count: usize,
        options: Vec<String>,
    },
    #[serde(rename = "game_over")]
    GameOver {
        winner: Option<String>,
        reason: String,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
    },
    #[serde(rename = "opponent_joined")]
    OpponentJoined {
        player_name: String,
    },
    #[serde(rename = "player_disconnected")]
    PlayerDisconnected {
        player_name: String,
    },
}

/// Per-client connection state.
struct ClientState {
    addr: SocketAddr,
    room_id: Option<String>,
    player_id: Option<PlayerId>,
    room_rx: Option<mpsc::UnboundedReceiver<RoomMessage>>,
}

async fn handle_client(
    stream: TcpStream,
    addr: SocketAddr,
    room_manager: Arc<RoomManager>,
) -> Result<()> {
    info!("New client connection from {}", addr);

    let ws = accept_async(stream).await?;
    let (mut ws_tx, mut ws_rx) = ws.split();

    let mut state = ClientState {
        addr,
        room_id: None,
        player_id: None,
        room_rx: None,
    };

    // Main message loop
    loop {
        tokio::select! {
            // WebSocket message from client
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let text_str = text.as_str();
                        debug!("Received from {}: {}", addr, text_str);
                        match handle_client_message(
                            text_str,
                            &mut state,
                            &room_manager,
                            &mut ws_tx
                        ).await {
                            Ok(true) => {}, // Continue
                            Ok(false) => break, // Disconnect
                            Err(e) => {
                                error!("Error handling message from {}: {}", addr, e);
                                let error_msg = ServerMessage::Error {
                                    message: e.to_string(),
                                };
                                let json = serde_json::to_string(&error_msg)?;
                                ws_tx.send(Message::Text(Utf8Bytes::from(json))).await?;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => {
                        info!("Client {} disconnected", addr);
                        break;
                    }
                    _ => {}
                }
            }

            // Room broadcast message
            Some(room_msg) = async {
                if let Some(ref mut rx) = state.room_rx {
                    rx.recv().await
                } else {
                    futures::future::pending().await
                }
            } => {
                let server_msg = convert_room_message(room_msg, state.player_id);
                let json = serde_json::to_string(&server_msg)?;
                debug!("Sending to {}: {}", addr, json);
                ws_tx.send(Message::Text(Utf8Bytes::from(json))).await?;
            }
        }
    }

    // Handle disconnect - notify room manager if player was in a room
    if let (Some(room_id), Some(player_id)) = (state.room_id, state.player_id) {
        info!("Notifying room {} of player {:?} disconnection", room_id, player_id);
        if let Err(e) = room_manager.player_disconnected(&room_id, player_id).await {
            error!("Failed to handle player disconnect: {}", e);
        }
    }

    Ok(())
}

async fn handle_client_message(
    text: &str,
    state: &mut ClientState,
    room_manager: &RoomManager,
    ws_tx: &mut futures::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>,
) -> Result<bool> {
    let msg: ClientMessage = serde_json::from_str(text)?;

    match msg {
        ClientMessage::JoinRoom { room_id, player_name } => {
            if state.room_id.is_some() {
                return Err(anyhow!("Already in a room"));
            }

            let result = room_manager.join_room(room_id.clone(), player_name).await?;
            state.room_id = Some(room_id.clone());
            state.player_id = Some(result.player_id);

            info!(
                "Player {:?} joined room {} as {:?}",
                result.player_name, room_id, result.player_id
            );

            // Subscribe to room broadcasts
            let room_rx = room_manager.subscribe(&room_id).await?;
            state.room_rx = Some(room_rx);

            // Send joined confirmation
            let response = ServerMessage::Joined {
                player_id: format!("{:?}", result.player_id),
                room_state: format!("{:?}", result.room_state),
            };
            let json = serde_json::to_string(&response)?;
            ws_tx.send(Message::Text(Utf8Bytes::from(json))).await?;
        }

        ClientMessage::SubmitDeck { deck_id, cards } => {
            let room_id = state
                .room_id
                .as_ref()
                .ok_or_else(|| anyhow!("Not in a room"))?;
            let player_id = state
                .player_id
                .ok_or_else(|| anyhow!("Player ID not set"))?;

            let card_ids: Vec<CardId> = cards.iter().map(|c| CardId::new(c)).collect();

            info!(
                "Player {:?} submitting deck '{}' with {} cards",
                player_id, deck_id, card_ids.len()
            );

            // Submit deck and mark as ready
            room_manager.submit_deck(room_id, player_id, card_ids, deck_id).await?;
        }

        ClientMessage::Unready => {
            let room_id = state
                .room_id
                .as_ref()
                .ok_or_else(|| anyhow!("Not in a room"))?;
            let player_id = state
                .player_id
                .ok_or_else(|| anyhow!("Player ID not set"))?;

            info!("Player {:?} is unready", player_id);

            room_manager.set_unready(room_id, player_id).await?;
        }

        ClientMessage::LeaveRoom => {
            let room_id = state
                .room_id
                .as_ref()
                .ok_or_else(|| anyhow!("Not in a room"))?;
            let player_id = state
                .player_id
                .ok_or_else(|| anyhow!("Player ID not set"))?;

            info!("Player {:?} leaving room {}", player_id, room_id);

            room_manager.leave_room(room_id, player_id).await?;

            // Clear state
            state.room_id = None;
            state.player_id = None;
            state.room_rx = None;

            // Send confirmation
            let response = ServerMessage::Left;
            let json = serde_json::to_string(&response)?;
            ws_tx.send(Message::Text(Utf8Bytes::from(json))).await?;
        }

        ClientMessage::QueryRoomState => {
            let room_id = state
                .room_id
                .as_ref()
                .ok_or_else(|| anyhow!("Not in a room"))?;
            let player_id = state
                .player_id
                .ok_or_else(|| anyhow!("Player ID not set"))?;

            let (players, all_ready) = room_manager.query_room_state(room_id, player_id).await?;

            let player_infos: Vec<PlayerInfo> = players
                .into_iter()
                .map(|(name, is_ready, deck_id)| PlayerInfo { name, is_ready, deck_id })
                .collect();

            let response = ServerMessage::RoomState {
                players: player_infos,
                all_ready,
            };
            let json = serde_json::to_string(&response)?;
            ws_tx.send(Message::Text(Utf8Bytes::from(json))).await?;
        }

        ClientMessage::Action { action } => {
            let room_id = state
                .room_id
                .as_ref()
                .ok_or_else(|| anyhow!("Not in a room"))?;
            let player_id = state
                .player_id
                .ok_or_else(|| anyhow!("Player ID not set"))?;

            let phase_action = convert_client_action(action)?;

            debug!("Player {:?} sending action: {:?}", player_id, phase_action);

            room_manager
                .submit_action(room_id, player_id, PlayerAction::PhaseAction(phase_action))
                .await?;
        }

        ClientMessage::Recovery { cards } => {
            let room_id = state
                .room_id
                .as_ref()
                .ok_or_else(|| anyhow!("Not in a room"))?;
            let player_id = state
                .player_id
                .ok_or_else(|| anyhow!("Player ID not set"))?;

            let instance_ids: Vec<InstanceId> =
                cards.iter().map(|c| InstanceId(*c)).collect();

            room_manager
                .submit_action(room_id, player_id, PlayerAction::RecoverySelection(instance_ids))
                .await?;
        }
    }

    Ok(true)
}

fn convert_client_action(action: ClientAction) -> Result<card_core::engine::phase::PhaseAction> {
    use card_core::engine::phase::PhaseAction;
    use card_core::types::Zone;

    match action {
        ClientAction::Pass => Ok(PhaseAction::Pass),
        ClientAction::Surrender => Ok(PhaseAction::Surrender),
        ClientAction::PlayCard {
            instance_id,
            target_zone,
        } => {
            let zone = match target_zone {
                ClientZone::Front { slot } => Zone::Front(slot),
                ClientZone::Back { slot } => Zone::Back(slot),
            };
            Ok(PhaseAction::PlayCard {
                instance_id: InstanceId(instance_id),
                target_zone: zone,
                cost_payment: vec![], // Simplified - should calculate from client
            })
        }
        ClientAction::DeclareAttack {
            attacker_id,
            target,
        } => {
            let target_slot = match target {
                AttackTarget::Direct => None,
                AttackTarget::Slot { slot_index } => Some(slot_index),
            };
            Ok(PhaseAction::DeclareAttack {
                attacker: InstanceId(attacker_id),
                target_slot,
            })
        }
    }
}

fn convert_room_message(msg: RoomMessage, my_player_id: Option<PlayerId>) -> ServerMessage {
    use crate::room::RoomMessage::*;

    match msg {
        PlayerJoined { player_name, .. } => ServerMessage::OpponentJoined { player_name },
        PlayerDisconnected { player_name, .. } => ServerMessage::PlayerDisconnected { player_name },
        PlayerReady { player_name, deck_id, .. } => ServerMessage::PlayerReady { player_name, deck_id },
        PlayerUnready { player_name, .. } => ServerMessage::PlayerUnready { player_name },
        WaitingForDecks => ServerMessage::WaitingForDeck,
        GameStarting => ServerMessage::GameStarting,
        GameStarted { state } => ServerMessage::GameStarted {
            state: serde_json::to_value(state).unwrap_or_default(),
        },
        StateUpdate { for_player, state } => {
            // Only send state intended for this player
            if my_player_id == Some(for_player) {
                ServerMessage::StateUpdate {
                    state: serde_json::to_value(state).unwrap_or_default(),
                }
            } else {
                ServerMessage::StateUpdate {
                    state: serde_json::Value::Null,
                }
            }
        }
        ActionRequested { player_id, available_actions: _, timeout_secs } => {
            if my_player_id == Some(player_id) {
                ServerMessage::ActionRequest {
                    available_actions: vec![], // Simplified
                    timeout_secs,
                }
            } else {
                ServerMessage::StateUpdate {
                    state: serde_json::Value::Null,
                }
            }
        }
        RecoveryRequested { player_id, count, options: _ } => {
            if my_player_id == Some(player_id) {
                ServerMessage::RecoveryRequest {
                    count,
                    options: vec![], // Simplified
                }
            } else {
                ServerMessage::StateUpdate {
                    state: serde_json::Value::Null,
                }
            }
        }
        GameOver { winner, reason } => ServerMessage::GameOver {
            winner: winner.map(|p| format!("{:?}", p)),
            reason,
        },
        Error { message } => ServerMessage::Error { message },
        _ => ServerMessage::StateUpdate {
            state: serde_json::Value::Null,
        },
    }
}
