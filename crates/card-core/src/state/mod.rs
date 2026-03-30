//! Game state data model.
//!
//! Pure data structures representing the complete state of a game in progress.
//! No game logic — only storage, querying, and serialization.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::effect::AppliedModifier;
use crate::rules::{CardRegistry, GameRules};
use crate::types::{CardDefinition, CardId, CardType, EffectKey, InstanceId, PlayerId, Zone};

pub mod events;
pub use events::CoreGameEvent;

// ─── Phase ───────────────────────────────────────────────────────────────────

/// Game phase within a turn.
///
/// Matches the spec: TurnStart → Draw → Recovery → Main1 → Battle → Main2 → TurnEnd.
/// Defined independently of card-protocol (card-core does not depend on it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    TurnStart,
    Draw,
    Recovery,
    Main1,
    Battle,
    Main2,
    TurnEnd,
}

// ─── BattleStep ──────────────────────────────────────────────────────────────

/// Sub-steps within the Battle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleStep {
    /// 宣言攻击
    Declaration,
    /// 伤害计算
    DamageCalc,
    /// 战斗结算
    Resolution,
}

// ─── CardInstance ─────────────────────────────────────────────────────────────

/// A card instance within a game session.
///
/// Represents a concrete card on the field/in hand/etc., as opposed to the
/// static [`CardDefinition`] it was created from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardInstance {
    /// Unique instance ID within this game session.
    pub instance_id: InstanceId,
    /// The card definition this instance was created from.
    pub definition_id: CardId,
    /// Current attack power (after modifiers). `None` for non-character cards.
    pub current_attack: Option<i32>,
    /// Active modifiers applied to this card.
    pub modifiers: Vec<AppliedModifier>,
    /// Whether this card has already attacked this turn.
    pub attacked_this_turn: bool,
    /// The turn number when this card was summoned (for summoning sickness).
    pub turn_summoned: u32,
}

impl CardInstance {
    pub fn new(instance_id: InstanceId, definition_id: CardId, base_attack: Option<i32>) -> Self {
        Self {
            instance_id,
            definition_id,
            current_attack: base_attack,
            modifiers: Vec::new(),
            attacked_this_turn: false,
            turn_summoned: 0,
        }
    }
}

// ─── PlayerZones ─────────────────────────────────────────────────────────────

/// All zones belonging to a single player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerZones {
    pub deck: Vec<CardInstance>,
    pub hand: Vec<CardInstance>,
    pub front: [Option<CardInstance>; 5],
    pub back: [Option<CardInstance>; 5],
    pub cost_zone: Vec<CardInstance>,
    pub grave: Vec<CardInstance>,
}

impl PlayerZones {
    pub fn new() -> Self {
        Self {
            deck: Vec::new(),
            hand: Vec::new(),
            front: [None, None, None, None, None],
            back: [None, None, None, None, None],
            cost_zone: Vec::new(),
            grave: Vec::new(),
        }
    }

    /// Iterate over all cards across every zone.
    pub fn all_cards(&self) -> impl Iterator<Item = &CardInstance> {
        self.deck
            .iter()
            .chain(self.hand.iter())
            .chain(self.front.iter().flatten())
            .chain(self.back.iter().flatten())
            .chain(self.cost_zone.iter())
            .chain(self.grave.iter())
    }

    /// Find a card by its instance ID across all zones.
    pub fn card_by_instance_id(&self, id: InstanceId) -> Option<&CardInstance> {
        self.all_cards().find(|c| c.instance_id == id)
    }

    /// Remove and return a card by instance ID, along with the zone it was in.
    pub fn remove_card(&mut self, id: InstanceId) -> Option<(CardInstance, Zone)> {
        macro_rules! try_remove_vec {
            ($vec:expr, $zone:expr) => {
                if let Some(pos) = $vec.iter().position(|c| c.instance_id == id) {
                    return Some(($vec.remove(pos), $zone));
                }
            };
        }
        try_remove_vec!(self.deck, Zone::Deck);
        try_remove_vec!(self.hand, Zone::Hand);
        try_remove_vec!(self.cost_zone, Zone::CostZone);
        try_remove_vec!(self.grave, Zone::Grave);

        for i in 0..5 {
            if self.front[i].as_ref().is_some_and(|c| c.instance_id == id) {
                return Some((self.front[i].take().unwrap(), Zone::Front(i)));
            }
        }
        for i in 0..5 {
            if self.back[i].as_ref().is_some_and(|c| c.instance_id == id) {
                return Some((self.back[i].take().unwrap(), Zone::Back(i)));
            }
        }

        None
    }

    /// Add a card to the specified zone.
    ///
    /// For `Front(i)` and `Back(i)`, the slot must be empty.
    /// Returns `Err` if the slot is occupied or the index is out of range.
    pub fn add_card_to_zone(&mut self, card: CardInstance, zone: Zone) -> Result<(), String> {
        match zone {
            Zone::Deck => self.deck.push(card),
            Zone::Hand => self.hand.push(card),
            Zone::CostZone => self.cost_zone.push(card),
            Zone::Grave => self.grave.push(card),
            Zone::Front(i) => {
                if i >= 5 {
                    return Err(format!("Front slot index {} out of range (0..5)", i));
                }
                if self.front[i].is_some() {
                    return Err(format!("Front slot {} is already occupied", i));
                }
                self.front[i] = Some(card);
            }
            Zone::Back(i) => {
                if i >= 5 {
                    return Err(format!("Back slot index {} out of range (0..5)", i));
                }
                if self.back[i].is_some() {
                    return Err(format!("Back slot {} is already occupied", i));
                }
                self.back[i] = Some(card);
            }
        }
        Ok(())
    }

    /// Count cards in a zone by name.
    pub fn zone_count(&self, zone_type: &str) -> usize {
        match zone_type {
            "deck" => self.deck.len(),
            "hand" => self.hand.len(),
            "front" => self.front.iter().flatten().count(),
            "back" => self.back.iter().flatten().count(),
            "cost_zone" => self.cost_zone.len(),
            "grave" => self.grave.len(),
            _ => 0,
        }
    }

    /// Check whether a zone is at capacity.
    pub fn is_zone_full(&self, zone: Zone) -> bool {
        match zone {
            Zone::Deck | Zone::Hand | Zone::CostZone | Zone::Grave => false,
            Zone::Front(i) => i < 5 && self.front[i].is_some(),
            Zone::Back(i) => i < 5 && self.back[i].is_some(),
        }
    }
}

impl Default for PlayerZones {
    fn default() -> Self {
        Self::new()
    }
}

// ─── PlayerState ─────────────────────────────────────────────────────────────

/// Complete state for a single player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: PlayerId,
    pub zones: PlayerZones,
    pub hp: u8,
    pub real_point: u8,
}

impl PlayerState {
    pub fn new(id: PlayerId, initial_hp: u8) -> Self {
        Self {
            id,
            zones: PlayerZones::new(),
            hp: initial_hp,
            real_point: 0,
        }
    }
}

// ─── ChainLink / ChainStack ─────────────────────────────────────────────────

/// A single link in the effect chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainLink {
    pub source_instance: InstanceId,
    pub effect_key: EffectKey,
    pub activating_player: PlayerId,
}

/// FILO chain stack for effect resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainStack {
    pub links: Vec<ChainLink>,
    /// Whether the chain is currently open for responses.
    pub is_open: bool,
    /// The last player who passed (used for chain resolution timing).
    pub last_passed: Option<PlayerId>,
}

impl ChainStack {
    pub fn new() -> Self {
        Self {
            links: Vec::new(),
            is_open: false,
            last_passed: None,
        }
    }

    pub fn push(&mut self, link: ChainLink) {
        self.links.push(link);
    }

    pub fn pop(&mut self) -> Option<ChainLink> {
        self.links.pop()
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

impl Default for ChainStack {
    fn default() -> Self {
        Self::new()
    }
}

// ─── CardRegistryImpl ────────────────────────────────────────────────────────

/// Concrete HashMap-backed card registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardRegistryImpl {
    cards: HashMap<CardId, CardDefinition>,
}

impl CardRegistryImpl {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
        }
    }

    pub fn insert(&mut self, def: CardDefinition) {
        self.cards.insert(def.id.clone(), def);
    }

    pub fn get(&self, id: &CardId) -> Option<&CardDefinition> {
        self.cards.get(id)
    }

    pub fn contains(&self, id: &CardId) -> bool {
        self.cards.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&CardId, &CardDefinition)> {
        self.cards.iter()
    }
}

impl Default for CardRegistryImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl CardRegistry for CardRegistryImpl {
    fn contains_card(&self, card_id: &CardId) -> bool {
        self.cards.contains_key(card_id)
    }

    fn get_card_type(&self, card_id: &CardId) -> Option<char> {
        self.cards.get(card_id).map(|def| match def.card_type {
            CardType::Character => 'C',
            CardType::Strategy => 'S',
            CardType::Item => 'I',
            CardType::Legendary => 'L',
        })
    }
}

// ─── GameState ───────────────────────────────────────────────────────────────

/// Complete game state — the single source of truth for a match in progress.
///
/// Supports `Serialize`/`Deserialize` for save/restore and `PartialEq` for
/// replay verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// Player states: `[Player1, Player2]`.
    pub players: [PlayerState; 2],
    /// The player whose turn it currently is.
    pub turn_player: PlayerId,
    /// Current game phase.
    pub phase: Phase,
    /// Current turn number (starts at 1).
    pub turn_number: u32,
    /// Current battle sub-step (only meaningful during `Phase::Battle`).
    pub battle_step: Option<BattleStep>,
    /// Effect chain stack.
    pub chain: ChainStack,
    /// Rules for this match.
    pub rules: GameRules,
    /// RNG seed for deterministic replay.
    pub rng_seed: u64,
    /// Auto-incrementing counter for generating unique [`InstanceId`]s.
    pub instance_counter: u32,
    /// 本回合已激活过的效果（用于 OncePerTurn 限制）
    /// Key: (实例ID, 效果键名)
    pub activated_this_turn: HashSet<(InstanceId, EffectKey)>,
}

impl GameState {
    /// Create a new game state with default initial values.
    pub fn new(rules: GameRules, rng_seed: u64) -> Self {
        let initial_hp = rules.initial_hp;
        Self {
            players: [
                PlayerState::new(PlayerId::Player1, initial_hp),
                PlayerState::new(PlayerId::Player2, initial_hp),
            ],
            turn_player: PlayerId::Player1,
            phase: Phase::TurnStart,
            turn_number: 1,
            battle_step: None,
            chain: ChainStack::new(),
            rules,
            rng_seed,
            instance_counter: 0,
            activated_this_turn: HashSet::new(),
        }
    }

    /// Get the current turn player's state.
    pub fn current_player(&self) -> &PlayerState {
        self.player(self.turn_player)
    }

    /// Get the current turn player's state mutably.
    pub fn current_player_mut(&mut self) -> &mut PlayerState {
        self.player_mut(self.turn_player)
    }

    /// Get the opponent of the current turn player.
    pub fn opponent_player(&self) -> &PlayerState {
        self.player(self.turn_player.opponent())
    }

    /// Get the opponent of the current turn player mutably.
    pub fn opponent_player_mut(&mut self) -> &mut PlayerState {
        self.player_mut(self.turn_player.opponent())
    }

    /// Get a player's state by ID.
    pub fn player(&self, id: PlayerId) -> &PlayerState {
        match id {
            PlayerId::Player1 => &self.players[0],
            PlayerId::Player2 => &self.players[1],
        }
    }

    /// Get a player's state mutably by ID.
    pub fn player_mut(&mut self, id: PlayerId) -> &mut PlayerState {
        match id {
            PlayerId::Player1 => &mut self.players[0],
            PlayerId::Player2 => &mut self.players[1],
        }
    }

    /// Generate the next unique instance ID.
    pub fn next_instance_id(&mut self) -> InstanceId {
        self.instance_counter += 1;
        InstanceId(self.instance_counter)
    }

    /// Find a card across all players and zones by instance ID.
    ///
    /// Returns the card reference, owning player, and zone.
    pub fn get_card(&self, id: InstanceId) -> Option<(&CardInstance, PlayerId, Zone)> {
        for player_state in &self.players {
            for card in &player_state.zones.deck {
                if card.instance_id == id {
                    return Some((card, player_state.id, Zone::Deck));
                }
            }
            for card in &player_state.zones.hand {
                if card.instance_id == id {
                    return Some((card, player_state.id, Zone::Hand));
                }
            }
            for (i, slot) in player_state.zones.front.iter().enumerate() {
                if let Some(card) = slot
                    && card.instance_id == id
                {
                    return Some((card, player_state.id, Zone::Front(i)));
                }
            }
            for (i, slot) in player_state.zones.back.iter().enumerate() {
                if let Some(card) = slot
                    && card.instance_id == id
                {
                    return Some((card, player_state.id, Zone::Back(i)));
                }
            }
            for card in &player_state.zones.cost_zone {
                if card.instance_id == id {
                    return Some((card, player_state.id, Zone::CostZone));
                }
            }
            for card in &player_state.zones.grave {
                if card.instance_id == id {
                    return Some((card, player_state.id, Zone::Grave));
                }
            }
        }
        None
    }

    /// Move a card from its current location to a target zone for a target player.
    pub fn move_card(
        &mut self,
        id: InstanceId,
        to_player: PlayerId,
        to_zone: Zone,
    ) -> Result<(), String> {
        let mut card = None;
        for player_state in &mut self.players {
            if let Some((c, _from_zone)) = player_state.zones.remove_card(id) {
                card = Some(c);
                break;
            }
        }
        let card = card.ok_or_else(|| format!("Card with InstanceId({}) not found", id.0))?;

        self.player_mut(to_player)
            .zones
            .add_card_to_zone(card, to_zone)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn default_state() -> GameState {
        GameState::new(GameRules::default(), 42)
    }

    fn test_card(instance_id: u32) -> CardInstance {
        CardInstance::new(
            InstanceId(instance_id),
            CardId::new("S001-C-001"),
            Some(100),
        )
    }

    #[test]
    fn test_game_state_new() {
        let state = default_state();
        assert_eq!(state.players[0].hp, 5);
        assert_eq!(state.players[1].hp, 5);
        assert_eq!(state.players[0].real_point, 0);
        assert_eq!(state.players[1].real_point, 0);
        assert_eq!(state.phase, Phase::TurnStart);
        assert_eq!(state.turn_number, 1);
        assert_eq!(state.turn_player, PlayerId::Player1);
        assert!(state.chain.is_empty());
        assert_eq!(state.instance_counter, 0);
    }

    #[test]
    fn test_game_state_serialize_roundtrip() {
        let mut state = default_state();
        let card = test_card(1);
        state.players[0]
            .zones
            .add_card_to_zone(card, Zone::Hand)
            .unwrap();

        let json = serde_json::to_string(&state).expect("serialize");
        let deserialized: GameState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_game_state_eq() {
        let state1 = default_state();
        let state2 = default_state();
        assert_eq!(state1, state2);
    }

    #[test]
    fn test_player_zones_add_remove() {
        let mut zones = PlayerZones::new();
        let card = test_card(10);

        zones.add_card_to_zone(card.clone(), Zone::Hand).unwrap();
        assert_eq!(zones.zone_count("hand"), 1);

        let (removed, zone) = zones.remove_card(InstanceId(10)).unwrap();
        assert_eq!(removed.instance_id, InstanceId(10));
        assert_eq!(zone, Zone::Hand);
        assert_eq!(zones.zone_count("hand"), 0);
    }

    #[test]
    fn test_player_zones_front_slot() {
        let mut zones = PlayerZones::new();
        let card = test_card(20);

        zones
            .add_card_to_zone(card.clone(), Zone::Front(2))
            .unwrap();
        assert_eq!(zones.zone_count("front"), 1);
        assert!(zones.is_zone_full(Zone::Front(2)));
        assert!(!zones.is_zone_full(Zone::Front(0)));

        let card2 = test_card(21);
        assert!(zones.add_card_to_zone(card2, Zone::Front(2)).is_err());

        let card3 = test_card(22);
        assert!(zones.add_card_to_zone(card3, Zone::Front(5)).is_err());
    }

    #[test]
    fn test_player_zones_card_by_instance_id() {
        let mut zones = PlayerZones::new();
        let card = test_card(30);
        zones.add_card_to_zone(card, Zone::CostZone).unwrap();

        assert!(zones.card_by_instance_id(InstanceId(30)).is_some());
        assert!(zones.card_by_instance_id(InstanceId(99)).is_none());
    }

    #[test]
    fn test_instance_id_increment() {
        let mut state = default_state();
        let id1 = state.next_instance_id();
        let id2 = state.next_instance_id();
        let id3 = state.next_instance_id();
        assert_eq!(id1, InstanceId(1));
        assert_eq!(id2, InstanceId(2));
        assert_eq!(id3, InstanceId(3));
    }

    #[test]
    fn test_chain_stack_push_pop() {
        let mut chain = ChainStack::new();
        assert!(chain.is_empty());

        let link1 = ChainLink {
            source_instance: InstanceId(1),
            effect_key: EffectKey("eff1".into()),
            activating_player: PlayerId::Player1,
        };
        let link2 = ChainLink {
            source_instance: InstanceId(2),
            effect_key: EffectKey("eff2".into()),
            activating_player: PlayerId::Player2,
        };

        chain.push(link1.clone());
        chain.push(link2.clone());
        assert!(!chain.is_empty());

        let popped = chain.pop().unwrap();
        assert_eq!(popped.source_instance, InstanceId(2));

        let popped = chain.pop().unwrap();
        assert_eq!(popped.source_instance, InstanceId(1));

        assert!(chain.is_empty());
        assert!(chain.pop().is_none());
    }

    #[test]
    fn test_move_card() {
        let mut state = default_state();
        let card = test_card(50);

        // Add to player1's hand
        state.players[0]
            .zones
            .add_card_to_zone(card, Zone::Hand)
            .unwrap();
        assert_eq!(state.players[0].zones.zone_count("hand"), 1);

        // Move to player1's grave
        state
            .move_card(InstanceId(50), PlayerId::Player1, Zone::Grave)
            .unwrap();
        assert_eq!(state.players[0].zones.zone_count("hand"), 0);
        assert_eq!(state.players[0].zones.zone_count("grave"), 1);

        // Verify the card is findable in the new location
        let (found, player, zone) = state.get_card(InstanceId(50)).unwrap();
        assert_eq!(found.instance_id, InstanceId(50));
        assert_eq!(player, PlayerId::Player1);
        assert_eq!(zone, Zone::Grave);
    }

    #[test]
    fn test_move_card_cross_player() {
        let mut state = default_state();
        let card = test_card(60);

        // Add to player1's front
        state.players[0]
            .zones
            .add_card_to_zone(card, Zone::Front(0))
            .unwrap();

        // Move to player2's grave
        state
            .move_card(InstanceId(60), PlayerId::Player2, Zone::Grave)
            .unwrap();
        assert_eq!(state.players[0].zones.zone_count("front"), 0);
        assert_eq!(state.players[1].zones.zone_count("grave"), 1);
    }

    #[test]
    fn test_move_card_not_found() {
        let mut state = default_state();
        let result = state.move_card(InstanceId(999), PlayerId::Player1, Zone::Hand);
        assert!(result.is_err());
    }

    #[test]
    fn test_current_opponent_player() {
        let state = default_state();
        assert_eq!(state.current_player().id, PlayerId::Player1);
        assert_eq!(state.opponent_player().id, PlayerId::Player2);
    }

    #[test]
    fn test_card_registry_impl() {
        let mut registry = CardRegistryImpl::new();
        assert!(registry.is_empty());

        let def = CardDefinition::new(
            CardId::new("S001-C-001"),
            "Test Character".into(),
            CardType::Character,
            crate::types::Property::Rational,
            crate::types::Category::Math,
            3,
        );
        registry.insert(def);

        assert_eq!(registry.len(), 1);
        assert!(registry.contains(&CardId::new("S001-C-001")));
        assert!(!registry.contains(&CardId::new("S001-C-999")));

        // CardRegistry trait
        assert!(registry.contains_card(&CardId::new("S001-C-001")));
        assert_eq!(
            registry.get_card_type(&CardId::new("S001-C-001")),
            Some('C')
        );
        assert_eq!(registry.get_card_type(&CardId::new("S001-C-999")), None);
    }

    #[test]
    fn test_all_cards_iterator() {
        let mut zones = PlayerZones::new();
        zones.add_card_to_zone(test_card(1), Zone::Deck).unwrap();
        zones.add_card_to_zone(test_card(2), Zone::Hand).unwrap();
        zones
            .add_card_to_zone(test_card(3), Zone::Front(0))
            .unwrap();
        zones.add_card_to_zone(test_card(4), Zone::Back(4)).unwrap();
        zones
            .add_card_to_zone(test_card(5), Zone::CostZone)
            .unwrap();
        zones.add_card_to_zone(test_card(6), Zone::Grave).unwrap();

        let all: Vec<_> = zones.all_cards().collect();
        assert_eq!(all.len(), 6);
    }
}
