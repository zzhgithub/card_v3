use card_core::engine::{GameEngine, GameResult};
use card_core::rules::GameRules;
use card_core::state::events::GameOverReason;
use card_core::state::{CardInstance, CardRegistryImpl, CoreGameEvent, GameState};
use card_core::types::{CardDefinition, CardId, CardType, Category, InstanceId, Property};
use card_server::AiClient;

const ENGINE_MAX_TURNS: u32 = 500;

fn make_registry() -> CardRegistryImpl {
    let mut registry = CardRegistryImpl::new();
    registry.insert(CardDefinition::new(
        CardId::new("S000-C-001"),
        "Test Card".to_string(),
        CardType::Character,
        Property::Rational,
        Category::Math,
        1,
    ));
    registry
}

fn make_game_state(deck_size: usize) -> GameState {
    let rules = GameRules::default();
    let mut state = GameState::new(rules, 42);

    for i in 0..deck_size {
        state.players[0].zones.deck.push(CardInstance::new(
            InstanceId(i as u32 + 1),
            CardId::new("S000-C-001"),
            Some(300),
        ));
        state.players[1].zones.deck.push(CardInstance::new(
            InstanceId((i + deck_size) as u32 + 1),
            CardId::new("S000-C-001"),
            Some(300),
        ));
    }

    state
}

fn run_ai_game(state_seed: u64, ai1_seed: u64, ai2_seed: u64, deck_size: usize) -> GameResult {
    let mut state = make_game_state(deck_size);
    state.rng_seed = state_seed;
    state.turn_number = 2;

    let engine = GameEngine::new(
        state,
        make_registry(),
        Box::new(AiClient::new(ai1_seed)),
        Box::new(AiClient::new(ai2_seed)),
    );

    engine.run()
}

#[test]
fn ai_vs_ai_complete_game() {
    let result = run_ai_game(42, 7, 11, 20);

    assert!(result.winner.is_some(), "expected winner");
    assert!(
        !result.event_log.is_empty(),
        "event log should not be empty"
    );
}

#[test]
fn ai_vs_ai_is_deterministic_with_same_seeds() {
    let result1 = run_ai_game(42, 7, 11, 20);
    let result2 = run_ai_game(42, 7, 11, 20);

    assert_eq!(result1.winner, result2.winner);
    assert_eq!(
        result1.final_state.turn_number,
        result2.final_state.turn_number
    );
    assert_eq!(result1.event_log.len(), result2.event_log.len());
}

#[test]
fn event_log_contains_draw_and_phase_events() {
    let result = run_ai_game(42, 7, 11, 20);

    assert!(
        result
            .event_log
            .iter()
            .any(|ev| matches!(ev, CoreGameEvent::DrawCard { .. })),
        "expected at least one DrawCard event"
    );
    assert!(
        result
            .event_log
            .iter()
            .any(|ev| matches!(ev, CoreGameEvent::PhaseChanged { .. })),
        "expected at least one PhaseChanged event"
    );
}

#[test]
fn game_ends_within_engine_max_turns() {
    let result = run_ai_game(42, 7, 11, 20);

    assert!(
        result.final_state.turn_number <= ENGINE_MAX_TURNS,
        "turn {} exceeded max {}",
        result.final_state.turn_number,
        ENGINE_MAX_TURNS
    );
}

#[test]
fn small_deck_game_can_end_by_deck_out_quickly() {
    let result = run_ai_game(42, 35, 57, 1);

    assert!(
        matches!(result.reason, GameOverReason::DeckOut),
        "expected DeckOut, got {:?}",
        result.reason
    );
    assert!(
        result.final_state.turn_number < 10,
        "expected small deck game to end quickly, got turn {}",
        result.final_state.turn_number
    );
}
