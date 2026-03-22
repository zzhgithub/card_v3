use anyhow::{bail, Result};

use card_core::engine::phase::PhaseClient;
use card_core::engine::{GameEngine, GameResult};
use card_core::rules::{validate_deck, GameRules};
use card_core::state::GameState;
use card_core::types::{CardId, PlayerId};
use card_script::loader::{ScriptIndex, ScriptLoader};

#[derive(Debug, Clone, PartialEq)]
pub enum SessionPhase {
    WaitingForPlayers,
    DeckSubmission,
    LoadingScripts,
    Playing,
    Finished,
}

struct PlayerSlot {
    deck: Vec<CardId>,
    client: Box<dyn PhaseClient>,
}

/// Manages a single game session from deck submission through game completion.
///
/// Flow: `WaitingForPlayers → DeckSubmission → LoadingScripts → Playing → Finished`
pub struct GameSession {
    phase: SessionPhase,
    rules: GameRules,
    script_index: ScriptIndex,
    players: Vec<PlayerSlot>,
    rng_seed: u64,
}

impl GameSession {
    pub fn new(rules: GameRules, script_index: ScriptIndex, rng_seed: u64) -> Self {
        Self {
            phase: SessionPhase::WaitingForPlayers,
            rules,
            script_index,
            players: Vec::new(),
            rng_seed,
        }
    }

    pub fn add_player(
        &mut self,
        client: Box<dyn PhaseClient>,
        deck: Vec<CardId>,
    ) -> Result<PlayerId> {
        match self.players.len() {
            0 => {
                self.players.push(PlayerSlot { deck, client });
                if self.phase == SessionPhase::WaitingForPlayers {
                    self.phase = SessionPhase::DeckSubmission;
                }
                Ok(PlayerId::Player1)
            }
            1 => {
                self.players.push(PlayerSlot { deck, client });
                Ok(PlayerId::Player2)
            }
            _ => bail!("Session already has 2 players"),
        }
    }

    pub fn start(mut self) -> Result<GameResult> {
        if self.players.len() < 2 {
            bail!("Need 2 players to start");
        }

        self.phase = SessionPhase::DeckSubmission;

        let deck1 = self.players[0].deck.clone();
        let deck2 = self.players[1].deck.clone();

        self.phase = SessionPhase::LoadingScripts;
        let loader = ScriptLoader::new(self.script_index);
        let registry = loader
            .load_for_game(&deck1, &deck2)
            .map_err(|e| anyhow::anyhow!("Script loading failed: {}", e))?;

        validate_deck(&deck1, &self.rules, &registry)
            .map_err(|e| anyhow::anyhow!("Player 1 deck invalid: {}", e))?;
        validate_deck(&deck2, &self.rules, &registry)
            .map_err(|e| anyhow::anyhow!("Player 2 deck invalid: {}", e))?;

        self.phase = SessionPhase::Playing;

        let state = GameState::new(self.rules.clone(), self.rng_seed);

        let mut players_iter = self.players.into_iter();
        let player1 = players_iter.next().unwrap();
        let player2 = players_iter.next().unwrap();

        let engine = GameEngine::new(state, registry, player1.client, player2.client);
        let result = engine.run();

        Ok(result)
    }

    pub fn phase(&self) -> &SessionPhase {
        &self.phase
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use card_core::engine::phase::{PhaseAction, PhaseClient, RecoveryCardOption};
    use card_core::rules::GameRules;
    use card_core::types::InstanceId;
    use card_script::loader::ScriptIndex;
    use std::path::Path;
    use std::time::Duration;

    struct SurrenderClient;
    impl PhaseClient for SurrenderClient {
        fn choose_action(&self, _: &[PhaseAction], _: Duration) -> Option<PhaseAction> {
            Some(PhaseAction::Surrender)
        }
        fn choose_recovery_cards(
            &self,
            _: &[RecoveryCardOption],
            _: usize,
            _: Duration,
        ) -> Option<Vec<InstanceId>> {
            Some(vec![])
        }
    }

    fn empty_index() -> ScriptIndex {
        ScriptIndex::scan(Path::new("/tmp/card_session_test_nonexistent")).unwrap()
    }

    #[test]
    fn session_starts_in_waiting_phase() {
        let session = GameSession::new(GameRules::default(), empty_index(), 42);
        assert_eq!(session.phase(), &SessionPhase::WaitingForPlayers);
    }

    #[test]
    fn session_add_first_player_returns_player1() {
        let mut session = GameSession::new(GameRules::default(), empty_index(), 42);
        let id = session
            .add_player(Box::new(SurrenderClient), vec![])
            .unwrap();
        assert_eq!(id, PlayerId::Player1);
        assert_eq!(session.phase(), &SessionPhase::DeckSubmission);
    }

    #[test]
    fn session_add_second_player_returns_player2() {
        let mut session = GameSession::new(GameRules::default(), empty_index(), 42);
        session
            .add_player(Box::new(SurrenderClient), vec![])
            .unwrap();
        let id = session
            .add_player(Box::new(SurrenderClient), vec![])
            .unwrap();
        assert_eq!(id, PlayerId::Player2);
    }

    #[test]
    fn session_rejects_third_player() {
        let mut session = GameSession::new(GameRules::default(), empty_index(), 42);
        session
            .add_player(Box::new(SurrenderClient), vec![])
            .unwrap();
        session
            .add_player(Box::new(SurrenderClient), vec![])
            .unwrap();
        let result = session.add_player(Box::new(SurrenderClient), vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn session_start_needs_two_players() {
        let mut session = GameSession::new(GameRules::default(), empty_index(), 42);
        session
            .add_player(Box::new(SurrenderClient), vec![])
            .unwrap();
        let result = session.start();
        assert!(result.is_err());
    }

    #[test]
    fn session_start_with_empty_decks_runs_game() {
        let mut rules = GameRules::default();
        // Allow empty decks for testing
        rules.deck_size_range = (0, 60);

        let mut session = GameSession::new(rules, empty_index(), 42);
        session
            .add_player(Box::new(SurrenderClient), vec![])
            .unwrap();
        session
            .add_player(Box::new(SurrenderClient), vec![])
            .unwrap();

        let result = session.start();
        assert!(result.is_ok());
    }
}
