use std::path::PathBuf;

use card_core::deck::{Deck, DeckManager};
use card_core::types::CardId;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("card_deck_integration_tests")
        .join(format!("{}_{}", name, std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn cleanup(dir: &PathBuf) {
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn test_deck_serialization_roundtrip() {
    let mut deck = Deck::new("TestDeck");
    deck.cards.push(CardId::new("S000-C-001"));
    deck.cards.push(CardId::new("S000-C-001"));
    deck.cards.push(CardId::new("S000-L-001"));

    let json = serde_json::to_string_pretty(&deck).unwrap();
    let decoded: Deck = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded.name, "TestDeck");
    assert_eq!(decoded.cards.len(), 3);
    assert_eq!(decoded.cards[0], CardId::new("S000-C-001"));
    assert_eq!(decoded.cards[2], CardId::new("S000-L-001"));
}

#[test]
fn test_deck_manager_save_load_roundtrip() {
    let dir = temp_dir("save_load_roundtrip");
    let path = dir.join("mydeck.json");

    let mut deck = Deck::new("RoundtripDeck");
    deck.cards.push(CardId::new("S000-C-001"));
    deck.cards.push(CardId::new("S000-S-001"));
    deck.cards.push(CardId::new("S000-I-001"));

    DeckManager::save(&path, &deck).unwrap();
    assert!(path.exists(), "saved file must exist");

    let loaded = DeckManager::load(&path).unwrap();
    assert_eq!(loaded, deck);

    cleanup(&dir);
}

#[test]
fn test_deck_manager_list_decks_returns_sorted() {
    let dir = temp_dir("list_sorted");

    let deck_b = Deck::new("Bravo");
    let deck_a = Deck::new("Alpha");
    let deck_c = Deck::new("Charlie");
    DeckManager::save(&dir.join("bravo.json"), &deck_b).unwrap();
    DeckManager::save(&dir.join("alpha.json"), &deck_a).unwrap();
    DeckManager::save(&dir.join("charlie.json"), &deck_c).unwrap();

    let summaries = DeckManager::list_decks(&dir).unwrap();
    assert_eq!(summaries.len(), 3);
    assert_eq!(summaries[0].name, "Alpha");
    assert_eq!(summaries[1].name, "Bravo");
    assert_eq!(summaries[2].name, "Charlie");

    cleanup(&dir);
}

#[test]
fn test_deck_manager_list_decks_nonexistent_dir() {
    let dir = PathBuf::from("/tmp/nonexistent_deck_dir_xyz_abc_123");
    let result = DeckManager::list_decks(&dir).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_deck_manager_load_invalid_json_returns_error() {
    let dir = temp_dir("invalid_json");
    let path = dir.join("broken.json");
    std::fs::write(&path, b"this is { not valid json!").unwrap();

    let result = DeckManager::load(&path);
    assert!(result.is_err(), "invalid JSON should return error");

    cleanup(&dir);
}

#[test]
fn test_deck_manager_delete() {
    let dir = temp_dir("delete");
    let path = dir.join("todelete.json");

    DeckManager::save(&path, &Deck::new("Temp")).unwrap();
    assert!(path.exists());

    DeckManager::delete(&path).unwrap();
    assert!(!path.exists());

    cleanup(&dir);
}

#[test]
fn test_example_deck_file_is_valid() {
    let candidates = [
        PathBuf::from("desks/Example.json"),
        PathBuf::from("../../desks/Example.json"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../desks/Example.json"),
    ];
    let path = candidates.iter().find(|p| p.exists()).cloned();
    let Some(path) = path else {
        return;
    };

    let deck = DeckManager::load(&path).expect("Example deck must load");
    assert_eq!(deck.name, "Example");
    assert_eq!(deck.cards.len(), 27, "Example deck should have 27 cards");

    let s000_prefix = "S000-";
    for card_id in &deck.cards {
        assert!(
            card_id.0.starts_with(s000_prefix),
            "all cards should be from S000 pack, got: {}",
            card_id.0
        );
    }
}

#[test]
fn test_deck_card_copy_limit_logic() {
    let mut deck = Deck::new("CopyLimit");
    let id = CardId::new("S000-C-001");

    for _ in 0..3 {
        deck.cards.push(id.clone());
    }
    assert_eq!(deck.cards.len(), 3);

    let count = deck.cards.iter().filter(|c| *c == &id).count();
    assert_eq!(count, 3);
}

#[test]
fn test_deck_summary_reflects_card_count() {
    let dir = temp_dir("summary_count");
    let path = dir.join("deck.json");

    let mut deck = Deck::new("CountDeck");
    for i in 0..15 {
        deck.cards
            .push(CardId::new(&format!("S000-C-00{}", (i % 3) + 1)));
    }
    DeckManager::save(&path, &deck).unwrap();

    let summaries = DeckManager::list_decks(&dir).unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].name, "CountDeck");
    assert_eq!(summaries[0].card_count, 15);

    cleanup(&dir);
}
