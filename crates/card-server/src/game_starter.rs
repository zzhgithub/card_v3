//! Game starter - handles launching the game when both players are ready.
//!
//! This module bridges the async room management with the synchronous GameEngine.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex as StdMutex};
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use card_core::engine::phase::{ChainActivateOption, PhaseAction, PhaseClient, RecoveryCardOption};
use card_core::engine::GameEngine;
use card_core::rules::{validate_deck, GameRules};
use card_core::state::{CardInstance, CardRegistryImpl, CoreGameEvent, GameState};
use card_core::types::CardDefinition;
use card_core::types::{CardId, CardType, Category, InstanceId, ItemKind, PlayerId, Property, StrategyKind, Zone};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

use crate::room::{ActionOption, ActionTarget, ActionZone, ChainEffectOption, PlayerAction, RecoveryOption, Room, RoomMessage, VisibleGameState};

/// Shared state for sync/async bridge between game engine and network clients.
#[derive(Debug)]
pub struct ActionRequestState {
    /// The pending action response (None = waiting, Some = received)
    response: std::sync::Mutex<Option<PlayerAction>>,
    /// Condition variable to signal when response is available
    condvar: std::sync::Condvar,
    /// Cancellation flag for player disconnection
    cancelled: AtomicBool,
}

impl ActionRequestState {
    pub fn new() -> Self {
        Self {
            response: std::sync::Mutex::new(None),
            condvar: std::sync::Condvar::new(),
            cancelled: AtomicBool::new(false),
        }
    }

    /// Wait for a response with timeout. Returns None if timeout or cancelled.
    pub fn wait_response(&self, timeout: Duration) -> Option<PlayerAction> {
        let mut response = self.response.lock().unwrap();

        // Check if already cancelled
        if self.cancelled.load(Ordering::Relaxed) {
            return None;
        }

        let result = self
            .condvar
            .wait_timeout_while(response, timeout, |r| r.is_none())
            .unwrap();
        response = result.0;

        // Check cancellation after being woken up
        if self.cancelled.load(Ordering::Relaxed) {
            return None;
        }

        response.take()
    }

    /// Submit a response. Returns false if already responded.
    pub fn respond(&self, action: PlayerAction) -> bool {
        let mut response = self.response.lock().unwrap();
        if response.is_some() {
            return false; // Already have a response
        }
        *response = Some(action);
        self.condvar.notify_one();
        true
    }

    /// Cancel the pending request (e.g. player disconnected).
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.condvar.notify_all();
    }

    /// Check if the request was cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Reset for next action request
    pub fn reset(&self) {
        let mut response = self.response.lock().unwrap();
        *response = None;
        self.cancelled.store(false, Ordering::Relaxed);
    }
}

impl Default for ActionRequestState {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for pending action requests from the game engine.
/// This allows the async WebSocket layer to submit responses.
pub type ActionRequestRegistry = Arc<tokio::sync::Mutex<std::collections::HashMap<PlayerId, Arc<ActionRequestState>>>>;

/// Starts the game when both players have submitted decks.
pub async fn start_game(
    room: Arc<Mutex<Room>>,
    rules: GameRules,
) -> Result<()> {
    let mut room_guard = room.lock().await;

    // Wait for both decks with timeout
    let mut attempts = 0;
    while room_guard.decks[0].is_none() || room_guard.decks[1].is_none() {
        drop(room_guard);
        tokio::time::sleep(Duration::from_millis(100)).await;
        room_guard = room.lock().await;

        attempts += 1;
        if attempts > 300 { // 30 seconds timeout
            bail!("Timeout waiting for deck submission");
        }
    }

    let deck1 = room_guard.decks[0].clone().unwrap();
    let deck2 = room_guard.decks[1].clone().unwrap();
    let broadcast_tx = room_guard.broadcast_tx.clone();

    room_guard.state = crate::room::RoomState::Playing;
    drop(room_guard);

    info!(
        "Starting game in room: Player1 has {} cards, Player2 has {} cards",
        deck1.len(),
        deck2.len()
    );

    // Load card definitions
    // Try to find scripts directory relative to manifest or current dir
    let scripts_path = find_scripts_path();
    debug!("Looking for scripts at: {:?}", scripts_path);

    // Build script index by recursively scanning JSON files
    let mut index_entries = std::collections::HashMap::new();
    fn scan_dir_recursive(dir: &std::path::Path, entries: &mut std::collections::HashMap<CardId, std::path::PathBuf>) {
        if let Ok(read_dir) = std::fs::read_dir(dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir_recursive(&path, entries);
                } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(card_id) = stem.parse::<CardId>() {
                            entries.insert(card_id, path);
                        }
                    }
                }
            }
        }
    }
    scan_dir_recursive(&scripts_path, &mut index_entries);
    info!("Indexed {} card scripts from {:?}", index_entries.len(), scripts_path);

    if index_entries.is_empty() {
        bail!("No card scripts found in {:?}", scripts_path);
    }

    // Load card registry using our manually built index
    let registry = match load_registry_for_game(&index_entries, &deck1, &deck2) {
        Ok(r) => {
            info!("Loaded {} card definitions", r.len());
            r
        }
        Err(e) => {
            error!("Failed to load card registry: {}", e);
            bail!("Failed to load card registry: {}", e);
        }
    };

    // Validate decks
    if let Err(e) = validate_deck(&deck1, &rules, &registry) {
        error!("Player 1 deck invalid: {}", e);
        bail!("Player 1 deck invalid: {}", e);
    }

    if let Err(e) = validate_deck(&deck2, &rules, &registry) {
        error!("Player 2 deck invalid: {}", e);
        bail!("Player 2 deck invalid: {}", e);
    }

    let mut state = GameState::new(rules, rand::random());

    // Populate decks with card instances
    let mut next_id = 1u32;
    for card_id in &deck1 {
        let base_attack = registry
            .get(card_id)
            .and_then(|def| def.attack.map(|a| a as i32));
        state.players[0].zones.deck.push(CardInstance::new(
            InstanceId(next_id),
            card_id.clone(),
            base_attack,
        ));
        next_id += 1;
    }

    for card_id in &deck2 {
        let base_attack = registry
            .get(card_id)
            .and_then(|def| def.attack.map(|a| a as i32));
        state.players[1].zones.deck.push(CardInstance::new(
            InstanceId(next_id),
            card_id.clone(),
            base_attack,
        ));
        next_id += 1;
    }

    // Create action request states for sync/async bridge
    let action_state1 = Arc::new(ActionRequestState::new());
    let action_state2 = Arc::new(ActionRequestState::new());

    // Store action states in room so WebSocket layer can submit responses
    {
        let mut room_guard = room.lock().await;
        room_guard.action_states[0] = Some(action_state1.clone());
        room_guard.action_states[1] = Some(action_state2.clone());
    }

    // Send initial game state to both players using room's broadcast
    let visible_p1 = build_visible_state(&state, PlayerId::Player1, &[]);
    let visible_p2 = build_visible_state(&state, PlayerId::Player2, &[]);

    {
        let room_guard = room.lock().await;
        room_guard.broadcast_message(RoomMessage::GameStarted);
        room_guard.broadcast_message(RoomMessage::StateUpdate {
            for_player: PlayerId::Player1,
            state: visible_p1,
        });
        room_guard.broadcast_message(RoomMessage::StateUpdate {
            for_player: PlayerId::Player2,
            state: visible_p2,
        });
    }

    info!("Game started, initial state sent to both players");

    // Create network-aware PhaseClients
    let client1 = NetworkPhaseClient::new(
        PlayerId::Player1,
        broadcast_tx.clone(),
        action_state1,
    );
    let client2 = NetworkPhaseClient::new(
        PlayerId::Player2,
        broadcast_tx.clone(),
        action_state2,
    );

    // Run game engine in blocking task
    let result = tokio::task::spawn_blocking(move || {
        let engine = GameEngine::new(state, registry, Box::new(client1), Box::new(client2));
        engine.run()
    })
    .await?;

    // Game over
    info!(
        "Game finished: winner={:?}, reason={:?}",
        result.winner, result.reason
    );

    let _ = broadcast_tx.send(RoomMessage::GameOver {
        winner: result.winner,
        reason: format!("{:?}", result.reason),
    });

    // Update room state
    let mut room_guard = room.lock().await;
    room_guard.state = crate::room::RoomState::Finished;

    Ok(())
}

/// PhaseClient implementation that bridges network requests to the game engine.
pub struct NetworkPhaseClient {
    player_id: PlayerId,
    broadcast_tx: mpsc::UnboundedSender<RoomMessage>,
    action_state: Arc<ActionRequestState>,
}

impl NetworkPhaseClient {
    pub fn new(
        player_id: PlayerId,
        broadcast_tx: mpsc::UnboundedSender<RoomMessage>,
        action_state: Arc<ActionRequestState>,
    ) -> Self {
        Self {
            player_id,
            broadcast_tx,
            action_state,
        }
    }
}

impl PhaseClient for NetworkPhaseClient {
    fn choose_action(
        &self,
        available: &[PhaseAction],
        timeout: Duration,
    ) -> Option<PhaseAction> {
        // Reset state for new request
        self.action_state.reset();

        // Notify the player that they need to choose an action
        let action_options: Vec<ActionOption> = available
            .iter()
            .map(|a| match a {
                PhaseAction::Pass => ActionOption::Pass,
                PhaseAction::Surrender => ActionOption::Surrender,
                PhaseAction::PlayCard {
                    instance_id,
                    target_zone,
                    ..
                } => {
                    let zone = match target_zone {
                        Zone::Front(slot) => ActionZone::Front { slot: *slot },
                        Zone::Back(slot) => ActionZone::Back { slot: *slot },
                        _ => ActionZone::Front { slot: 0 },
                    };
                    ActionOption::PlayCard {
                        instance_id: instance_id.0,
                        target_zone: zone,
                    }
                }
                PhaseAction::DeclareAttack {
                    attacker,
                    target_slot,
                } => {
                    let target = match target_slot {
                        Some(slot) => ActionTarget::Slot { slot_index: *slot },
                        None => ActionTarget::Direct,
                    };
                    ActionOption::DeclareAttack {
                        attacker: attacker.0,
                        target,
                    }
                }
                PhaseAction::ActivateEffect {
                    instance_id,
                    effect_key,
                    ..
                } => ActionOption::ActivateEffect {
                    instance_id: instance_id.0,
                    effect_key: effect_key.0.clone(),
                },
            })
            .collect();

        let _ = self.broadcast_tx.send(RoomMessage::ActionRequested {
            player_id: self.player_id,
            available_actions: action_options,
            timeout_secs: timeout.as_secs(),
        });

        // Block waiting for response from the async side
        match self.action_state.wait_response(timeout) {
            Some(PlayerAction::PhaseAction(phase_action)) => {
                debug!("Player {:?} chose action: {:?}", self.player_id, phase_action);
                Some(phase_action)
            }
            Some(PlayerAction::RecoverySelection(_)) => {
                warn!("Player {:?} sent recovery selection instead of phase action", self.player_id);
                Some(PhaseAction::Pass) // Fallback
            }
            Some(PlayerAction::ChainResponse(_)) => {
                warn!("Player {:?} sent chain response instead of phase action", self.player_id);
                Some(PhaseAction::Pass) // Fallback
            }
            None => {
                if self.action_state.is_cancelled() {
                    warn!("Action request cancelled for {:?} (disconnected)", self.player_id);
                    Some(PhaseAction::Surrender)
                } else {
                    warn!("Player {:?} action timeout, using Pass", self.player_id);
                    Some(PhaseAction::Pass) // Timeout fallback
                }
            }
        }
    }

    fn choose_recovery_cards(
        &self,
        options: &[RecoveryCardOption],
        count: usize,
        timeout: Duration,
    ) -> Option<Vec<InstanceId>> {
        // Reset state for new request
        self.action_state.reset();

        // Notify the player that they need to choose recovery cards
        let options_data: Vec<RecoveryOption> = options
            .iter()
            .map(|o| RecoveryOption {
                instance_id: o.instance_id.0,
                definition_id: o.definition_id.0.clone(),
            })
            .collect();

        let _ = self.broadcast_tx.send(RoomMessage::RecoveryRequested {
            player_id: self.player_id,
            count,
            options: options_data,
        });

        // Block waiting for response
        match self.action_state.wait_response(timeout) {
            Some(PlayerAction::RecoverySelection(instance_ids)) => {
                debug!("Player {:?} recovery selection: {:?}", self.player_id, instance_ids);
                Some(instance_ids)
            }
            Some(PlayerAction::PhaseAction(_)) => {
                warn!("Player {:?} sent phase action instead of recovery", self.player_id);
                Some(vec![]) // Fallback
            }
            Some(PlayerAction::ChainResponse(_)) => {
                warn!("Player {:?} sent chain response instead of recovery", self.player_id);
                Some(vec![]) // Fallback
            }
            None => {
                warn!("Player {:?} recovery timeout", self.player_id);
                Some(vec![]) // Timeout fallback
            }
        }
    }

    fn choose_chain_action(
        &self,
        chain_stack: &card_core::state::ChainStack,
        available: &[ChainActivateOption],
        timeout: Duration,
    ) -> Option<card_core::engine::phase::ChainResponse> {
        // Reset state for new request
        self.action_state.reset();

        // Build chain effect options for the player
        let effect_options: Vec<ChainEffectOption> = available
            .iter()
            .map(|opt| ChainEffectOption {
                instance_id: opt.instance_id.0,
                effect_key: opt.effect_key.0.clone(),
                definition_id: String::new(), // client can look up from card cache
            })
            .collect();

        let _ = self.broadcast_tx.send(RoomMessage::ChainActionRequested {
            player_id: self.player_id,
            chain_size: chain_stack.links.len(),
            available_effects: effect_options,
            timeout_secs: timeout.as_secs(),
        });

        // Block waiting for response
        match self.action_state.wait_response(timeout) {
            Some(PlayerAction::ChainResponse(response)) => {
                debug!("Player {:?} chain response: {:?}", self.player_id, response);
                Some(response)
            }
            Some(PlayerAction::PhaseAction(_)) | Some(PlayerAction::RecoverySelection(_)) => {
                warn!("Player {:?} sent wrong action type instead of chain response", self.player_id);
                Some(card_core::engine::phase::ChainResponse::ChainPass)
            }
            None => {
                if self.action_state.is_cancelled() {
                    warn!("Chain action cancelled for {:?} (disconnected)", self.player_id);
                    None // treat as disconnect — engine will handle
                } else {
                    warn!("Player {:?} chain action timeout, using Pass", self.player_id);
                    Some(card_core::engine::phase::ChainResponse::ChainPass)
                }
            }
        }
    }

    fn on_events(&self, events: &[CoreGameEvent], state: &GameState) {
        let visible = build_visible_state(state, self.player_id, events);
        let _ = self.broadcast_tx.send(RoomMessage::StateUpdate {
            for_player: self.player_id,
            state: visible,
        });
    }
}

/// Find scripts directory by checking multiple locations.
fn find_scripts_path() -> std::path::PathBuf {
    // Check current directory first
    let scripts_json = std::path::PathBuf::from("scripts_json");
    if scripts_json.exists() && scripts_json.read_dir().map(|mut d| d.next().is_some()).unwrap_or(false) {
        return scripts_json;
    }

    let scripts = std::path::PathBuf::from("scripts");
    if scripts.exists() {
        return scripts;
    }

    // Check relative to manifest dir (for tests)
    // Need to go up 2 levels: crates/card-server -> crates -> workspace root
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = std::path::PathBuf::from(&manifest_dir);
        if let Some(crates_dir) = manifest_path.parent() {
            if let Some(workspace_root) = crates_dir.parent() {
                let scripts_json = workspace_root.join("scripts_json");
                if scripts_json.exists() {
                    return scripts_json;
                }
                let scripts = workspace_root.join("scripts");
                if scripts.exists() {
                    return scripts;
                }
            }
        }
    }

    // Default fallback
    std::path::PathBuf::from("scripts_json")
}

/// Build visible game state for a specific player (with information hiding).
fn build_visible_state(state: &GameState, for_player: PlayerId, events: &[CoreGameEvent]) -> VisibleGameState {
    let my_idx = match for_player {
        PlayerId::Player1 => 0,
        PlayerId::Player2 => 1,
    };
    let opp_idx = 1 - my_idx;

    let me = &state.players[my_idx];
    let opponent = &state.players[opp_idx];

    fn card_to_visible(card: &card_core::state::CardInstance) -> crate::room::CardVisibleInfo {
        crate::room::CardVisibleInfo {
            instance_id: card.instance_id,
            definition_id: card.definition_id.clone(),
            current_attack: card.current_attack,
        }
    }

    let my_state = crate::room::PlayerVisibleState {
        hp: me.hp,
        real_point: me.real_point,
        deck_count: me.zones.deck.len(),
        hand: me.zones.hand.iter().map(card_to_visible).collect(),
        front: me.zones.front.clone().map(|opt| opt.as_ref().map(card_to_visible)),
        back: me.zones.back.clone().map(|opt| opt.as_ref().map(card_to_visible)),
        cost_zone: me.zones.cost_zone.iter().map(card_to_visible).collect(),
        grave: me.zones.grave.iter().map(card_to_visible).collect(),
    };

    // Opponent state hides hand details - only show count
    let opponent_state = crate::room::OpponentVisibleState {
        hp: opponent.hp,
        real_point: opponent.real_point,
        deck_count: opponent.zones.deck.len(),
        hand_count: opponent.zones.hand.len(), // Only count, not contents!
        front: opponent.zones.front.clone().map(|opt| opt.as_ref().map(card_to_visible)),
        back: opponent.zones.back.clone().map(|opt| opt.as_ref().map(card_to_visible)),
        cost_zone: opponent.zones.cost_zone.iter().map(card_to_visible).collect(),
        grave: opponent.zones.grave.iter().map(card_to_visible).collect(),
    };

    let phase = match state.phase {
        card_core::state::Phase::TurnStart => card_protocol::message::Phase::TurnStart,
        card_core::state::Phase::Draw => card_protocol::message::Phase::Draw,
        card_core::state::Phase::Recovery => card_protocol::message::Phase::Recovery,
        card_core::state::Phase::Main1 => card_protocol::message::Phase::Main1,
        card_core::state::Phase::Battle => card_protocol::message::Phase::Battle,
        card_core::state::Phase::Main2 => card_protocol::message::Phase::Main2,
        card_core::state::Phase::TurnEnd => card_protocol::message::Phase::TurnEnd,
    };

    let recent_events: Vec<serde_json::Value> = events
        .iter()
        .filter_map(|e| serde_json::to_value(e).ok())
        .collect();

    VisibleGameState {
        turn_number: state.turn_number,
        current_phase: phase,
        current_player: state.turn_player,
        your_state: my_state,
        opponent_state: opponent_state,
        recent_events,
    }
}

/// Load card registry for a game using a pre-built script index.
fn load_registry_for_game(
    index_entries: &std::collections::HashMap<CardId, std::path::PathBuf>,
    deck1: &[CardId],
    deck2: &[CardId],
) -> Result<CardRegistryImpl> {
    let mut to_load: std::collections::HashSet<CardId> = deck1.iter().cloned().collect();
    to_load.extend(deck2.iter().cloned());

    let mut registry = CardRegistryImpl::new();

    for card_id in &to_load {
        let path = index_entries
            .get(card_id)
            .ok_or_else(|| anyhow!("Card not found: {}", card_id))?;

        let definition = load_card_from_json(path)
            .map_err(|e| anyhow!("Failed to load card {}: {}", card_id, e))?;

        registry.insert(definition);
    }

    Ok(registry)
}

/// Load a card definition from a JSON file.
fn load_card_from_json(path: &std::path::Path) -> Result<CardDefinition> {
    use card_core::types::{CardType, Category, EffectKey, ItemKind, Property, StrategyKind};
    use serde_json::Value;

    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("Cannot read {:?}: {}", path, e))?;

    let json: Value = serde_json::from_str(&content)
        .map_err(|e| anyhow!("JSON parse error in {:?}: {}", path, e))?;

    let id_str = json
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing 'id' field"))?;

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing 'name' field"))?;

    let card_type_str = json
        .get("card_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing 'card_type' field"))?;

    let card_type = parse_card_type(card_type_str)?;

    let property_str = json
        .get("property")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing 'property' field"))?;

    let property = parse_property(property_str)?;

    let category_str = json
        .get("category")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing 'category' field"))?;

    let category = parse_category(category_str)?;

    let cost = json
        .get("cost")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow!("missing or invalid 'cost' field"))? as u32;

    let mut def = CardDefinition::new(
        CardId::new(id_str),
        name.to_string(),
        card_type,
        property,
        category,
        cost,
    );

    // Optional fields
    if let Some(attack) = json.get("attack").and_then(|v| v.as_u64()) {
        def = def.with_attack(attack as u32);
    }

    if let Some(strategy_kind) = json.get("strategy_kind").and_then(|v| v.as_str()) {
        def = def.with_strategy_kind(parse_strategy_kind(strategy_kind)?);
    }

    if let Some(item_kind) = json.get("item_kind").and_then(|v| v.as_str()) {
        def = def.with_item_kind(parse_item_kind(item_kind)?);
    }

    if let Some(tags_array) = json.get("tags").and_then(|v| v.as_array()) {
        let tags: Vec<String> = tags_array
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        def = def.with_tags(tags);
    }

    // Effects
    if let Some(effects_obj) = json.get("effects").and_then(|v| v.as_object()) {
        for (key, value) in effects_obj {
            def = def.with_effect(EffectKey(key.clone()), value.clone());
        }
    }

    Ok(def)
}

fn parse_card_type(s: &str) -> Result<CardType> {
    use card_core::types::CardType;
    match s {
        "Character" => Ok(CardType::Character),
        "Strategy" => Ok(CardType::Strategy),
        "Item" => Ok(CardType::Item),
        "Legendary" => Ok(CardType::Legendary),
        _ => Err(anyhow!("unknown card_type '{}'", s)),
    }
}

fn parse_property(s: &str) -> Result<Property> {
    use card_core::types::Property;
    match s {
        "Rational" => Ok(Property::Rational),
        "Divine" => Ok(Property::Divine),
        "Spiritual" => Ok(Property::Spiritual),
        _ => Err(anyhow!("unknown property '{}'", s)),
    }
}

fn parse_category(s: &str) -> Result<Category> {
    use card_core::types::Category;
    match s {
        "Math" => Ok(Category::Math),
        "Science" => Ok(Category::Science),
        "Literature" => Ok(Category::Literature),
        "Philosophy" => Ok(Category::Philosophy),
        "Mystery" => Ok(Category::Mystery),
        _ => Err(anyhow!("unknown category '{}'", s)),
    }
}

fn parse_strategy_kind(s: &str) -> Result<StrategyKind> {
    use card_core::types::StrategyKind;
    match s {
        "Normal" => Ok(StrategyKind::Normal),
        "Trick" => Ok(StrategyKind::Trick),
        "Instant" => Ok(StrategyKind::Instant),
        _ => Err(anyhow!("unknown strategy_kind '{}'", s)),
    }
}

fn parse_item_kind(s: &str) -> Result<ItemKind> {
    use card_core::types::ItemKind;
    match s {
        "Normal" => Ok(ItemKind::Normal),
        "Persistent" => Ok(ItemKind::Persistent),
        _ => Err(anyhow!("unknown item_kind '{}'", s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use card_core::state::{CardInstance, GameState};
    use card_core::rules::GameRules;
    use card_core::types::{CardId, InstanceId};

    #[test]
    fn test_network_phase_client_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let action_state = Arc::new(ActionRequestState::new());

        let client = NetworkPhaseClient::new(
            PlayerId::Player1,
            tx,
            action_state,
        );

        assert!(matches!(client.player_id, PlayerId::Player1));
    }

    #[test]
    fn network_phase_client_on_events_broadcasts_state_update() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let action_state = Arc::new(ActionRequestState::new());
        let client = NetworkPhaseClient::new(
            PlayerId::Player1,
            tx,
            action_state,
        );

        let mut state = GameState::new(GameRules::default(), 42);
        // Populate P1 hand with 2 cards
        state.players[0].zones.hand.push(CardInstance::new(
            InstanceId(1),
            CardId::new("S000-C-001"),
            Some(100),
        ));
        state.players[0].zones.hand.push(CardInstance::new(
            InstanceId(2),
            CardId::new("S000-C-002"),
            Some(200),
        ));
        // Populate P2 hand with 1 card
        state.players[1].zones.hand.push(CardInstance::new(
            InstanceId(3),
            CardId::new("S000-C-003"),
            Some(300),
        ));

        let events = vec![CoreGameEvent::PhaseChanged {
            new_phase: card_core::state::Phase::Main1,
        }];
        client.on_events(&events, &state);

        // Now only sends StateUpdate for its own player (Player1)
        let msg = rx.try_recv().expect("Expected StateUpdate for P1");
        match msg {
            RoomMessage::StateUpdate { for_player, state } => {
                assert_eq!(for_player, PlayerId::Player1);
                assert_eq!(state.your_state.hand.len(), 2);
                assert_eq!(state.opponent_state.hand_count, 1);
            }
            _ => panic!("Expected StateUpdate for P1, got {:?}", msg),
        }

        // No more messages (no longer sends P2's perspective)
        assert!(rx.try_recv().is_err());
    }
}
