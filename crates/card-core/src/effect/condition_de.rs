use serde::{Deserialize, Deserializer};
use crate::effect::{Condition, CompareOp, ValueExpr};
use crate::types::CardRef;

/// Custom deserializer for Condition that handles both:
/// - Godot format: internally tagged {"type": "Compare", "left": ...}
/// - External tagged: {"Compare": {"left": ...}}
impl<'de> Deserialize<'de> for Condition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(tag = "type")]
        enum GodotCondition {
            And(Vec<Condition>),
            Or(Vec<Condition>),
            Not(Box<Condition>),
            Compare {
                left: ValueExpr,
                op: CompareOp,
                right: ValueExpr,
            },
            CardIsOnField { card: CardRef },
        }

        impl From<GodotCondition> for Condition {
            fn from(g: GodotCondition) -> Self {
                match g {
                    GodotCondition::And(v) => Condition::And(v),
                    GodotCondition::Or(v) => Condition::Or(v),
                    GodotCondition::Not(b) => Condition::Not(b),
                    GodotCondition::Compare { left, op, right } => {
                        Condition::Compare { left, op, right }
                    }
                    GodotCondition::CardIsOnField { card } => Condition::CardIsOnField { card },
                }
            }
        }

        // Try internally tagged format first (Godot format)
        let value = serde_json::Value::deserialize(deserializer)?;

        // Check if it has a "type" field (internally tagged format)
        if value.get("type").is_some() {
            let godot: GodotCondition = serde_json::from_value(value)
                .map_err(serde::de::Error::custom)?;
            return Ok(godot.into());
        }

        // Otherwise, try externally tagged format (Rust format)
        let ext: ExternallyTaggedCondition = serde_json::from_value(value)
            .map_err(serde::de::Error::custom)?;
        Ok(ext.into())
    }
}

#[derive(Deserialize)]
enum ExternallyTaggedCondition {
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
    Compare {
        left: ValueExpr,
        op: CompareOp,
        right: ValueExpr,
    },
    CardIsOnField { card: CardRef },
}

impl From<ExternallyTaggedCondition> for Condition {
    fn from(e: ExternallyTaggedCondition) -> Self {
        match e {
            ExternallyTaggedCondition::And(v) => Condition::And(v),
            ExternallyTaggedCondition::Or(v) => Condition::Or(v),
            ExternallyTaggedCondition::Not(b) => Condition::Not(b),
            ExternallyTaggedCondition::Compare { left, op, right } => {
                Condition::Compare { left, op, right }
            }
            ExternallyTaggedCondition::CardIsOnField { card } => Condition::CardIsOnField { card },
        }
    }
}
