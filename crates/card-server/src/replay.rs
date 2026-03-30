use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use card_core::rules::GameRules;
use card_core::state::CoreGameEvent;
use card_core::types::CardId;

/// Recorded game data for replay and verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayData {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub rules: GameRules,
    pub deck1: Vec<CardId>,
    pub deck2: Vec<CardId>,
    pub rng_seed: u64,
    pub events: Vec<CoreGameEvent>,
}

impl ReplayData {
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

/// Records game events for later replay.
pub struct ReplayRecorder {
    version: String,
    rules: GameRules,
    deck1: Vec<CardId>,
    deck2: Vec<CardId>,
    rng_seed: u64,
    events: Vec<CoreGameEvent>,
}

impl ReplayRecorder {
    pub fn new(rules: GameRules, deck1: Vec<CardId>, deck2: Vec<CardId>, rng_seed: u64) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            rules,
            deck1,
            deck2,
            rng_seed,
            events: Vec::new(),
        }
    }

    pub fn record_event(&mut self, event: CoreGameEvent) {
        self.events.push(event);
    }

    pub fn record_events(&mut self, events: &[CoreGameEvent]) {
        self.events.extend_from_slice(events);
    }

    pub fn finalize(self) -> ReplayData {
        ReplayData {
            version: self.version,
            timestamp: Utc::now(),
            rules: self.rules,
            deck1: self.deck1,
            deck2: self.deck2,
            rng_seed: self.rng_seed,
            events: self.events,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use card_core::state::Phase;

    #[test]
    fn replay_recorder_finalize() {
        let rules = GameRules::default();
        let recorder = ReplayRecorder::new(rules.clone(), vec![], vec![], 42);
        let data = recorder.finalize();
        assert_eq!(data.rng_seed, 42);
        assert!(data.events.is_empty());
    }

    #[test]
    fn replay_data_json_roundtrip() {
        let rules = GameRules::default();
        let mut recorder = ReplayRecorder::new(rules, vec![], vec![], 99);
        recorder.record_event(CoreGameEvent::PhaseChanged {
            new_phase: Phase::TurnStart,
        });
        let data = recorder.finalize();
        let json = serde_json::to_string(&data).unwrap();
        let loaded: ReplayData = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.rng_seed, 99);
        assert_eq!(loaded.events.len(), 1);
    }

    #[test]
    fn replay_data_file_roundtrip() {
        let rules = GameRules::default();
        let recorder = ReplayRecorder::new(rules, vec![], vec![], 77);
        let data = recorder.finalize();
        let tmp = std::env::temp_dir().join("test_replay.json");
        data.save_to_file(&tmp).unwrap();
        let loaded = ReplayData::load_from_file(&tmp).unwrap();
        assert_eq!(loaded.rng_seed, 77);
        let _ = std::fs::remove_file(&tmp);
    }
}
