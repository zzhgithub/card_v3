use super::{CardId, CardType, Category, EffectKey, ItemKind, Property, StrategyKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDefinition {
    pub id: CardId,
    pub name: String,
    pub card_type: CardType,
    pub property: Property,
    pub category: Category,
    pub tags: Vec<String>,
    pub cost: u32,
    pub attack: Option<u32>,
    pub strategy_kind: Option<StrategyKind>,
    pub item_kind: Option<ItemKind>,
    pub effects: HashMap<EffectKey, serde_json::Value>,
}

impl CardDefinition {
    pub fn new(
        id: CardId,
        name: String,
        card_type: CardType,
        property: Property,
        category: Category,
        cost: u32,
    ) -> Self {
        Self {
            id,
            name,
            card_type,
            property,
            category,
            tags: Vec::new(),
            cost,
            attack: None,
            strategy_kind: None,
            item_kind: None,
            effects: HashMap::new(),
        }
    }

    pub fn with_attack(mut self, attack: u32) -> Self {
        self.attack = Some(attack);
        self
    }

    pub fn with_strategy_kind(mut self, kind: StrategyKind) -> Self {
        self.strategy_kind = Some(kind);
        self
    }

    pub fn with_item_kind(mut self, kind: ItemKind) -> Self {
        self.item_kind = Some(kind);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_effect(mut self, key: EffectKey, effect: serde_json::Value) -> Self {
        self.effects.insert(key, effect);
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CardFilter {
    pub card_type: Option<CardType>,
    pub property: Option<Property>,
    pub category: Option<Category>,
    pub tags: Option<Vec<String>>,
    pub min_cost: Option<u32>,
    pub max_cost: Option<u32>,
}

impl CardFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_card_type(mut self, card_type: CardType) -> Self {
        self.card_type = Some(card_type);
        self
    }

    pub fn with_property(mut self, property: Property) -> Self {
        self.property = Some(property);
        self
    }

    pub fn with_category(mut self, category: Category) -> Self {
        self.category = Some(category);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_cost_range(mut self, min: u32, max: u32) -> Self {
        self.min_cost = Some(min);
        self.max_cost = Some(max);
        self
    }

    pub fn matches(&self, card: &CardDefinition) -> bool {
        if let Some(card_type) = self.card_type {
            if card.card_type != card_type {
                return false;
            }
        }

        if let Some(property) = self.property {
            if card.property != property {
                return false;
            }
        }

        if let Some(category) = self.category {
            if card.category != category {
                return false;
            }
        }

        if let Some(ref tags) = self.tags {
            if !tags.iter().all(|tag| card.tags.contains(tag)) {
                return false;
            }
        }

        if let Some(min_cost) = self.min_cost {
            if card.cost < min_cost {
                return false;
            }
        }

        if let Some(max_cost) = self.max_cost {
            if card.cost > max_cost {
                return false;
            }
        }

        true
    }
}
