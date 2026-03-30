use std::path::PathBuf;

use anyhow::Result;
use card_core::engine::phase::PhaseClient;
use card_core::engine::{GameEngine, GameResult};
use card_core::rules::GameRules;
use card_core::state::{CardInstance, CardRegistryImpl, GameState};
use card_core::types::{CardDefinition, CardId, CardType, Category, InstanceId, Property};
use card_script::loader::{ScriptIndex, ScriptLoader};

pub fn run_local_game(
    state: GameState,
    registry: CardRegistryImpl,
    client1: Box<dyn PhaseClient>,
    client2: Box<dyn PhaseClient>,
) -> Result<GameResult> {
    let engine = GameEngine::new(state, registry, client1, client2);
    Ok(engine.run())
}

pub fn find_scripts_root() -> PathBuf {
    let candidates = [PathBuf::from("scripts"), PathBuf::from("../../scripts")];
    candidates
        .iter()
        .find(|p| p.join("S000").exists())
        .cloned()
        .unwrap_or(PathBuf::from("scripts"))
}

pub fn build_game_from_decks(
    player_cards: Vec<CardId>,
    ai_cards: Vec<CardId>,
    scripts_root: PathBuf,
) -> Result<(GameState, CardRegistryImpl)> {
    let index =
        ScriptIndex::scan(&scripts_root).map_err(|e| anyhow::anyhow!("扫描脚本目录失败: {e}"))?;
    let loader = ScriptLoader::new(index);
    let registry = loader
        .load_for_game(&player_cards, &ai_cards)
        .map_err(|e| anyhow::anyhow!("加载卡片定义失败: {e}"))?;

    let rules = GameRules::default();
    let mut state = GameState::new(rules, 20260322);
    let mut next_id = 1u32;

    for card_id in &player_cards {
        let base_attack = registry
            .get(card_id)
            .and_then(|def| def.attack.map(|a| a as i32));
        state.players[0].zones.deck.push(CardInstance::new(
            InstanceId(next_id),
            card_id.clone(),
            base_attack,
        ));
        next_id += 1;
    }

    for card_id in &ai_cards {
        let base_attack = registry
            .get(card_id)
            .and_then(|def| def.attack.map(|a| a as i32));
        state.players[1].zones.deck.push(CardInstance::new(
            InstanceId(next_id),
            card_id.clone(),
            base_attack,
        ));
        next_id += 1;
    }

    Ok((state, registry))
}

#[allow(dead_code)]
pub fn build_demo_state(rules: GameRules) -> (GameState, CardRegistryImpl) {
    let mut registry = CardRegistryImpl::new();
    let base_card_id = CardId::new("S000-C-001");
    registry.insert(CardDefinition::new(
        base_card_id.clone(),
        "训练木桩".to_string(),
        CardType::Character,
        Property::Rational,
        Category::Math,
        1,
    ));

    let mut state = GameState::new(rules, 20260322);
    let mut next_id = 1u32;
    for _ in 0..20 {
        state.players[0].zones.deck.push(CardInstance::new(
            InstanceId(next_id),
            base_card_id.clone(),
            Some(300),
        ));
        next_id += 1;
        state.players[1].zones.deck.push(CardInstance::new(
            InstanceId(next_id),
            base_card_id.clone(),
            Some(300),
        ));
        next_id += 1;
    }

    (state, registry)
}
