//! Room-based game session management.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use card_core::engine::phase::{PhaseAction, RecoveryCardOption};
use card_core::rules::GameRules;
use card_core::types::{CardId, InstanceId, PlayerId};
use card_protocol::message::Phase;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{error, info};

/// Room manager handles multiple game rooms.
#[derive(Clone)]
pub struct RoomManager {
    rooms: Arc<RwLock<HashMap<String, Arc<Mutex<Room>>>>>,
    rules: GameRules,
}

impl RoomManager {
    pub fn new(rules: GameRules) -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            rules,
        }
    }

    /// Join a room by ID.
    pub async fn join_room(
        &self,
        room_id: String,
        player_name: String,
    ) -> Result<RoomJoinResult> {
        let mut rooms = self.rooms.write().await;

        if let Some(room) = rooms.get(&room_id) {
            let mut room_guard = room.lock().await;
            let result = room_guard.add_player(player_name).await?;
            drop(room_guard);
            return Ok(result);
        }

        // Create new room
        let (broadcast_tx, _) = mpsc::unbounded_channel();
        let room = Arc::new(Mutex::new(Room::new(
            room_id.clone(),
            self.rules.clone(),
            broadcast_tx,
        )));

        let mut room_guard = room.lock().await;
        let result = room_guard.add_player(player_name).await?;
        drop(room_guard);

        rooms.insert(room_id, room);
        Ok(result)
    }

    /// Submit deck for a player (used for ready state).
    pub async fn submit_deck(&self, room_id: &str, player_id: PlayerId, deck: Vec<CardId>, deck_id: String) -> Result<()> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(room_id)
            .ok_or_else(|| anyhow!("Room not found"))?
            .clone();
        drop(rooms);

        let mut room_guard = room.lock().await;
        // Store the deck
        room_guard.submit_deck(player_id, deck).await?;
        // Mark player as ready with deck_id
        room_guard.set_ready(player_id, deck_id).await?;

        Ok(())
    }

    /// Set player as not ready.
    pub async fn set_unready(&self, room_id: &str, player_id: PlayerId) -> Result<()> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(room_id)
            .ok_or_else(|| anyhow!("Room not found"))?
            .clone();
        drop(rooms);

        let mut room_guard = room.lock().await;
        room_guard.set_unready(player_id).await
    }

    /// Handle player leaving room.
    pub async fn leave_room(&self, room_id: &str, player_id: PlayerId) -> Result<()> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(room_id)
            .ok_or_else(|| anyhow!("Room not found"))?
            .clone();
        drop(rooms);

        let mut room_guard = room.lock().await;
        room_guard.leave_room(player_id).await?;

        // Check if room is empty, remove it
        if room_guard.players[0].is_none() && room_guard.players[1].is_none() {
            drop(room_guard);
            let mut rooms_write = self.rooms.write().await;
            rooms_write.remove(room_id);
            info!("Room {} removed (empty)", room_id);
        }

        Ok(())
    }

    /// Query room state.
    pub async fn query_room_state(&self, room_id: &str, player_id: PlayerId) -> Result<(Vec<(String, bool, Option<String>)>, bool)> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(room_id)
            .ok_or_else(|| anyhow!("Room not found"))?
            .clone();
        drop(rooms);

        let room_guard = room.lock().await;
        Ok(room_guard.get_room_state())
    }

    /// Submit an action from a player.
    pub async fn submit_action(&self, room_id: &str, player_id: PlayerId, action: PlayerAction) -> Result<()> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(room_id)
            .ok_or_else(|| anyhow!("Room not found"))?
            .clone();
        drop(rooms);

        let room_guard = room.lock().await;
        room_guard.send_action(player_id, action)
    }

    /// Handle player disconnection.
    pub async fn player_disconnected(&self, room_id: &str, player_id: PlayerId) -> Result<()> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(room_id)
            .ok_or_else(|| anyhow!("Room not found"))?
            .clone();
        drop(rooms);

        let mut room_guard = room.lock().await;
        room_guard.handle_disconnect(player_id).await;

        // If game was in progress, log the disconnection
        if room_guard.state == RoomState::Playing {
            info!("Player disconnected from active game in room {}, game will end", room_id);
        }

        Ok(())
    }

    /// Remove a room (for cleanup).
    pub async fn remove_room(&self, room_id: &str) -> Result<()> {
        let mut rooms = self.rooms.write().await;
        if rooms.remove(room_id).is_some() {
            info!("Room {} removed", room_id);
        }
        Ok(())
    }

    /// Subscribe to room broadcast messages.
    pub async fn subscribe(&self, room_id: &str) -> Result<mpsc::UnboundedReceiver<RoomMessage>> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(room_id)
            .ok_or_else(|| anyhow!("Room not found"))?
            .clone();
        drop(rooms);

        let (tx, rx) = mpsc::unbounded_channel();
        let mut room_guard = room.lock().await;
        room_guard.add_subscriber(tx);
        Ok(rx)
    }
}

/// Result of joining a room.
#[derive(Debug, Clone)]
pub struct RoomJoinResult {
    pub player_id: PlayerId,
    pub player_name: String,
    pub slot_filled: usize,
    pub room_state: RoomState,
}

/// Room states.
#[derive(Debug, Clone, PartialEq)]
pub enum RoomState {
    Waiting,
    DeckSubmission,
    Playing,
    Finished,
}

/// Structured action option sent to clients in action requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action_type")]
pub enum ActionOption {
    #[serde(rename = "pass")]
    Pass,
    #[serde(rename = "surrender")]
    Surrender,
    #[serde(rename = "play_card")]
    PlayCard { instance_id: u32, target_zone: ActionZone },
    #[serde(rename = "declare_attack")]
    DeclareAttack { attacker: u32, target: ActionTarget },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "zone_type")]
pub enum ActionZone {
    #[serde(rename = "front")]
    Front { slot: usize },
    #[serde(rename = "back")]
    Back { slot: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "target_type")]
pub enum ActionTarget {
    #[serde(rename = "direct")]
    Direct,
    #[serde(rename = "slot")]
    Slot { slot_index: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryOption {
    pub instance_id: u32,
    pub definition_id: String,
}

/// Messages broadcasted to room participants.
#[derive(Debug, Clone)]
pub enum RoomMessage {
    PlayerJoined { player_id: PlayerId, player_name: String },
    PlayerDisconnected { player_id: PlayerId, player_name: String },
    PlayerReady { player_id: PlayerId, player_name: String, deck_id: String },
    PlayerUnready { player_id: PlayerId, player_name: String },
    WaitingForDecks,
    GameStarting,
    GameStarted,
    StateUpdate { for_player: PlayerId, state: VisibleGameState },
    ActionRequested { player_id: PlayerId, available_actions: Vec<ActionOption>, timeout_secs: u64 },
    RecoveryRequested { player_id: PlayerId, count: usize, options: Vec<RecoveryOption> },
    GameOver { winner: Option<PlayerId>, reason: String },
    Error { message: String },
}

/// A game room.
pub struct Room {
    pub id: String,
    pub state: RoomState,
    pub rules: GameRules,
    pub players: [Option<PlayerSlot>; 2],
    pub decks: [Option<Vec<CardId>>; 2],
    /// Ready state: deck_id if ready, None if not
    pub ready_state: [Option<String>; 2],
    pub broadcast_tx: mpsc::UnboundedSender<RoomMessage>,
    /// Action request states for sync/async bridge (Player1, Player2)
    pub action_states: [Option<std::sync::Arc<crate::game_starter::ActionRequestState>>; 2],
    subscribers: Vec<mpsc::UnboundedSender<RoomMessage>>,
}

#[derive(Clone)]
pub struct PlayerSlot {
    pub name: String,
    pub connected: bool,
}

/// Actions that players can send.
#[derive(Debug, Clone)]
pub enum PlayerAction {
    PhaseAction(PhaseAction),
    RecoverySelection(Vec<InstanceId>),
}

/// Visible game state for a player.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VisibleGameState {
    pub turn_number: u32,
    pub current_phase: Phase,
    pub current_player: PlayerId,
    pub your_state: PlayerVisibleState,
    pub opponent_state: OpponentVisibleState,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerVisibleState {
    pub hp: u8,
    pub real_point: u8,
    pub deck_count: usize,
    pub hand: Vec<CardVisibleInfo>,
    pub front: [Option<CardVisibleInfo>; 5],
    pub back: [Option<CardVisibleInfo>; 5],
    pub cost_zone: Vec<CardVisibleInfo>,
    pub grave: Vec<CardVisibleInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpponentVisibleState {
    pub hp: u8,
    pub real_point: u8,
    pub deck_count: usize,
    pub hand_count: usize,
    pub front: [Option<CardVisibleInfo>; 5],
    pub back: [Option<CardVisibleInfo>; 5],
    pub cost_zone: Vec<CardVisibleInfo>,
    pub grave: Vec<CardVisibleInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CardVisibleInfo {
    pub instance_id: InstanceId,
    pub definition_id: CardId,
    pub current_attack: Option<i32>,
}

impl Room {
    pub fn new(id: String, rules: GameRules, broadcast_tx: mpsc::UnboundedSender<RoomMessage>) -> Self {
        Self {
            id,
            state: RoomState::Waiting,
            rules,
            players: [None, None],
            decks: [None, None],
            ready_state: [None, None],
            broadcast_tx,
            action_states: [None, None],
            subscribers: Vec::new(),
        }
    }

    pub async fn add_player(&mut self, name: String) -> Result<RoomJoinResult> {
        let slot = if self.players[0].is_none() {
            0
        } else if self.players[1].is_none() {
            1
        } else {
            bail!("Room is full");
        };

        self.players[slot] = Some(PlayerSlot { name: name.clone(), connected: true });
        let player_id = if slot == 0 { PlayerId::Player1 } else { PlayerId::Player2 };

        info!("Player {} joined room {} as {:?}", name, self.id, player_id);
        let _ = self.broadcast(RoomMessage::PlayerJoined { player_id, player_name: name.clone() });

        if slot == 1 {
            self.state = RoomState::DeckSubmission;
            let _ = self.broadcast(RoomMessage::WaitingForDecks);
        }

        Ok(RoomJoinResult { player_id, player_name: name, slot_filled: slot, room_state: self.state.clone() })
    }

    pub async fn submit_deck(&mut self, player_id: PlayerId, deck: Vec<CardId>) -> Result<()> {
        let slot = match player_id { PlayerId::Player1 => 0, PlayerId::Player2 => 1 };
        info!("Player {:?} submitted deck with {} cards", player_id, deck.len());
        self.decks[slot] = Some(deck);
        Ok(())
    }

    /// Mark player as ready with their deck selection.
    pub async fn set_ready(&mut self, player_id: PlayerId, deck_id: String) -> Result<()> {
        let slot = match player_id { PlayerId::Player1 => 0, PlayerId::Player2 => 1 };

        if self.players[slot].is_none() {
            bail!("Player not in room");
        }

        self.ready_state[slot] = Some(deck_id.clone());
        let player_name = self.players[slot].as_ref().unwrap().name.clone();

        info!("Player {:?} ({}) is ready with deck {}", player_id, player_name, deck_id);

        // Broadcast player ready message
        let _ = self.broadcast(RoomMessage::PlayerReady {
            player_id,
            player_name: player_name.clone(),
            deck_id
        });

        // Check if both players are ready
        if self.ready_state[0].is_some() && self.ready_state[1].is_some() {
            info!("Both players ready in room {}, starting game", self.id);
            let _ = self.broadcast(RoomMessage::GameStarting);

            // Start the game
            let room_clone = Arc::new(Mutex::new(self.clone_room()));
            let rules = self.rules.clone();

            tokio::spawn(async move {
                if let Err(e) = crate::game_starter::start_game(room_clone, rules).await {
                    error!("Game start failed: {}", e);
                }
            });
        }

        Ok(())
    }

    /// Mark player as not ready.
    pub async fn set_unready(&mut self, player_id: PlayerId) -> Result<()> {
        let slot = match player_id { PlayerId::Player1 => 0, PlayerId::Player2 => 1 };

        if self.players[slot].is_none() {
            bail!("Player not in room");
        }

        self.ready_state[slot] = None;
        let player_name = self.players[slot].as_ref().unwrap().name.clone();

        info!("Player {:?} ({}) is not ready", player_id, player_name);

        // Broadcast player unready message
        let _ = self.broadcast(RoomMessage::PlayerUnready { player_id, player_name });

        Ok(())
    }

    /// Handle player leaving the room.
    pub async fn leave_room(&mut self, player_id: PlayerId) -> Result<()> {
        let slot = match player_id { PlayerId::Player1 => 0, PlayerId::Player2 => 1 };

        if let Some(player) = self.players[slot].take() {
            info!("Player {:?} ({}) left room {}", player_id, player.name, self.id);

            // Clear ready state and deck
            self.ready_state[slot] = None;
            self.decks[slot] = None;

            // Broadcast player left message
            let _ = self.broadcast(RoomMessage::PlayerDisconnected {
                player_id,
                player_name: player.name,
            });

            // If game was playing, end it
            if self.state == RoomState::Playing {
                let winner = if slot == 0 && self.players[1].is_some() {
                    Some(PlayerId::Player2)
                } else if slot == 1 && self.players[0].is_some() {
                    Some(PlayerId::Player1)
                } else {
                    None
                };

                let _ = self.broadcast(RoomMessage::GameOver {
                    winner,
                    reason: "Player left".to_string(),
                });
                self.state = RoomState::Finished;
            }
        }

        Ok(())
    }

    /// Get current room state for query.
    pub fn get_room_state(&self) -> (Vec<(String, bool, Option<String>)>, bool) {
        let mut players = Vec::new();
        let mut all_ready = true;

        for slot in 0..2 {
            if let Some(player) = &self.players[slot] {
                let is_ready = self.ready_state[slot].is_some();
                if !is_ready {
                    all_ready = false;
                }
                players.push((player.name.clone(), is_ready, self.ready_state[slot].clone()));
            } else {
                all_ready = false;
            }
        }

        (players, all_ready)
    }

    /// Clone room state for game starting.
    fn clone_room(&self) -> Self {
        Self {
            id: self.id.clone(),
            state: self.state.clone(),
            rules: self.rules.clone(),
            players: self.players.clone(),
            decks: self.decks.clone(),
            ready_state: self.ready_state.clone(),
            broadcast_tx: self.broadcast_tx.clone(),
            action_states: [None, None],
            subscribers: self.subscribers.clone(),
        }
    }

    pub fn send_action(&self, player_id: PlayerId, action: PlayerAction) -> Result<()> {
        let slot = match player_id {
            PlayerId::Player1 => 0,
            PlayerId::Player2 => 1,
        };

        if let Some(ref action_state) = self.action_states[slot] {
            let responded = action_state.respond(action);
            if !responded {
                bail!("Action already submitted or no pending request");
            }
            Ok(())
        } else {
            bail!("No action request pending for player {:?}", player_id)
        }
    }

    pub fn add_subscriber(&mut self, tx: mpsc::UnboundedSender<RoomMessage>) {
        self.subscribers.push(tx);
    }

    fn broadcast(&self, msg: RoomMessage) {
        let _ = self.broadcast_tx.send(msg.clone());
        for sub in &self.subscribers {
            let _ = sub.send(msg.clone());
        }
    }

    /// Broadcast a message to all room subscribers (public version for external use)
    pub fn broadcast_message(&self, msg: RoomMessage) {
        self.broadcast(msg);
    }

    /// Handle player disconnection.
    pub async fn handle_disconnect(&mut self, player_id: PlayerId) {
        let slot = match player_id {
            PlayerId::Player1 => 0,
            PlayerId::Player2 => 1,
        };

        if let Some(ref mut player_slot) = self.players[slot] {
            player_slot.connected = false;
            info!("Player {:?} ({}) disconnected from room {}",
                player_id, player_slot.name, self.id);

            // Signal the action state if there's a pending request
            if let Some(ref action_state) = self.action_states[slot] {
                action_state.cancel();
                info!("Player {:?} had pending action request, cancelled due to disconnect", player_id);
            }
        }

        // Broadcast disconnect event to remaining players
        let _ = self.broadcast(RoomMessage::PlayerDisconnected {
            player_id,
            player_name: self.players[slot].as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        });

        // If game is playing, end it with the disconnected player losing
        if self.state == RoomState::Playing {
            let winner = match player_id {
                PlayerId::Player1 => Some(PlayerId::Player2),
                PlayerId::Player2 => Some(PlayerId::Player1),
            };
            let _ = self.broadcast(RoomMessage::GameOver {
                winner,
                reason: format!("Player {:?} disconnected", player_id),
            });
            self.state = RoomState::Finished;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_room_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let room = Room::new("test".to_string(), GameRules::default(), tx);
        assert_eq!(room.state, RoomState::Waiting);
        assert!(room.players[0].is_none());
        assert!(room.players[1].is_none());
    }

    #[tokio::test]
    async fn test_room_join() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut room = Room::new("test".to_string(), GameRules::default(), tx);

        let result = room.add_player("Alice".to_string()).await.unwrap();
        assert_eq!(result.player_id, PlayerId::Player1);

        let msg = rx.try_recv().unwrap();
        match msg {
            RoomMessage::PlayerJoined { player_id, player_name } => {
                assert_eq!(player_id, PlayerId::Player1);
                assert_eq!(player_name, "Alice");
            }
            _ => panic!("Expected PlayerJoined message"),
        }
    }

    #[tokio::test]
    async fn test_room_full() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut room = Room::new("test".to_string(), GameRules::default(), tx);

        room.add_player("Alice".to_string()).await.unwrap();
        room.add_player("Bob".to_string()).await.unwrap();

        assert!(room.add_player("Charlie".to_string()).await.is_err());
    }
}
