use std::collections::{BTreeMap, HashSet};

use card_core::types::{
    CardDefinition, CardId, CardType, Category, EffectKey, ItemKind, Property, StrategyKind,
};
use mlua::{Lua, Table, Value};
use serde_json::{json, Map as JsonMap, Value as JsonValue};

use crate::error::ScriptError;

pub fn parse_card_definition(_lua: &Lua, table: Table) -> Result<CardDefinition, ScriptError> {
    let id_str = get_required_string(&table, "id")?;
    let name = get_required_string(&table, "name")?;
    let card_type = parse_card_type(&get_required_string(&table, "card_type")?)?;
    let property = parse_property(&get_required_string(&table, "property")?)?;
    let category = parse_category(&get_required_string(&table, "category")?)?;
    let cost = get_required_u32(&table, "cost")?;

    let attack = get_optional_u32(&table, "attack")?;
    let strategy_kind = table
        .get::<Option<String>>("strategy_kind")
        .map_err(ScriptError::LuaError)?
        .map(|v| parse_strategy_kind(&v))
        .transpose()?;
    let item_kind = table
        .get::<Option<String>>("item_kind")
        .map_err(ScriptError::LuaError)?
        .map(|v| parse_item_kind(&v))
        .transpose()?;

    let tags = parse_tags(&table)?;
    let effects = parse_effects(&table)?;

    let mut def = CardDefinition::new(
        CardId::new(&id_str),
        name,
        card_type,
        property,
        category,
        cost,
    );
    def.attack = attack;
    def.strategy_kind = strategy_kind;
    def.item_kind = item_kind;
    def.tags = tags;
    def.effects = effects;

    validate_type_specific_fields(&id_str, &def)?;
    Ok(def)
}

pub fn extract_referenced_cards(def: &CardDefinition) -> Vec<CardId> {
    let mut refs = Vec::new();
    let mut seen = HashSet::new();

    for effect in def.effects.values() {
        if let Some(actions) = effect.get("actions").and_then(JsonValue::as_array) {
            for action in actions {
                if action.get("type").and_then(JsonValue::as_str) != Some("SummonFromZone") {
                    continue;
                }
                if let Some(card_id) = action.get("card_id").and_then(JsonValue::as_str)
                    && seen.insert(card_id.to_string())
                {
                    refs.push(CardId::new(card_id));
                }
            }
        }
    }

    refs
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

fn get_required_string(table: &Table, key: &str) -> Result<String, ScriptError> {
    table
        .get::<Option<String>>(key)
        .map_err(ScriptError::LuaError)?
        .ok_or_else(|| ScriptError::InvalidCardDefinition {
            reason: format!("missing required field '{}'", key),
        })
}

fn get_required_u32(table: &Table, key: &str) -> Result<u32, ScriptError> {
    let value = table.get::<Value>(key).map_err(ScriptError::LuaError)?;
    value_to_u32(value, key)
}

fn get_optional_u32(table: &Table, key: &str) -> Result<Option<u32>, ScriptError> {
    let value = table.get::<Value>(key).map_err(ScriptError::LuaError)?;
    match value {
        Value::Nil => Ok(None),
        other => value_to_u32(other, key).map(Some),
    }
}

fn value_to_u32(value: Value, field: &str) -> Result<u32, ScriptError> {
    match value {
        Value::Integer(v) if v >= 0 => {
            u32::try_from(v).map_err(|_| ScriptError::InvalidCardDefinition {
                reason: format!("field '{}' is out of range", field),
            })
        }
        Value::Integer(_) => Err(ScriptError::InvalidCardDefinition {
            reason: format!("field '{}' cannot be negative", field),
        }),
        Value::Number(v) if v >= 0.0 => {
            if (v.fract() - 0.0).abs() > f64::EPSILON {
                return Err(ScriptError::InvalidCardDefinition {
                    reason: format!("field '{}' must be an integer", field),
                });
            }
            Ok(v as u32)
        }
        Value::Number(_) => Err(ScriptError::InvalidCardDefinition {
            reason: format!("field '{}' cannot be negative", field),
        }),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("field '{}' must be a number", field),
        }),
    }
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

fn parse_tags(table: &Table) -> Result<Vec<String>, ScriptError> {
    let tags_table = match table.get::<Value>("tags").map_err(ScriptError::LuaError)? {
        Value::Nil => return Ok(Vec::new()),
        Value::Table(t) => t,
        _ => {
            return Err(ScriptError::InvalidCardDefinition {
                reason: "field 'tags' must be an array".to_string(),
            });
        }
    };

    let mut tags = Vec::new();
    for value in tags_table.sequence_values::<String>() {
        tags.push(value.map_err(ScriptError::LuaError)?);
    }
    Ok(tags)
}

fn parse_effects(
    table: &Table,
) -> Result<std::collections::HashMap<EffectKey, JsonValue>, ScriptError> {
    let mut effects = std::collections::HashMap::new();
    let effects_table = match table
        .get::<Value>("effects")
        .map_err(ScriptError::LuaError)?
    {
        Value::Nil => return Ok(effects),
        Value::Table(t) => t,
        _ => {
            return Err(ScriptError::InvalidCardDefinition {
                reason: "field 'effects' must be a table".to_string(),
            });
        }
    };

    for pair in effects_table.pairs::<String, Table>() {
        let (key, effect_table) = pair.map_err(ScriptError::LuaError)?;
        let effect = parse_effect(&effect_table)?;
        effects.insert(EffectKey(key), effect);
    }
    Ok(effects)
}

fn parse_effect(table: &Table) -> Result<JsonValue, ScriptError> {
    let trigger = table
        .get::<Option<String>>("trigger")
        .map_err(ScriptError::LuaError)?
        .unwrap_or_else(|| "OwnMainPhase".to_string());
    parse_trigger(&trigger)?;

    let optional = table
        .get::<Option<bool>>("optional")
        .map_err(ScriptError::LuaError)?
        .unwrap_or(false);

    let activation_limit = table
        .get::<Option<String>>("activation_limit")
        .map_err(ScriptError::LuaError)?;
    if let Some(limit) = activation_limit.as_deref() {
        parse_activation_limit(limit)?;
    }

    let conditions = match table
        .get::<Value>("conditions")
        .map_err(ScriptError::LuaError)?
    {
        Value::Nil => None,
        Value::Table(t) => Some(parse_condition(&t)?),
        _ => {
            return Err(ScriptError::InvalidCardDefinition {
                reason: "field 'conditions' must be a table".to_string(),
            });
        }
    };

    let choices = parse_choices(table)?;
    let actions = parse_actions(table)?;
    let costs = match table.get::<Value>("costs").map_err(ScriptError::LuaError)? {
        Value::Nil => None,
        Value::Table(t) => Some(parse_cost_requirement(&t)?),
        _ => {
            return Err(ScriptError::InvalidCardDefinition {
                reason: "field 'costs' must be a table".to_string(),
            });
        }
    };

    Ok(json!({
        "trigger": trigger,
        "optional": optional,
        "activation_limit": activation_limit,
        "conditions": conditions,
        "choices": choices,
        "actions": actions,
        "costs": costs,
    }))
}

fn parse_trigger(s: &str) -> Result<(), ScriptError> {
    match s {
        "TurnStart" | "DrawPhase" | "RecoveryPhase" | "OwnMainPhase" | "OpponentMainPhase"
        | "BothMainPhase" | "BattlePhase" | "TurnEnd" | "OnSummon" | "OnAttack" | "OnExpose"
        | "OnDestroy" => Ok(()),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown trigger '{}'", s),
        }),
    }
}

fn parse_activation_limit(s: &str) -> Result<(), ScriptError> {
    match s {
        "OncePerTurn" | "OncePerTurnSameName" => Ok(()),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown activation_limit '{}'", s),
        }),
    }
}

fn parse_condition(table: &Table) -> Result<JsonValue, ScriptError> {
    let condition_type = get_required_string(table, "type")?;
    match condition_type.as_str() {
        "And" | "Or" => {
            let subs =
                table
                    .get::<Table>("subs")
                    .map_err(|_| ScriptError::InvalidCardDefinition {
                        reason: format!("{} condition requires 'subs'", condition_type),
                    })?;
            let mut parsed = Vec::new();
            for value in subs.sequence_values::<Table>() {
                parsed.push(parse_condition(&value.map_err(ScriptError::LuaError)?)?);
            }
            Ok(json!({"type": condition_type, "subs": parsed}))
        }
        "Not" => {
            let inner =
                table
                    .get::<Table>("inner")
                    .map_err(|_| ScriptError::InvalidCardDefinition {
                        reason: "Not condition requires 'inner'".to_string(),
                    })?;
            Ok(json!({"type": "Not", "inner": parse_condition(&inner)?}))
        }
        "Compare" => {
            let left =
                table
                    .get::<Table>("left")
                    .map_err(|_| ScriptError::InvalidCardDefinition {
                        reason: "Compare condition requires 'left'".to_string(),
                    })?;
            let right =
                table
                    .get::<Table>("right")
                    .map_err(|_| ScriptError::InvalidCardDefinition {
                        reason: "Compare condition requires 'right'".to_string(),
                    })?;
            let op = get_required_string(table, "op")?;
            parse_compare_op(&op)?;

            Ok(json!({
                "type": "Compare",
                "left": parse_value_expr(&left)?,
                "op": op,
                "right": parse_value_expr(&right)?,
            }))
        }
        "CardIsOnField" => {
            let card = table
                .get::<Option<String>>("card")
                .map_err(ScriptError::LuaError)?
                .unwrap_or_else(|| "This".to_string());
            parse_card_ref(&card)?;
            Ok(json!({"type": "CardIsOnField", "card": card}))
        }
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown condition type '{}'", condition_type),
        }),
    }
}

fn parse_compare_op(s: &str) -> Result<(), ScriptError> {
    match s {
        "Gt" | "Lt" | "Ge" | "Le" | "Eq" | "Ne" => Ok(()),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown compare op '{}'", s),
        }),
    }
}

fn parse_value_expr(table: &Table) -> Result<JsonValue, ScriptError> {
    let expr_type = get_required_string(table, "type")?;
    match expr_type.as_str() {
        "Literal" => {
            let n = table.get::<i32>("value").map_err(ScriptError::LuaError)?;
            Ok(json!({"type": "Literal", "value": n}))
        }
        "HandCount" | "CostZoneCount" | "FrontFieldCount" | "BackFieldCount" | "RealPoint"
        | "Hp" | "HighestCostOnField" => {
            let player = table
                .get::<Option<String>>("player")
                .map_err(ScriptError::LuaError)?
                .unwrap_or_else(|| "Self_".to_string());
            parse_player_ref(&player)?;
            Ok(json!({"type": expr_type, "player": player}))
        }
        "AttackPower" => {
            let card = table
                .get::<Option<String>>("card")
                .map_err(ScriptError::LuaError)?
                .unwrap_or_else(|| "This".to_string());
            parse_card_ref(&card)?;
            Ok(json!({"type": "AttackPower", "card": card}))
        }
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown value_expr type '{}'", expr_type),
        }),
    }
}

fn parse_player_ref(s: &str) -> Result<(), ScriptError> {
    match s {
        "Self_" | "Self" | "Opponent" => Ok(()),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown player_ref '{}'", s),
        }),
    }
}

fn parse_card_ref(s: &str) -> Result<(), ScriptError> {
    if s == "This" {
        return Ok(());
    }
    if let Some((zone, slot)) = s.split_once(':') {
        parse_zone(zone)?;
        slot.parse::<usize>()
            .map_err(|_| ScriptError::InvalidCardDefinition {
                reason: format!("invalid slot in card_ref '{}'", s),
            })?;
        return Ok(());
    }
    if s.parse::<u32>().is_ok() {
        return Ok(());
    }
    Err(ScriptError::InvalidCardDefinition {
        reason: format!("unknown card_ref '{}'", s),
    })
}

fn parse_choices(table: &Table) -> Result<Vec<JsonValue>, ScriptError> {
    let choices_table = match table
        .get::<Value>("choices")
        .map_err(ScriptError::LuaError)?
    {
        Value::Nil => return Ok(Vec::new()),
        Value::Table(t) => t,
        _ => {
            return Err(ScriptError::InvalidCardDefinition {
                reason: "field 'choices' must be an array table".to_string(),
            });
        }
    };

    let mut choices = Vec::new();
    for raw_choice in choices_table.sequence_values::<Table>() {
        let choice = raw_choice.map_err(ScriptError::LuaError)?;
        let choice_id = choice
            .get::<Option<u8>>("choice_id")
            .map_err(ScriptError::LuaError)?
            .unwrap_or(0);
        let target_type = choice
            .get::<Option<String>>("target_type")
            .map_err(ScriptError::LuaError)?
            .unwrap_or_else(|| "Card".to_string());
        parse_target_type(&target_type)?;

        let count = match choice
            .get::<Value>("count")
            .map_err(ScriptError::LuaError)?
        {
            Value::Nil => json!({"type": "Exactly", "value": 1}),
            Value::Table(t) => {
                let count_type = get_required_string(&t, "type")?;
                match count_type.as_str() {
                    "All" => json!({"type": "All"}),
                    "Exactly" | "UpTo" => {
                        let value = get_required_u32(&t, "value")?;
                        json!({"type": count_type, "value": value})
                    }
                    _ => {
                        return Err(ScriptError::InvalidCardDefinition {
                            reason: format!("unknown choice count type '{}'", count_type),
                        });
                    }
                }
            }
            _ => {
                return Err(ScriptError::InvalidCardDefinition {
                    reason: "choice.count must be a table".to_string(),
                });
            }
        };

        let filter = match choice
            .get::<Value>("filter")
            .map_err(ScriptError::LuaError)?
        {
            Value::Nil => JsonValue::Null,
            Value::Table(t) => lua_table_to_json_object(&t)?,
            _ => {
                return Err(ScriptError::InvalidCardDefinition {
                    reason: "choice.filter must be a table".to_string(),
                });
            }
        };

        choices.push(json!({
            "choice_id": choice_id,
            "target_type": target_type,
            "count": count,
            "filter": filter,
        }));
    }

    Ok(choices)
}

fn parse_actions(table: &Table) -> Result<Vec<JsonValue>, ScriptError> {
    let actions_table =
        table
            .get::<Table>("actions")
            .map_err(|_| ScriptError::InvalidCardDefinition {
                reason: "effect must define 'actions' array".to_string(),
            })?;

    let mut actions = Vec::new();
    for pair in actions_table.sequence_values::<Table>() {
        let action_table = pair.map_err(ScriptError::LuaError)?;
        actions.push(parse_action(&action_table)?);
    }
    Ok(actions)
}

fn parse_action(table: &Table) -> Result<JsonValue, ScriptError> {
    let action_type = get_required_string(table, "type")?;
    match action_type.as_str() {
        "Draw" | "Damage" | "HealHp" | "GainRealPoint" | "Discard" => {
            let player = table
                .get::<Option<String>>("player")
                .map_err(ScriptError::LuaError)?
                .unwrap_or_else(|| {
                    if action_type == "Damage" {
                        "Opponent".to_string()
                    } else {
                        "Self_".to_string()
                    }
                });
            parse_player_ref(&player)?;

            let amount_key = if action_type == "Draw" || action_type == "Discard" {
                "count"
            } else {
                "amount"
            };
            let amount = table
                .get::<Option<u32>>(amount_key)
                .map_err(ScriptError::LuaError)?
                .unwrap_or(1);

            Ok(json!({"type": action_type, "player": player, amount_key: amount}))
        }
        "Destroy" | "ReturnToHand" | "SendToGrave" | "ModifyAttack" => {
            let target = table
                .get::<Option<String>>("target")
                .map_err(ScriptError::LuaError)?
                .unwrap_or_else(|| "This".to_string());
            parse_card_ref(&target)?;

            let mut obj = JsonMap::new();
            obj.insert("type".to_string(), JsonValue::String(action_type.clone()));
            obj.insert("target".to_string(), JsonValue::String(target));
            if action_type == "ModifyAttack" {
                let amount = table
                    .get::<Option<i16>>("amount")
                    .map_err(ScriptError::LuaError)?
                    .unwrap_or(0);
                obj.insert(
                    "amount".to_string(),
                    JsonValue::Number(serde_json::Number::from(amount)),
                );
            }
            Ok(JsonValue::Object(obj))
        }
        "SummonFromZone" => {
            let card_id = get_required_string(table, "card_id")?;
            let no_cost = table
                .get::<Option<bool>>("no_cost")
                .map_err(ScriptError::LuaError)?
                .unwrap_or(false);
            let from_zones = match table
                .get::<Value>("from_zones")
                .map_err(ScriptError::LuaError)?
            {
                Value::Nil => {
                    vec![
                        JsonValue::String("Deck".to_string()),
                        JsonValue::String("CostZone".to_string()),
                        JsonValue::String("Grave".to_string()),
                    ]
                }
                Value::Table(t) => {
                    let mut zones = Vec::new();
                    for entry in t.sequence_values::<String>() {
                        let zone = entry.map_err(ScriptError::LuaError)?;
                        parse_zone(&zone)?;
                        zones.push(JsonValue::String(zone));
                    }
                    zones
                }
                _ => {
                    return Err(ScriptError::InvalidCardDefinition {
                        reason: "SummonFromZone.from_zones must be an array".to_string(),
                    });
                }
            };

            let to_zone = table
                .get::<Option<String>>("to_zone")
                .map_err(ScriptError::LuaError)?
                .unwrap_or_else(|| "Front:0".to_string());
            parse_zone_ref(&to_zone)?;

            Ok(json!({
                "type": "SummonFromZone",
                "card_id": card_id,
                "from_zones": from_zones,
                "to_zone": to_zone,
                "no_cost": no_cost,
            }))
        }
        "Special" => {
            let key = get_required_string(table, "key")?;
            Ok(json!({"type": "Special", "key": key}))
        }
        "ApplyModifier" | "RemoveModifier" => lua_table_to_json_object(table),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown action type '{}'", action_type),
        }),
    }
}

fn parse_cost_requirement(table: &Table) -> Result<JsonValue, ScriptError> {
    let cost_type = get_required_string(table, "type")?;
    match cost_type.as_str() {
        "SendFieldCardToGrave" | "DiscardHand" | "SendCostZoneToGrave" => {
            let mut obj = JsonMap::new();
            obj.insert("type".to_string(), JsonValue::String(cost_type));
            obj.insert(
                "count".to_string(),
                JsonValue::Number(serde_json::Number::from(
                    table
                        .get::<Option<u32>>("count")
                        .map_err(ScriptError::LuaError)?
                        .unwrap_or(1),
                )),
            );

            match table
                .get::<Value>("filter")
                .map_err(ScriptError::LuaError)?
            {
                Value::Nil => obj.insert("filter".to_string(), JsonValue::Null),
                Value::Table(t) => obj.insert("filter".to_string(), lua_table_to_json_object(&t)?),
                _ => {
                    return Err(ScriptError::InvalidCardDefinition {
                        reason: "cost filter must be a table".to_string(),
                    });
                }
            };

            Ok(JsonValue::Object(obj))
        }
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown cost type '{}'", cost_type),
        }),
    }
}

fn parse_target_type(s: &str) -> Result<(), ScriptError> {
    match s {
        "Player" | "Card" | "Zone" => Ok(()),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown target_type '{}'", s),
        }),
    }
}

fn parse_zone(s: &str) -> Result<(), ScriptError> {
    match s {
        "Deck" | "Hand" | "CostZone" | "Grave" => Ok(()),
        _ => Err(ScriptError::InvalidCardDefinition {
            reason: format!("unknown zone '{}'", s),
        }),
    }
}

fn parse_zone_ref(s: &str) -> Result<(), ScriptError> {
    if let Some((zone, slot)) = s.split_once(':') {
        if zone != "Front" && zone != "Back" {
            return Err(ScriptError::InvalidCardDefinition {
                reason: format!("unknown indexed zone '{}'", zone),
            });
        }
        slot.parse::<usize>()
            .map_err(|_| ScriptError::InvalidCardDefinition {
                reason: format!("invalid zone slot in '{}'", s),
            })?;
        return Ok(());
    }
    parse_zone(s)
}

fn lua_table_to_json_object(table: &Table) -> Result<JsonValue, ScriptError> {
    lua_value_to_json(Value::Table(table.clone()))
}

fn lua_value_to_json(value: Value) -> Result<JsonValue, ScriptError> {
    match value {
        Value::Nil => Ok(JsonValue::Null),
        Value::Boolean(v) => Ok(JsonValue::Bool(v)),
        Value::Integer(v) => Ok(JsonValue::Number(serde_json::Number::from(v))),
        Value::Number(v) => serde_json::Number::from_f64(v)
            .map(JsonValue::Number)
            .ok_or_else(|| ScriptError::ParseError {
                reason: "cannot encode non-finite float".to_string(),
            }),
        Value::String(v) => Ok(JsonValue::String(
            v.to_str().map_err(ScriptError::LuaError)?.to_string(),
        )),
        Value::Table(table) => {
            let entries = collect_table_entries(&table)?;
            if entries.is_empty() {
                return Ok(JsonValue::Array(Vec::new()));
            }

            let mut int_keys = BTreeMap::new();
            let mut object_entries = JsonMap::new();
            let mut has_non_int = false;

            for (key, val) in entries {
                match key {
                    Value::Integer(i) if i >= 1 => {
                        int_keys.insert(i as usize, val);
                    }
                    Value::String(s) => {
                        has_non_int = true;
                        object_entries.insert(
                            s.to_str().map_err(ScriptError::LuaError)?.to_string(),
                            lua_value_to_json(val)?,
                        );
                    }
                    _ => {
                        return Err(ScriptError::ParseError {
                            reason: "table contains unsupported key type".to_string(),
                        });
                    }
                }
            }

            if !has_non_int {
                let mut arr = Vec::new();
                for (expected, (idx, val)) in (1usize..).zip(int_keys) {
                    if idx != expected {
                        return Err(ScriptError::ParseError {
                            reason: "array table has sparse keys".to_string(),
                        });
                    }
                    arr.push(lua_value_to_json(val)?);
                }
                return Ok(JsonValue::Array(arr));
            }

            for (idx, val) in int_keys {
                object_entries.insert(idx.to_string(), lua_value_to_json(val)?);
            }
            Ok(JsonValue::Object(object_entries))
        }
        Value::Function(_) | Value::Thread(_) | Value::UserData(_) | Value::LightUserData(_) => {
            Err(ScriptError::ParseError {
                reason: "unsupported lua value in card definition".to_string(),
            })
        }
        Value::Error(err) => Err(ScriptError::LuaError(*err)),
        Value::Other(_) => Err(ScriptError::ParseError {
            reason: "unsupported opaque lua value in card definition".to_string(),
        }),
    }
}

fn collect_table_entries(table: &Table) -> Result<Vec<(Value, Value)>, ScriptError> {
    let mut entries = Vec::new();
    for pair in table.pairs::<Value, Value>() {
        let (k, v) = pair.map_err(ScriptError::LuaError)?;
        entries.push((k, v));
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use card_core::types::{CardType, EffectKey, StrategyKind};

    #[test]
    fn test_parse_basic_character() {
        let lua = Lua::new();
        let script = r#"
return {
    id = "S000-C-001",
    name = "Test Character",
    card_type = "Character",
    property = "Rational",
    category = "Math",
    cost = 3,
    attack = 1500,
    tags = {},
    effects = {}
}
"#;
        let table: mlua::Table = lua.load(script).eval().unwrap();
        let def = parse_card_definition(&lua, table).unwrap();

        assert_eq!(def.id.0, "S000-C-001");
        assert_eq!(def.name, "Test Character");
        assert!(matches!(def.card_type, CardType::Character));
        assert_eq!(def.attack, Some(1500));
        assert_eq!(def.cost, 3);
    }

    #[test]
    fn test_parse_with_onsummon_effect() {
        let lua = Lua::new();
        let script = r#"
return {
    id = "S000-C-002",
    name = "Healer",
    card_type = "Character",
    property = "Divine",
    category = "Philosophy",
    cost = 2,
    attack = 800,
    tags = {},
    effects = {
        e1 = {
            trigger = "OnSummon",
            optional = false,
            actions = {
                { type = "HealHp", player = "Self_", amount = 1 }
            }
        }
    }
}
"#;
        let table: mlua::Table = lua.load(script).eval().unwrap();
        let def = parse_card_definition(&lua, table).unwrap();

        assert_eq!(def.effects.len(), 1);
        let effect = def.effects.get(&EffectKey("e1".to_string())).unwrap();
        assert_eq!(
            effect["trigger"],
            serde_json::Value::String("OnSummon".to_string())
        );
        assert_eq!(effect["actions"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_parse_strategy_card() {
        let lua = Lua::new();
        let script = r#"
return {
    id = "S000-S-001",
    name = "Draw Strategy",
    card_type = "Strategy",
    strategy_kind = "Normal",
    property = "Rational",
    category = "Science",
    cost = 1,
    tags = {},
    effects = {
        e1 = {
            trigger = "OwnMainPhase",
            optional = true,
            actions = {
                { type = "Draw", player = "Self_", count = 1 }
            }
        }
    }
}
"#;
        let table: mlua::Table = lua.load(script).eval().unwrap();
        let def = parse_card_definition(&lua, table).unwrap();

        assert!(matches!(def.strategy_kind, Some(StrategyKind::Normal)));
        assert!(def.attack.is_none());
    }

    #[test]
    fn test_parse_missing_required_field_errors() {
        let lua = Lua::new();
        let script = r#"return { name = "Test", card_type = "Character", property = "Rational", category = "Math", cost = 1 }"#;
        let table: mlua::Table = lua.load(script).eval().unwrap();
        let result = parse_card_definition(&lua, table);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_referenced_cards() {
        let lua = Lua::new();
        let script = r#"
return {
    id = "S000-C-003",
    name = "Summoner",
    card_type = "Character",
    property = "Rational",
    category = "Math",
    cost = 4,
    attack = 1000,
    tags = {},
    effects = {
        e1 = {
            trigger = "OnSummon",
            optional = false,
            actions = {
                { type = "SummonFromZone", card_id = "S000-C-001", no_cost = true }
            }
        }
    }
}
"#;
        let table: mlua::Table = lua.load(script).eval().unwrap();
        let def = parse_card_definition(&lua, table).unwrap();
        let refs = extract_referenced_cards(&def);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].0, "S000-C-001");
    }
}
