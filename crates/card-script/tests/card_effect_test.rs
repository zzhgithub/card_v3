use std::path::{Path, PathBuf};

use card_core::effect::text::card_effects_text;
use card_core::types::CardId;
use card_script::loader::{ScriptIndex, ScriptLoader};

fn scripts_root() -> PathBuf {
    let candidates = [
        PathBuf::from("scripts"),
        PathBuf::from("../../scripts"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts"),
    ];

    for candidate in candidates {
        if candidate.join("S000").exists() {
            return candidate;
        }
    }

    panic!("cannot locate scripts directory containing S000 pack")
}

fn load_card_effects(card_id: &str) -> Vec<String> {
    let index = ScriptIndex::scan(Path::new(&scripts_root())).expect("scan scripts should succeed");
    let loader = ScriptLoader::new(index);

    let card_id = CardId::new(card_id);
    let registry = loader
        .load_for_game(&[card_id.clone()], &[])
        .expect("loading card should succeed");

    let def = registry
        .get(&card_id)
        .expect("card should exist in registry");
    card_effects_text(&def.effects)
}

#[test]
fn s000_c_001_effects_text() {
    let texts = load_card_effects("S000-C-001");
    assert!(texts.is_empty());
}

#[test]
fn s000_c_002_effects_text() {
    let texts = load_card_effects("S000-C-002");
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "登场时，自己抽1张卡。");
}

#[test]
fn s000_c_003_effects_text() {
    let texts = load_card_effects("S000-C-003");
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "登场时，自己恢复1点生命值。");
}

#[test]
fn s000_s_001_effects_text() {
    let texts = load_card_effects("S000-S-001");
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "自己主要阶段，对手受到1点伤害。");
}

#[test]
fn s000_s_002_effects_text() {
    let texts = load_card_effects("S000-S-002");
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "自己主要阶段，对手丢弃1张手牌。");
}

#[test]
fn s000_s_003_effects_text() {
    let texts = load_card_effects("S000-S-003");
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "主要阶段，自己抽2张卡。");
}

#[test]
fn s000_i_001_effects_text() {
    let texts = load_card_effects("S000-I-001");
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "自己主要阶段，自己获得1点RealPoint。");
}

#[test]
fn s000_i_002_effects_text() {
    let texts = load_card_effects("S000-I-002");
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "回合开始时，自己恢复1点生命值。");
}

#[test]
fn s000_l_001_effects_text() {
    let texts = load_card_effects("S000-L-001");
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "登场时，对手受到2点伤害。");
}

#[test]
fn all_s000_cards_effects_are_valid() {
    let card_ids = vec![
        "S000-C-001",
        "S000-C-002",
        "S000-C-003",
        "S000-S-001",
        "S000-S-002",
        "S000-S-003",
        "S000-I-001",
        "S000-I-002",
        "S000-L-001",
    ];

    for card_id in &card_ids {
        let texts = load_card_effects(card_id);
        for text in &texts {
            assert!(!text.contains("效果数据异常"), "{}: {}", card_id, text);
        }
    }
}

#[test]
fn s000_cards_with_effects_have_correct_count() {
    let expected_counts = vec![
        ("S000-C-001", 0),
        ("S000-C-002", 1),
        ("S000-C-003", 1),
        ("S000-S-001", 1),
        ("S000-S-002", 1),
        ("S000-S-003", 1),
        ("S000-I-001", 1),
        ("S000-I-002", 1),
        ("S000-L-001", 1),
    ];

    for (card_id, expected_count) in expected_counts {
        let texts = load_card_effects(card_id);
        assert_eq!(
            texts.len(),
            expected_count,
            "{} expected {} effects, got {}",
            card_id,
            expected_count,
            texts.len()
        );
    }
}
