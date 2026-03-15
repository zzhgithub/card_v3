use std::collections::HashMap;
use std::path::{Path, PathBuf};

use card_core::state::CardRegistryImpl;
use card_core::types::{CardDefinition, CardId};
use tracing::{debug, warn};

use crate::error::ScriptError;
use crate::parser::parse_card_definition;

// ─── ScriptIndex ─────────────────────────────────────────────────────────────

pub struct ScriptIndex {
    entries: HashMap<CardId, PathBuf>,
}

impl ScriptIndex {
    /// Scan `scripts_dir` recursively for `{CardId}.lua` files.
    /// Non-CardId filenames are skipped with a warning.
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
            } else if path.extension().and_then(|e| e.to_str()) == Some("lua") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    match stem.parse::<CardId>() {
                        Ok(card_id) => {
                            debug!("indexed script: {} -> {:?}", stem, path);
                            entries.insert(card_id, path);
                        }
                        Err(_) => {
                            warn!("skipping non-CardId lua file: {:?}", path);
                        }
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

            let definition = self.load_script(card_id, path)?;
            registry.insert(definition);
        }

        Ok(registry)
    }

    fn load_script(&self, card_id: &CardId, path: &Path) -> Result<CardDefinition, ScriptError> {
        let _ = card_id;
        let lua = mlua::Lua::new();
        let content = std::fs::read_to_string(path).map_err(|e| ScriptError::ParseError {
            reason: format!("cannot read {:?}: {}", path, e),
        })?;
        let table: mlua::Table = lua.load(&content).eval().map_err(ScriptError::LuaError)?;
        parse_card_definition(&lua, table)
    }
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
    fn test_scan_nonexistent_dir() {
        let dir = PathBuf::from("/tmp/card_script_tests_nonexistent_dir_xyz");
        let index = ScriptIndex::scan(&dir).unwrap();
        assert!(index.is_empty());
    }

    #[test]
    fn test_scan_finds_lua_files() {
        let dir = make_test_dir("finds_lua");
        fs::write(dir.join("S001-C-001.lua"), b"-- card script").unwrap();
        fs::write(dir.join("S001-S-002.lua"), b"-- strategy").unwrap();
        fs::write(dir.join("S002-I-001.lua"), b"-- item").unwrap();

        let index = ScriptIndex::scan(&dir).unwrap();
        assert_eq!(index.len(), 3);
        assert!(index.contains(&CardId::new("S001-C-001")));
        assert!(index.contains(&CardId::new("S001-S-002")));
        assert!(index.contains(&CardId::new("S002-I-001")));

        let path = index.get_path(&CardId::new("S001-C-001")).unwrap();
        assert!(path.exists());

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_scan_ignores_non_card_id_files() {
        let dir = make_test_dir("ignores_non_card");
        fs::write(dir.join("S001-C-001.lua"), b"-- valid").unwrap();
        fs::write(dir.join("readme.lua"), b"-- not a card").unwrap();
        fs::write(dir.join("utils.lua"), b"-- utility").unwrap();
        fs::write(dir.join("notes.txt"), b"not lua").unwrap();

        let index = ScriptIndex::scan(&dir).unwrap();
        assert_eq!(index.len(), 1);
        assert!(index.contains(&CardId::new("S001-C-001")));
        assert!(!index.contains(&CardId::new("readme")));

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_scan_recursive() {
        let dir = make_test_dir("recursive");
        let sub = dir.join("pack01");
        fs::create_dir_all(&sub).unwrap();

        fs::write(dir.join("S001-C-001.lua"), b"-- root").unwrap();
        fs::write(sub.join("S001-C-002.lua"), b"-- sub").unwrap();

        let index = ScriptIndex::scan(&dir).unwrap();
        assert_eq!(index.len(), 2);
        assert!(index.contains(&CardId::new("S001-C-001")));
        assert!(index.contains(&CardId::new("S001-C-002")));

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_loader_load_for_game() {
        let dir = make_test_dir("load_game");
        fs::write(
            dir.join("S001-C-001.lua"),
            r#"return {
    id = "S001-C-001",
    name = "Card 1",
    card_type = "Character",
    property = "Rational",
    category = "Math",
    cost = 1,
    attack = 100,
    effects = {}
}"#,
        )
        .unwrap();
        fs::write(
            dir.join("S001-C-002.lua"),
            r#"return {
    id = "S001-C-002",
    name = "Card 2",
    card_type = "Character",
    property = "Divine",
    category = "Science",
    cost = 2,
    attack = 200,
    effects = {}
}"#,
        )
        .unwrap();
        fs::write(
            dir.join("S001-S-001.lua"),
            r#"return {
    id = "S001-S-001",
    name = "Card 3",
    card_type = "Strategy",
    strategy_kind = "Normal",
    property = "Spiritual",
    category = "Literature",
    cost = 1,
    effects = {
        e1 = {
            trigger = "OwnMainPhase",
            actions = { { type = "Draw", player = "Self_", count = 1 } }
        }
    }
}"#,
        )
        .unwrap();

        let index = ScriptIndex::scan(&dir).unwrap();
        let loader = ScriptLoader::new(index);

        let deck1 = vec![CardId::new("S001-C-001"), CardId::new("S001-C-002")];
        let deck2 = vec![CardId::new("S001-C-001"), CardId::new("S001-S-001")];

        let registry = loader.load_for_game(&deck1, &deck2).unwrap();
        assert_eq!(registry.len(), 3);
        assert!(registry.get(&CardId::new("S001-C-001")).is_some());
        assert!(registry.get(&CardId::new("S001-C-002")).is_some());
        assert!(registry.get(&CardId::new("S001-S-001")).is_some());

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_loader_missing_card_error() {
        let dir = make_test_dir("missing_card");
        fs::write(
            dir.join("S001-C-001.lua"),
            r#"return {
    id = "S001-C-001",
    name = "Card 1",
    card_type = "Character",
    property = "Rational",
    category = "Math",
    cost = 1,
    attack = 100,
    effects = {}
}"#,
        )
        .unwrap();

        let index = ScriptIndex::scan(&dir).unwrap();
        let loader = ScriptLoader::new(index);

        let deck1 = vec![CardId::new("S001-C-001")];
        let deck2 = vec![CardId::new("S999-C-999")];

        let result = loader.load_for_game(&deck1, &deck2);
        assert!(result.is_err());

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_index_iter() {
        let dir = make_test_dir("iter");
        fs::write(dir.join("S001-C-001.lua"), b"-- a").unwrap();
        fs::write(dir.join("S001-L-001.lua"), b"-- b").unwrap();

        let index = ScriptIndex::scan(&dir).unwrap();
        let pairs: Vec<_> = index.iter().collect();
        assert_eq!(pairs.len(), 2);

        cleanup_test_dir(&dir);
    }
}
