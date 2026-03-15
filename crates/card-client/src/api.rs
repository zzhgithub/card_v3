//! Client API trait and visible game state types.
//!
//! The `ClientApi` trait abstracts over all client implementations (TUI, Bevy, Web, etc.).
//! It receives game events and `VisibleGameState`, and provides player decisions.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use card_core::state::{CardInstance, Phase, PlayerState};
use card_core::types::{CardId, CardType, InstanceId, PlayerId, TargetRef};
use card_protocol::message::{AvailableAction, Command, GameEvent};

// ─── Visible State Types ─────────────────────────────────────────────────────

/// Public information about a single card that the opponent can see.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardPublicInfo {
    pub instance_id: InstanceId,
    pub definition_id: CardId,
    pub card_type: CardType,
    pub current_attack: Option<i32>,
}

impl CardPublicInfo {
    pub fn from_instance(instance: &CardInstance) -> Self {
        Self {
            instance_id: instance.instance_id,
            definition_id: instance.definition_id.clone(),
            // TODO: Fill from card registry in real implementation
            card_type: CardType::Character,
            current_attack: instance.current_attack,
        }
    }
}

/// Opponent's publicly visible state (hand count only, no card details).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpponentView {
    /// Number of cards in opponent's hand (visible count, not contents).
    pub hand_count: usize,
    /// Number of cards in opponent's deck.
    pub deck_count: usize,
    /// Opponent's front field slots (publicly visible).
    pub front: [Option<CardPublicInfo>; 5],
    /// Opponent's back field slots (publicly visible).
    pub back: [Option<CardPublicInfo>; 5],
    /// Opponent's cost zone (count and public info).
    pub cost_zone: Vec<CardPublicInfo>,
    /// Opponent's graveyard.
    pub grave: Vec<CardPublicInfo>,
    /// Opponent's HP.
    pub hp: u8,
    /// Opponent's RealPoint.
    pub real_point: u8,
}

impl OpponentView {
    pub fn from_player_state(state: &PlayerState) -> Self {
        Self {
            hand_count: state.zones.hand.len(),
            deck_count: state.zones.deck.len(),
            front: std::array::from_fn(|i| {
                state.zones.front[i]
                    .as_ref()
                    .map(CardPublicInfo::from_instance)
            }),
            back: std::array::from_fn(|i| {
                state.zones.back[i]
                    .as_ref()
                    .map(CardPublicInfo::from_instance)
            }),
            cost_zone: state
                .zones
                .cost_zone
                .iter()
                .map(CardPublicInfo::from_instance)
                .collect(),
            grave: state
                .zones
                .grave
                .iter()
                .map(CardPublicInfo::from_instance)
                .collect(),
            hp: state.hp,
            real_point: state.real_point,
        }
    }
}

/// The game state visible to one player.
///
/// Contains the player's own full state and only the opponent's public information.
/// The opponent's hand contents are never revealed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibleGameState {
    /// This player's complete state.
    pub my_state: PlayerState,
    /// Opponent's publicly visible information.
    pub opponent_public: OpponentView,
    /// Current game phase.
    pub phase: Phase,
    /// Current turn number.
    pub turn_number: u32,
    /// Number of links currently in the chain stack.
    pub chain_size: usize,
    /// Whether it's this player's turn.
    pub is_my_turn: bool,
}

// ─── Target / Card Selection Types ───────────────────────────────────────────

/// A selectable target presented to the player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetOption {
    pub target: TargetRef,
    pub description: String,
}

/// A selectable card in hand presented to the player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardOption {
    pub instance_id: InstanceId,
    pub definition_id: CardId,
    pub description: String,
}

// ─── ClientApi Trait ─────────────────────────────────────────────────────────

/// Trait abstracting over all client implementations.
///
/// Implementations include TUI, AI, replay, and test clients.
/// All methods are `async` to support timeout-based interactions.
#[async_trait]
pub trait ClientApi: Send + Sync {
    /// Called when a game event occurs and the visible state updates.
    async fn on_event(&self, event: &GameEvent, state: &VisibleGameState);

    /// Ask the player to choose an action from available options.
    ///
    /// Returns `None` if the timeout expires before a choice is made.
    async fn choose_action(
        &self,
        available: &[AvailableAction],
        timeout: Duration,
    ) -> Option<Command>;

    /// Ask the player to select target(s).
    async fn choose_targets(
        &self,
        options: &[TargetOption],
        count: usize,
        timeout: Duration,
    ) -> Option<Vec<InstanceId>>;

    /// Ask the player to select card(s) from hand.
    async fn choose_cards(
        &self,
        options: &[CardOption],
        count: usize,
        timeout: Duration,
    ) -> Option<Vec<InstanceId>>;
}

// ─── Helper: GameState → VisibleGameState ────────────────────────────────────

/// Convert a full `GameState` into the view visible to one player.
///
/// This lives in card-client (not card-core) to avoid a circular dependency,
/// since `VisibleGameState` is defined here.
pub fn game_state_to_visible(
    state: &card_core::state::GameState,
    for_player: PlayerId,
) -> VisibleGameState {
    let my_state = state.player(for_player).clone();
    let opponent = state.player(for_player.opponent());
    VisibleGameState {
        my_state,
        opponent_public: OpponentView::from_player_state(opponent),
        phase: state.phase,
        turn_number: state.turn_number,
        chain_size: state.chain.links.len(),
        is_my_turn: state.turn_player == for_player,
    }
}

// ─── TestClient ──────────────────────────────────────────────────────────────

/// A test client that returns pre-programmed commands in order.
///
/// Used for deterministic testing of the game engine.
pub struct TestClient {
    commands: Mutex<VecDeque<Command>>,
    events: Mutex<Vec<GameEvent>>,
}

impl TestClient {
    /// Create a `TestClient` with a pre-programmed list of commands.
    pub fn new(commands: Vec<Command>) -> Self {
        Self {
            commands: Mutex::new(commands.into()),
            events: Mutex::new(Vec::new()),
        }
    }

    /// Return all events received so far.
    pub fn events(&self) -> Vec<GameEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Check if the command queue is exhausted.
    pub fn is_done(&self) -> bool {
        self.commands.lock().unwrap().is_empty()
    }
}

#[async_trait]
impl ClientApi for TestClient {
    async fn on_event(&self, event: &GameEvent, _state: &VisibleGameState) {
        self.events.lock().unwrap().push(event.clone());
    }

    async fn choose_action(
        &self,
        _available: &[AvailableAction],
        _timeout: Duration,
    ) -> Option<Command> {
        self.commands.lock().unwrap().pop_front()
    }

    async fn choose_targets(
        &self,
        _options: &[TargetOption],
        _count: usize,
        _timeout: Duration,
    ) -> Option<Vec<InstanceId>> {
        Some(Vec::new())
    }

    async fn choose_cards(
        &self,
        _options: &[CardOption],
        _count: usize,
        _timeout: Duration,
    ) -> Option<Vec<InstanceId>> {
        Some(Vec::new())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use card_core::rules::GameRules;
    use card_core::state::{CardInstance, GameState};
    use card_core::types::{CardId, InstanceId, PlayerId, Zone};
    use card_protocol::message::Command;

    fn make_state() -> GameState {
        GameState::new(GameRules::default(), 42)
    }

    #[test]
    fn test_visible_state_hides_opponent_hand() {
        let mut state = make_state();
        // Add 3 cards to Player2's hand
        for i in 0u32..3 {
            let card = CardInstance::new(
                InstanceId(i + 100),
                CardId::new("S000-C-001"),
                Some(500),
            );
            state.players[1]
                .zones
                .add_card_to_zone(card, Zone::Hand)
                .unwrap();
        }

        // View from Player1
        let visible = game_state_to_visible(&state, PlayerId::Player1);

        // Can see opponent's hand COUNT
        assert_eq!(visible.opponent_public.hand_count, 3);

        // my_state is Player1's state
        assert_eq!(visible.my_state.id, PlayerId::Player1);
        assert_eq!(visible.my_state.zones.hand.len(), 0);
    }

    #[test]
    fn test_visible_state_my_state_is_complete() {
        let mut state = make_state();
        // Add cards to Player1's hand
        for i in 0u32..5 {
            let card = CardInstance::new(
                InstanceId(i + 200),
                CardId::new("S000-C-001"),
                Some(1000),
            );
            state.players[0]
                .zones
                .add_card_to_zone(card, Zone::Hand)
                .unwrap();
        }

        let visible = game_state_to_visible(&state, PlayerId::Player1);
        // Own hand is fully visible
        assert_eq!(visible.my_state.zones.hand.len(), 5);
    }

    #[test]
    fn test_visible_state_shows_opponent_field() {
        let mut state = make_state();
        // Place a card on Player2's front slot 0
        let card = CardInstance::new(InstanceId(300), CardId::new("S000-C-002"), Some(800));
        state.players[1]
            .zones
            .add_card_to_zone(card, Zone::Front(0))
            .unwrap();

        let visible = game_state_to_visible(&state, PlayerId::Player1);
        assert!(visible.opponent_public.front[0].is_some());
        assert_eq!(
            visible.opponent_public.front[0].as_ref().unwrap().instance_id,
            InstanceId(300)
        );
        assert!(visible.opponent_public.front[1].is_none());
    }

    #[test]
    fn test_visible_state_phase_and_turn() {
        let state = make_state();
        let visible = game_state_to_visible(&state, PlayerId::Player1);
        assert_eq!(visible.phase, Phase::TurnStart);
        assert_eq!(visible.turn_number, 1);
        assert!(visible.is_my_turn); // Player1 goes first
        assert_eq!(visible.chain_size, 0);
    }

    #[test]
    fn test_visible_state_is_not_my_turn() {
        let state = make_state();
        let visible = game_state_to_visible(&state, PlayerId::Player2);
        assert!(!visible.is_my_turn);
    }

    #[test]
    fn test_test_client_returns_commands_in_order() {
        let client = TestClient::new(vec![Command::ChainPass, Command::Surrender]);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let cmd1 = rt.block_on(client.choose_action(&[], Duration::from_secs(1)));
        let cmd2 = rt.block_on(client.choose_action(&[], Duration::from_secs(1)));
        let cmd3 = rt.block_on(client.choose_action(&[], Duration::from_secs(1)));

        assert!(matches!(cmd1, Some(Command::ChainPass)));
        assert!(matches!(cmd2, Some(Command::Surrender)));
        assert!(cmd3.is_none()); // queue exhausted
    }

    #[test]
    fn test_test_client_is_done() {
        let client = TestClient::new(vec![Command::ChainPass]);
        assert!(!client.is_done());

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(client.choose_action(&[], Duration::from_secs(1)));
        assert!(client.is_done());
    }

    #[test]
    fn test_test_client_records_events() {
        let client = TestClient::new(vec![]);
        let state = make_state();
        let visible = game_state_to_visible(&state, PlayerId::Player1);

        let event = GameEvent::PhaseChanged {
            new_phase: card_protocol::message::Phase::Draw,
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(client.on_event(&event, &visible));

        let events = client.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], GameEvent::PhaseChanged { .. }));
    }

    #[test]
    fn test_opponent_view_from_player_state() {
        let mut ps = PlayerState::new(PlayerId::Player2, 5);
        // Add some cards to various zones
        for i in 0u32..3 {
            let card = CardInstance::new(InstanceId(i), CardId::new("S000-C-001"), Some(100));
            ps.zones.add_card_to_zone(card, Zone::Hand).unwrap();
        }
        let grave_card = CardInstance::new(InstanceId(10), CardId::new("S000-C-002"), Some(200));
        ps.zones.add_card_to_zone(grave_card, Zone::Grave).unwrap();

        let view = OpponentView::from_player_state(&ps);
        assert_eq!(view.hand_count, 3);
        assert_eq!(view.grave.len(), 1);
        assert_eq!(view.hp, 5);
        assert_eq!(view.real_point, 0);
    }
}
