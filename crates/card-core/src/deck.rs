use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::types::CardId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Deck {
    pub name: String,
    pub cards: Vec<CardId>,
}

impl Deck {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cards: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeckSummary {
    pub name: String,
    pub card_count: usize,
    pub file_path: PathBuf,
}

pub struct DeckManager;

impl DeckManager {
    pub fn save(path: &Path, deck: &Deck) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(deck)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> anyhow::Result<Deck> {
        let content = std::fs::read_to_string(path)?;
        let deck = serde_json::from_str(&content)?;
        Ok(deck)
    }

    pub fn list_decks(dir: &Path) -> anyhow::Result<Vec<DeckSummary>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut summaries = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json")
                && let Ok(deck) = Self::load(&path)
            {
                summaries.push(DeckSummary {
                    name: deck.name,
                    card_count: deck.cards.len(),
                    file_path: path,
                });
            }
        }
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(summaries)
    }

    pub fn delete(path: &Path) -> anyhow::Result<()> {
        std::fs::remove_file(path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("card_deck_tests").join(format!(
            "{}_{}",
            name,
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).ok();
        dir
    }

    fn cleanup(dir: &Path) {
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn test_deck_json_roundtrip() {
        let mut deck = Deck::new("MyDeck");
        deck.cards.push(CardId::new("S000-C-001"));
        deck.cards.push(CardId::new("S000-C-001"));
        deck.cards.push(CardId::new("S000-L-001"));

        let json = serde_json::to_string(&deck).unwrap();
        let decoded: Deck = serde_json::from_str(&json).unwrap();
        assert_eq!(deck, decoded);
        assert_eq!(decoded.name, "MyDeck");
        assert_eq!(decoded.cards.len(), 3);
    }

    #[test]
    fn test_deck_manager_save_load_roundtrip() {
        let dir = temp_dir("save_load");
        let path = dir.join("test_deck.json");

        let mut deck = Deck::new("TestDeck");
        deck.cards.push(CardId::new("S000-C-001"));
        deck.cards.push(CardId::new("S000-S-001"));

        DeckManager::save(&path, &deck).unwrap();
        assert!(path.exists());

        let loaded = DeckManager::load(&path).unwrap();
        assert_eq!(loaded, deck);

        cleanup(&dir);
    }

    #[test]
    fn test_deck_manager_list_decks() {
        let dir = temp_dir("list_decks");

        let deck_a = Deck::new("DeckA");
        let deck_b = Deck::new("DeckB");
        DeckManager::save(&dir.join("deck_a.json"), &deck_a).unwrap();
        DeckManager::save(&dir.join("deck_b.json"), &deck_b).unwrap();

        let summaries = DeckManager::list_decks(&dir).unwrap();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].name, "DeckA");
        assert_eq!(summaries[1].name, "DeckB");

        cleanup(&dir);
    }

    #[test]
    fn test_deck_manager_load_invalid_json_returns_error() {
        let dir = temp_dir("invalid_json");
        let path = dir.join("bad.json");
        std::fs::write(&path, b"this is not valid json!!!").unwrap();

        let result = DeckManager::load(&path);
        assert!(result.is_err());

        cleanup(&dir);
    }

    #[test]
    fn test_deck_manager_list_decks_empty_dir() {
        let dir = temp_dir("empty_dir");
        let summaries = DeckManager::list_decks(&dir).unwrap();
        assert!(summaries.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn test_example_deck_loads_correctly() {
        let candidates = [
            PathBuf::from("desks/Example.json"),
            PathBuf::from("../../desks/Example.json"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../desks/Example.json"),
        ];
        let path = candidates.iter().find(|p| p.exists()).cloned();
        let Some(path) = path else {
            return;
        };
        let deck = DeckManager::load(&path).unwrap();
        assert_eq!(deck.name, "Example");
        assert_eq!(deck.cards.len(), 27);
    }
}
