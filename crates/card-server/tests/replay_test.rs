use std::time::{SystemTime, UNIX_EPOCH};

use card_core::engine::GameEngine;
use card_core::rules::GameRules;
use card_core::state::{CardInstance, CardRegistryImpl, CoreGameEvent, GameState, Phase};
use card_core::types::{
    CardDefinition, CardId, CardType, Category, InstanceId, PlayerId, Zone, ZoneLocation,
};
use card_server::{AiClient, ReplayData, ReplayRecorder};
use chrono::{TimeZone, Utc};

fn make_registry() -> CardRegistryImpl {
    let mut registry = CardRegistryImpl::new();
    registry.insert(CardDefinition::new(
        CardId::new("S000-C-001"),
        "Test Card".to_string(),
        CardType::Character,
        card_core::types::Property::Rational,
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

fn run_ai_game(
    state_seed: u64,
    ai1_seed: u64,
    ai2_seed: u64,
    deck_size: usize,
) -> card_core::engine::GameResult {
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

fn record_replay_from_game(
    state_seed: u64,
    ai1_seed: u64,
    ai2_seed: u64,
    deck_size: usize,
) -> ReplayData {
    let result = run_ai_game(state_seed, ai1_seed, ai2_seed, deck_size);
    let card_id = CardId::new("S000-C-001");
    let mut recorder = ReplayRecorder::new(
        GameRules::default(),
        vec![card_id.clone(); deck_size],
        vec![card_id; deck_size],
        state_seed,
    );
    recorder.record_events(&result.event_log);
    recorder.finalize()
}

#[test]
fn replay_recorder_records_events_and_metadata() {
    let replay = record_replay_from_game(42, 7, 11, 20);

    assert!(
        !replay.events.is_empty(),
        "replay events should not be empty"
    );
    assert!(!replay.deck1.is_empty(), "deck1 should not be empty");
    assert!(!replay.deck2.is_empty(), "deck2 should not be empty");
}

#[test]
fn replay_data_json_roundtrip() {
    let replay = ReplayData {
        version: "test-version".to_string(),
        timestamp: Utc.with_ymd_and_hms(2026, 3, 12, 10, 0, 0).unwrap(),
        rules: GameRules::default(),
        deck1: vec![CardId::new("S001-C-001")],
        deck2: vec![CardId::new("S001-C-002")],
        rng_seed: 12345,
        events: vec![
            CoreGameEvent::PhaseChanged {
                new_phase: Phase::TurnStart,
            },
            CoreGameEvent::CardMoved {
                instance_id: InstanceId(9),
                from: ZoneLocation::new(PlayerId::Player1, Zone::Deck),
                to: ZoneLocation::new(PlayerId::Player1, Zone::Hand),
            },
        ],
    };

    let json = serde_json::to_string(&replay).expect("serialize ReplayData to JSON");
    let decoded: ReplayData =
        serde_json::from_str(&json).expect("deserialize ReplayData from JSON");

    assert_eq!(decoded.version, replay.version);
    assert_eq!(decoded.timestamp, replay.timestamp);
    assert_eq!(decoded.rules, replay.rules);
    assert_eq!(decoded.deck1, replay.deck1);
    assert_eq!(decoded.deck2, replay.deck2);
    assert_eq!(decoded.rng_seed, replay.rng_seed);
    assert_eq!(decoded.events, replay.events);
}

#[test]
fn replay_data_file_save_and_load() {
    let replay = ReplayData {
        version: "file-roundtrip".to_string(),
        timestamp: Utc::now(),
        rules: GameRules::default(),
        deck1: vec![CardId::new("S001-C-010")],
        deck2: vec![CardId::new("S001-C-011")],
        rng_seed: 999,
        events: vec![CoreGameEvent::PhaseChanged {
            new_phase: Phase::Main1,
        }],
    };

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("card_server_replay_test_{nanos}.json"));

    replay
        .save_to_file(&path)
        .expect("save replay file should succeed");
    let loaded = ReplayData::load_from_file(&path).expect("load replay file should succeed");
    std::fs::remove_file(&path).expect("remove temp replay file");

    assert_eq!(loaded.version, replay.version);
    assert_eq!(loaded.timestamp, replay.timestamp);
    assert_eq!(loaded.rules, replay.rules);
    assert_eq!(loaded.deck1, replay.deck1);
    assert_eq!(loaded.deck2, replay.deck2);
    assert_eq!(loaded.rng_seed, replay.rng_seed);
    assert_eq!(loaded.events, replay.events);
}

#[test]
fn replay_data_contains_expected_fields_from_real_game() {
    let replay = record_replay_from_game(123, 31, 37, 20);

    assert!(!replay.version.is_empty(), "version should not be empty");
    assert!(
        replay.timestamp.timestamp() > 0,
        "timestamp should be a valid UTC datetime"
    );
    assert!(
        replay.events.len() > 0,
        "events should contain data from game execution"
    );
}

#[test]
fn multiple_replay_recordings_produce_distinct_event_logs() {
    let replay1 = record_replay_from_game(42, 7, 11, 20);
    let replay2 = record_replay_from_game(84, 19, 23, 20);

    assert_ne!(replay1.rng_seed, replay2.rng_seed);
    assert_ne!(
        replay1.events, replay2.events,
        "different seeds should produce different event logs"
    );
}
