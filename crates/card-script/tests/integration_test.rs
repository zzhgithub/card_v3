use std::collections::HashSet;
use std::path::{Path, PathBuf};

use card_core::types::{CardId, CardType};
use card_script::loader::{ScriptIndex, ScriptLoader};
use card_script::parser::parse_card_definition;
use card_script::sandbox::create_sandboxed_lua;
use card_script::ScriptError;
use mlua::Lua;

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

fn load_s000_cards() -> (Vec<CardId>, card_core::state::CardRegistryImpl) {
    let index = ScriptIndex::scan(Path::new(&scripts_root())).expect("scan scripts should succeed");

    let mut s000_ids: Vec<CardId> = index
        .iter()
        .map(|(id, _)| id.clone())
        .filter(|id| id.0.starts_with("S000-"))
        .collect();
    s000_ids.sort_by(|a, b| a.0.cmp(&b.0));

    let loader = ScriptLoader::new(index);
    let registry = loader
        .load_for_game(&s000_ids, &[])
        .expect("loading S000 cards should succeed");

    (s000_ids, registry)
}

fn parse_script(script: &str) -> Result<card_core::types::CardDefinition, ScriptError> {
    let lua = Lua::new();
    let table: mlua::Table = lua.load(script).eval().map_err(ScriptError::LuaError)?;
    parse_card_definition(&lua, table)
}

#[test]
fn s000_scan_finds_all_nine_cards() {
    let index = ScriptIndex::scan(Path::new(&scripts_root())).expect("scan scripts should succeed");
    let s000_count = index
        .iter()
        .filter(|(id, _)| id.0.starts_with("S000-"))
        .count();
    assert_eq!(s000_count, 9, "S000 should contain exactly 9 card scripts");
}

#[test]
fn s000_load_for_game_succeeds_for_all_ids() {
    let (ids, registry) = load_s000_cards();
    assert_eq!(ids.len(), 9, "expected all 9 S000 ids");
    assert_eq!(registry.len(), 9, "all S000 cards should be loaded");
}

#[test]
fn s000_loaded_cards_have_valid_core_fields() {
    let (ids, registry) = load_s000_cards();

    for id in &ids {
        let def = registry
            .get(id)
            .unwrap_or_else(|| panic!("missing loaded card definition for {}", id));
        assert!(!def.id.0.is_empty(), "card id should not be empty");
        assert!(!def.name.trim().is_empty(), "card name should not be empty");
        assert!(
            matches!(
                def.card_type,
                CardType::Character | CardType::Strategy | CardType::Item | CardType::Legendary
            ),
            "card_type must be one of supported variants"
        );
    }
}

#[test]
fn s000_contains_all_four_card_types() {
    let (ids, registry) = load_s000_cards();
    let mut seen = HashSet::new();

    for id in &ids {
        let def = registry
            .get(id)
            .unwrap_or_else(|| panic!("missing loaded card definition for {}", id));
        seen.insert(def.card_type);
    }

    assert!(seen.contains(&CardType::Character));
    assert!(seen.contains(&CardType::Strategy));
    assert!(seen.contains(&CardType::Item));
    assert!(seen.contains(&CardType::Legendary));
}

#[test]
fn s000_has_at_least_one_card_with_effects() {
    let (ids, registry) = load_s000_cards();

    let has_effects = ids.iter().any(|id| {
        registry
            .get(id)
            .map(|def| !def.effects.is_empty())
            .unwrap_or(false)
    });

    assert!(has_effects, "at least one S000 card should define effects");
}

#[test]
fn parser_reports_syntax_error_for_malformed_lua() {
    let malformed = "return { id = 'S000-C-001', name = 'Bad', card_type = 'Character', ";
    let result = parse_script(malformed);
    assert!(matches!(result, Err(ScriptError::LuaError(_))));
}

#[test]
fn parser_reports_error_when_required_id_missing() {
    let missing_id = r#"
return {
    name = "No Id",
    card_type = "Character",
    property = "Rational",
    category = "Math",
    cost = 1,
    attack = 100,
    effects = {}
}
"#;

    let result = parse_script(missing_id);
    assert!(matches!(
        result,
        Err(ScriptError::InvalidCardDefinition { .. })
    ));
}

#[test]
fn parser_reports_error_for_empty_lua_script() {
    let result = parse_script("");
    assert!(matches!(result, Err(ScriptError::LuaError(_))));
}

#[test]
fn sandbox_blocks_os_execute() {
    let lua = create_sandboxed_lua().expect("sandbox should initialize");
    let result: mlua::Result<()> = lua.load("os.execute('echo unsafe')").exec();
    assert!(result.is_err(), "os.execute must be blocked in sandbox");
}

#[test]
fn sandbox_blocks_io_open() {
    let lua = create_sandboxed_lua().expect("sandbox should initialize");
    let result: mlua::Result<()> = lua.load("io.open('x.txt', 'w')").exec();
    assert!(result.is_err(), "io.open must be blocked in sandbox");
}

#[test]
fn sandbox_blocks_require() {
    let lua = create_sandboxed_lua().expect("sandbox should initialize");
    let result: mlua::Result<()> = lua.load("require('os')").exec();
    assert!(result.is_err(), "require must be blocked in sandbox");
}

#[test]
fn sandbox_interrupts_unbounded_loop_via_memory_limit() {
    let lua = create_sandboxed_lua().expect("sandbox should initialize");
    let result: mlua::Result<()> = lua
        .load("local t = {}; while true do t[#t + 1] = string.rep('x', 1024) end")
        .exec();
    assert!(
        result.is_err(),
        "unbounded loop with allocations should be interrupted by sandbox limits"
    );
}
