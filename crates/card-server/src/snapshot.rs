use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use card_core::rules::GameRules;
use card_core::state::GameState;
use card_core::types::CardId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub state: GameState,
    pub rules: GameRules,
    pub card_ids: Vec<CardId>,
}

pub fn save_snapshot(state: &GameState, card_ids: Vec<CardId>) -> GameSnapshot {
    GameSnapshot {
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
        state: state.clone(),
        rules: state.rules.clone(),
        card_ids,
    }
}

impl GameSnapshot {
    pub fn save_to_file(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use card_core::rules::GameRules;
    use card_core::state::GameState;

    #[test]
    fn snapshot_json_roundtrip() {
        let state = GameState::new(GameRules::default(), 42);
        let snap = save_snapshot(&state, vec![]);
        let json = serde_json::to_string(&snap).unwrap();
        let loaded: GameSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.state.rng_seed, 42);
    }

    #[test]
    fn snapshot_file_roundtrip() {
        let state = GameState::new(GameRules::default(), 55);
        let snap = save_snapshot(&state, vec![]);
        let tmp = std::env::temp_dir().join("test_snapshot.json");
        snap.save_to_file(&tmp).unwrap();
        let loaded = GameSnapshot::load_from_file(&tmp).unwrap();
        assert_eq!(loaded.state.rng_seed, 55);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn snapshot_preserves_turn_number() {
        let mut state = GameState::new(GameRules::default(), 1);
        state.turn_number = 5;
        let snap = save_snapshot(&state, vec![]);
        let json = serde_json::to_string(&snap).unwrap();
        let loaded: GameSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.state.turn_number, 5);
    }
}
