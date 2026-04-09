//! JSON-based card definition loader.
//!
//! Replaces Lua script loading with direct JSON parsing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use card_core::state::CardRegistryImpl;
use card_core::types::{CardDefinition, CardId, CardType, Category, EffectKey, ItemKind, Property, StrategyKind};
use serde_json::Value;
use tracing::{debug, warn};

use crate::error::ScriptError;

// ─── ScriptIndex ─────────────────────────────────────────────────────────────

pub struct ScriptIndex {
    entries: HashMap<CardId, PathBuf>,
}

impl ScriptIndex {
    /// Scan `scripts_dir` recursively for `{CardId}.json` files.
    pub fn scan(scripts_dir: &Path) -> Result<Self, ScriptError> {
        let mut entries = HashMap::new();

        if !scripts_dir.exists() {
            return Ok(Self { entries });
        }

        Self::scan_dir(scripts_dir, &mut entries)?;

        debug!("script index built: {} entries", entries.len());
        Ok(Self { entries })
    }

    fn scan_dir(dir: &Path, entries: &mut HashMap<CardId, PathBuf>) -> Result<(), ScriptError> {
        let read_dir = std::fs::read_dir(dir).map_err(|e| ScriptError::ParseError {
            reason: format!("cannot read dir {:?}: {}", dir, e),
        })?;

        for entry in read_dir {
            let entry = entry.map_err(|e| ScriptError::ParseError {
                reason: e.to_string(),
            })?;
            let path = entry.path();

            if path.is_dir() {
                Self::scan_dir(&path, entries)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("json")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                match stem.parse::<CardId>() {
                    Ok(card_id) => {
                        debug!("indexed script: {} -> {:?}", stem, path);
                        entries.insert(card_id, path);
                    }
                    Err(_) => {
                        warn!("skipping non-CardId json file: {:?}", path);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get_path(&self, card_id: &CardId) -> Option<&Path> {
        self.entries.get(card_id).map(|p| p.as_path())
    }

    pub fn contains(&self, card_id: &CardId) -> bool {
        self.entries.contains_key(card_id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&CardId, &PathBuf)> {
        self.entries.iter()
    }
}

// ─── ScriptLoader ─────────────────────────────────────────────────────────────

pub struct ScriptLoader {
    index: ScriptIndex,
}

impl ScriptLoader {
    pub fn new(index: ScriptIndex) -> Self {
        Self { index }
    }

    pub fn load_for_game(
        &self,
        deck1: &[CardId],
        deck2: &[CardId],
    ) -> Result<CardRegistryImpl, ScriptError> {
        let mut to_load: std::collections::HashSet<CardId> = deck1.iter().cloned().collect();
        to_load.extend(deck2.iter().cloned());

        let mut registry = CardRegistryImpl::new();

        for card_id in &to_load {
            let path = self
                .index
                .get_path(card_id)
                .ok_or_else(|| ScriptError::CardNotFound(card_id.clone()))?;

            let definition = self.load_json(card_id, path)?;
            registry.insert(definition);
        }

        Ok(registry)
    }

    pub fn load_all(&self) -> Result<CardRegistryImpl, ScriptError> {
        let mut registry = CardRegistryImpl::new();
        for (card_id, path) in self.index.iter() {
            let definition = self.load_json(card_id, path)?;
            registry.insert(definition);
        }
        Ok(registry)
    }

    pub fn load_card(&self, card_id: &CardId) -> Result<CardDefinition, ScriptError> {
        let path = self
            .index
            .get_path(card_id)
            .ok_or_else(|| ScriptError::CardNotFound(card_id.clone()))?;
        self.load_json(card_id, path)
    }

    fn load_json(&self,
        _card_id: &CardId,
        path: &Path,
    ) -> Result<CardDefinition, ScriptError> {
        let content = std::fs::read_to_string(path).map_err(|e| ScriptError::ParseError {
            reason: format!("cannot read {:?}: {}", path, e),
        })?;

        let json: Value = serde_json::from_str(&content).map_err(|e| ScriptError::ParseError {
            reason: format!("JSON parse error in {:?}: {}", path, e),
        })?;

        parse_card_definition_from_json(&json)
    }
}

/// Parse a CardDefinition from JSON value.
fn parse_card_definition_from_json(json: &Value) -> Result<CardDefinition, ScriptError> {
    let id_str = json
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ScriptError::InvalidCardDefinition {
            reason: "missing 'id' field".to_string(),
        })?;

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ScriptError::InvalidCardDefinition {
            reason: "missing 'name' field".to_string(),
        })?;

    let card_type_str = json
        .get("card_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ScriptError::InvalidCardDefinition {
            reason: "missing 'card_type' field".to_string(),
        })?;

    let card_type = parse_card_type(card_type_str)?;

    let property_str = json
        .get("property")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ScriptError::InvalidCardDefinition {
            reason: "missing 'property' field".to_string(),
        })?;

    let property = parse_property(property_str)?;

    let category_str = json
        .get("category")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ScriptError::InvalidCardDefinition {
            reason: "missing 'category' field".to_string(),
        })?;

    let category = parse_category(category_str)?;

    let cost = json
        .get("cost")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ScriptError::InvalidCardDefinition {
            reason: "missing or invalid 'cost' field".to_string(),
        })? as u32;

    let mut def = CardDefinition::new(
        CardId::new(id_str),
        name.to_string(),
        card_type,
        property,
        category,
        cost,
    );

    // Optional fields
    if let Some(attack) = json.get("attack").and_then(|v| v.as_u64()) {
        def = def.with_attack(attack as u32);
    }

    if let Some(strategy_kind) = json.get("strategy_kind").and_then(|v| v.as_str()) {
        def = def.with_strategy_kind(parse_strategy_kind(strategy_kind)?);
    }

    if let Some(item_kind) = json.get("item_kind").and_then(|v| v.as_str()) {
        def = def.with_item_kind(parse_item_kind(item_kind)?);
    }

    if let Some(tags_array) = json.get("tags").and_then(|v| v.as_array()) {
        let tags: Vec<String> = tags_array
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        def = def.with_tags(tags);
    }

    // Effects
    if let Some(effects_obj) = json.get("effects").and_then(|v| v.as_object()) {
        for (key, value) in effects_obj {
            def = def.with_effect(EffectKey(key.clone()), value.clone());
        }
    }

    validate_type_specific_fields(id_str, &def)?;
    Ok(def)
}

fn parse_card_type(s: &str) -> Result<CardType, ScriptError> {
    match s {
        "Character" => Ok(CardType::Character),
        "Strategy" => Ok(CardType::Strategy),
        "Item" => Ok(CardType::Item),
        "Legendary" => Ok(CardType::Legendary),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown card_type '{}'", s),
        }),
    }
}

fn parse_property(s: &str) -> Result<Property, ScriptError> {
    match s {
        "Rational" => Ok(Property::Rational),
        "Divine" => Ok(Property::Divine),
        "Spiritual" => Ok(Property::Spiritual),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown property '{}'", s),
        }),
    }
}

fn parse_category(s: &str) -> Result<Category, ScriptError> {
    match s {
        "Math" => Ok(Category::Math),
        "Science" => Ok(Category::Science),
        "Literature" => Ok(Category::Literature),
        "Philosophy" => Ok(Category::Philosophy),
        "Mystery" => Ok(Category::Mystery),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown category '{}'", s),
        }),
    }
}

fn parse_strategy_kind(s: &str) -> Result<StrategyKind, ScriptError> {
    match s {
        "Normal" => Ok(StrategyKind::Normal),
        "Trick" => Ok(StrategyKind::Trick),
        "Instant" => Ok(StrategyKind::Instant),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown strategy_kind '{}'", s),
        }),
    }
}

fn parse_item_kind(s: &str) -> Result<ItemKind, ScriptError> {
    match s {
        "Normal" => Ok(ItemKind::Normal),
        "Persistent" => Ok(ItemKind::Persistent),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown item_kind '{}'", s),
        }),
    }
}

fn validate_type_specific_fields(id: &str, def: &CardDefinition) -> Result<(), ScriptError> {
    match def.card_type {
        CardType::Character | CardType::Legendary => {}
        CardType::Strategy | CardType::Item => {
            if def.attack.is_some() {
                return Err(ScriptError::InvalidCardDefinition {
                    reason: format!("card {} should not define attack", id),
                });
            }
        }
    }

    if matches!(def.card_type, CardType::Strategy) && def.strategy_kind.is_none() {
        return Err(ScriptError::InvalidCardDefinition {
            reason: format!("card {} missing strategy_kind", id),
        });
    }

    if matches!(def.card_type, CardType::Item) && def.item_kind.is_none() {
        return Err(ScriptError::InvalidCardDefinition {
            reason: format!("card {} missing item_kind", id),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("card_script_tests")
            .join(name)
            .join(format!("{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).ok();
        }
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn cleanup_test_dir(dir: &Path) {
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn test_scan_empty_dir() {
        let dir = make_test_dir("empty");
        let index = ScriptIndex::scan(&dir).unwrap();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_scan_finds_json_files() {
        let dir = make_test_dir("finds_json");
        fs::write(
            dir.join("S001-C-001.json"),
            r#"{"id": "S001-C-001", "name": "Card 1", "card_type": "Character", "property": "Rational", "category": "Math", "cost": 1}"#,
        )
        .unwrap();
        fs::write(
            dir.join("S001-S-002.json"),
            r#"{"id": "S001-S-002", "name": "Strategy", "card_type": "Strategy", "strategy_kind": "Normal", "property": "Divine", "category": "Science", "cost": 2}"#,
        )
        .unwrap();

        let index = ScriptIndex::scan(&dir).unwrap();
        assert_eq!(index.len(), 2);
        assert!(index.contains(&CardId::new("S001-C-001")));
        assert!(index.contains(&CardId::new("S001-S-002")));

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_load_basic_character() {
        let json = serde_json::json!({
            "id": "S000-C-001",
            "name": "Test Character",
            "card_type": "Character",
            "property": "Rational",
            "category": "Math",
            "cost": 3,
            "attack": 1500,
            "tags": [],
            "effects": {}
        });

        let def = parse_card_definition_from_json(&json).unwrap();
        assert_eq!(def.id.0, "S000-C-001");
        assert_eq!(def.name, "Test Character");
        assert!(matches!(def.card_type, CardType::Character));
        assert_eq!(def.attack, Some(1500));
        assert_eq!(def.cost, 3);
    }

    #[test]
    fn test_load_strategy_card() {
        let json = serde_json::json!({
            "id": "S000-S-001",
            "name": "Test Strategy",
            "card_type": "Strategy",
            "strategy_kind": "Normal",
            "property": "Rational",
            "category": "Science",
            "cost": 1,
            "effects": {
                "e1": {
                    "trigger": "OwnMainPhase",
                    "actions": [{ "type": "Draw", "player": "Self_", "count": 1 }]
                }
            }
        });

        let def = parse_card_definition_from_json(&json).unwrap();
        assert!(matches!(def.card_type, CardType::Strategy));
        assert!(def.attack.is_none());
        assert_eq!(def.effects.len(), 1);
    }

    #[test]
    fn test_load_item_card() {
        let json = serde_json::json!({
            "id": "S000-I-001",
            "name": "Test Item",
            "card_type": "Item",
            "item_kind": "Persistent",
            "property": "Spiritual",
            "category": "Literature",
            "cost": 2,
            "effects": {}
        });

        let def = parse_card_definition_from_json(&json).unwrap();
        assert!(matches!(def.card_type, CardType::Item));
        assert!(matches!(def.item_kind, Some(ItemKind::Persistent)));
    }

    #[test]
    fn test_missing_required_field_errors() {
        let json = serde_json::json!({
            "name": "Test",
            "card_type": "Character",
            "property": "Rational",
            "category": "Math",
            "cost": 1
        });

        let result = parse_card_definition_from_json(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_strategy_requires_strategy_kind() {
        let json = serde_json::json!({
            "id": "S000-S-001",
            "name": "Bad Strategy",
            "card_type": "Strategy",
            "property": "Rational",
            "category": "Math",
            "cost": 1
        });

        let result = parse_card_definition_from_json(&json);
        assert!(result.is_err());
    }
}
