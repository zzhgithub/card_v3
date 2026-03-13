//! Game rules configuration and deck validation.
//!
//! This module defines the configurable game rules and provides validation
//! for deck construction according to the game specification.

use crate::types::CardId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Trait for accessing card registry information.
/// Implementations provide access to card definitions for validation.
pub trait CardRegistry {
    /// Check if a card ID exists in the registry.
    fn contains_card(&self, card_id: &CardId) -> bool;

    /// Get the card type for a given card ID (C/S/I/L).
    fn get_card_type(&self, card_id: &CardId) -> Option<char>;
}

/// Configurable game rules for a match.
///
/// All values are fully configurable per-match, allowing for custom rule variants.
/// Defaults follow the specification in plan/ReadMe.md.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameRules {
    /// Initial HP for each player (default: 5)
    pub initial_hp: u8,

    /// Maximum HP a player can have (default: 6)
    pub max_hp: u8,

    /// Maximum number of cards in hand (default: 20)
    pub max_hand: usize,

    /// Maximum number of cards in cost zone (default: 6)
    pub max_cost_zone: usize,

    /// Maximum real points a player can have (default: 6)
    pub max_real_point: u8,

    /// Valid deck size range (min, max) in cards (default: (40, 60))
    pub deck_size_range: (usize, usize),

    /// Maximum number of cards with the same ID in a deck (default: 3)
    pub max_same_card: usize,

    /// Maximum number of legendary cards in a deck (default: 5)
    pub max_legendary: usize,

    /// Initial hand size when game starts (default: 5)
    pub initial_hand_size: usize,

    /// Whether the first player draws a card in their first turn (default: false)
    pub first_player_draws: bool,

    /// Timeout for a single operation in seconds (default: 30s)
    pub operation_timeout: Duration,

    /// Total timeout for an entire game in seconds (default: 3600s = 1 hour)
    pub total_game_timeout: Duration,
}

impl Default for GameRules {
    fn default() -> Self {
        Self {
            initial_hp: 5,
            max_hp: 6,
            max_hand: 20,
            max_cost_zone: 6,
            max_real_point: 6,
            deck_size_range: (40, 60),
            max_same_card: 3,
            max_legendary: 5,
            initial_hand_size: 5,
            first_player_draws: false,
            operation_timeout: Duration::from_secs(30),
            total_game_timeout: Duration::from_secs(3600),
        }
    }
}

/// Validates a deck according to the game rules.
///
/// Checks:
/// - Deck size is within the configured range
/// - No card ID appears more than max_same_card times
/// - Legendary cards (type 'L') do not exceed max_legendary
/// - All card IDs exist in the registry
///
/// # Arguments
/// * `deck` - Slice of card IDs in the deck
/// * `rules` - Game rules to validate against
/// * `registry` - Card registry for checking card existence
///
/// # Returns
/// * `Ok(())` if deck is valid
/// * `Err(String)` with description of validation failure
pub fn validate_deck(
    deck: &[CardId],
    rules: &GameRules,
    registry: &dyn CardRegistry,
) -> Result<(), String> {
    // Check deck size
    let deck_size = deck.len();
    if deck_size < rules.deck_size_range.0 {
        return Err(format!(
            "Deck size {} is below minimum {}",
            deck_size, rules.deck_size_range.0
        ));
    }
    if deck_size > rules.deck_size_range.1 {
        return Err(format!(
            "Deck size {} exceeds maximum {}",
            deck_size, rules.deck_size_range.1
        ));
    }

    // Count card occurrences and legendary cards
    let mut card_counts: HashMap<&CardId, usize> = HashMap::new();
    let mut legendary_count = 0;

    for card_id in deck {
        // Check if card exists in registry
        if !registry.contains_card(card_id) {
            return Err(format!("Card {} not found in registry", card_id));
        }

        // Count this card
        *card_counts.entry(card_id).or_insert(0) += 1;

        // Check if it's a legendary card
        if let Some('L') = registry.get_card_type(card_id) {
            legendary_count += 1;
        }
    }

    // Check max same card limit
    for (card_id, count) in card_counts {
        if count > rules.max_same_card {
            return Err(format!(
                "Card {} appears {} times, exceeds limit of {}",
                card_id, count, rules.max_same_card
            ));
        }
    }

    // Check legendary card limit
    if legendary_count > rules.max_legendary {
        return Err(format!(
            "Deck contains {} legendary cards, exceeds limit of {}",
            legendary_count, rules.max_legendary
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock registry for testing
    struct MockRegistry {
        cards: HashMap<String, char>,
    }

    impl MockRegistry {
        fn new() -> Self {
            Self {
                cards: HashMap::new(),
            }
        }

        fn add_card(&mut self, id: &str, card_type: char) {
            self.cards.insert(id.to_string(), card_type);
        }
    }

    impl CardRegistry for MockRegistry {
        fn contains_card(&self, card_id: &CardId) -> bool {
            self.cards.contains_key(&card_id.0)
        }

        fn get_card_type(&self, card_id: &CardId) -> Option<char> {
            self.cards.get(&card_id.0).copied()
        }
    }

    #[test]
    fn test_default_rules() {
        let rules = GameRules::default();
        assert_eq!(rules.initial_hp, 5);
        assert_eq!(rules.max_hp, 6);
        assert_eq!(rules.max_hand, 20);
        assert_eq!(rules.max_cost_zone, 6);
        assert_eq!(rules.max_real_point, 6);
        assert_eq!(rules.deck_size_range, (40, 60));
        assert_eq!(rules.max_same_card, 3);
        assert_eq!(rules.max_legendary, 5);
        assert_eq!(rules.initial_hand_size, 5);
        assert_eq!(rules.first_player_draws, false);
        assert_eq!(rules.operation_timeout, Duration::from_secs(30));
        assert_eq!(rules.total_game_timeout, Duration::from_secs(3600));
    }

    #[test]
    fn test_validate_deck_valid_40_cards() {
        let mut registry = MockRegistry::new();
        let mut deck = Vec::new();

        // Add 40 cards: 10 of each type (C, S, I) and 10 of type L
        for i in 0..10 {
            let id = format!("S001-C-{:03}", i);
            registry.add_card(&id, 'C');
            deck.push(CardId::new(&id));
        }
        for i in 0..10 {
            let id = format!("S001-S-{:03}", i);
            registry.add_card(&id, 'S');
            deck.push(CardId::new(&id));
        }
        for i in 0..10 {
            let id = format!("S001-I-{:03}", i);
            registry.add_card(&id, 'I');
            deck.push(CardId::new(&id));
        }
        // Add only 5 legendary cards (within limit)
        for i in 0..5 {
            let id = format!("S001-L-{:03}", i);
            registry.add_card(&id, 'L');
            deck.push(CardId::new(&id));
        }
        // Add 5 more non-legendary cards to reach 40
        for i in 10..15 {
            let id = format!("S001-C-{:03}", i);
            registry.add_card(&id, 'C');
            deck.push(CardId::new(&id));
        }

        let rules = GameRules::default();
        match validate_deck(&deck, &rules, &registry) {
            Ok(()) => {}
            Err(e) => panic!("Validation failed: {}", e),
        }
    }

    #[test]
    fn test_validate_deck_too_small() {
        let mut registry = MockRegistry::new();
        let mut deck = Vec::new();

        // Add only 39 cards
        for i in 0..39 {
            let id = format!("S001-C-{:03}", i);
            registry.add_card(&id, 'C');
            deck.push(CardId::new(&id));
        }

        let rules = GameRules::default();
        let result = validate_deck(&deck, &rules, &registry);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("below minimum"));
    }

    #[test]
    fn test_validate_deck_too_large() {
        let mut registry = MockRegistry::new();
        let mut deck = Vec::new();

        // Add 61 cards
        for i in 0..61 {
            let id = format!("S001-C-{:03}", i);
            registry.add_card(&id, 'C');
            deck.push(CardId::new(&id));
        }

        let rules = GameRules::default();
        let result = validate_deck(&deck, &rules, &registry);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds maximum"));
    }

    #[test]
    fn test_validate_deck_too_many_same_card() {
        let mut registry = MockRegistry::new();
        let mut deck = Vec::new();

        // Add 4 copies of the same card (exceeds limit of 3)
        let card_id = "S001-C-001";
        registry.add_card(card_id, 'C');
        for _ in 0..4 {
            deck.push(CardId::new(card_id));
        }

        // Fill rest with different cards to reach 40
        for i in 4..40 {
            let id = format!("S001-C-{:03}", i);
            registry.add_card(&id, 'C');
            deck.push(CardId::new(&id));
        }

        let rules = GameRules::default();
        let result = validate_deck(&deck, &rules, &registry);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("appears 4 times"));
    }

    #[test]
    fn test_validate_deck_too_many_legendary() {
        let mut registry = MockRegistry::new();
        let mut deck = Vec::new();

        // Add 6 legendary cards (exceeds limit of 5)
        for i in 0..6 {
            let id = format!("S001-L-{:03}", i);
            registry.add_card(&id, 'L');
            deck.push(CardId::new(&id));
        }

        // Fill rest with non-legendary cards to reach 40
        for i in 6..40 {
            let id = format!("S001-C-{:03}", i);
            registry.add_card(&id, 'C');
            deck.push(CardId::new(&id));
        }

        let rules = GameRules::default();
        let result = validate_deck(&deck, &rules, &registry);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("6 legendary cards"));
    }

    #[test]
    fn test_validate_deck_card_not_in_registry() {
        let mut registry = MockRegistry::new();
        let mut deck = Vec::new();

        // Add 39 valid cards
        for i in 0..39 {
            let id = format!("S001-C-{:03}", i);
            registry.add_card(&id, 'C');
            deck.push(CardId::new(&id));
        }

        // Add 1 card that's not in registry
        deck.push(CardId::new("S001-C-999"));

        let rules = GameRules::default();
        let result = validate_deck(&deck, &rules, &registry);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("not found in registry"),
            "Got error: {}",
            err_msg
        );
    }

    #[test]
    fn test_validate_deck_custom_rules() {
        let mut registry = MockRegistry::new();
        let mut deck = Vec::new();

        // Create a deck with 50 cards
        for i in 0..50 {
            let id = format!("S001-C-{:03}", i);
            registry.add_card(&id, 'C');
            deck.push(CardId::new(&id));
        }

        // Custom rules: allow 50-50 deck size
        let mut rules = GameRules::default();
        rules.deck_size_range = (50, 50);

        assert!(validate_deck(&deck, &rules, &registry).is_ok());
    }
}
