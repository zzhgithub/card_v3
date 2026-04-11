use std::collections::HashMap;
use crate::effect::Effect;
use crate::types::EffectKey;
use serde_json::Value;

#[test]
fn test_simple_effect() {
    let json = r#"{"e1":{"actions":[{"amount":1,"player":"Self_","type":"HealHp"}],"optional":false,"trigger":"OpponentMainPhase"}}"#;

    let effects: HashMap<EffectKey, Value> = serde_json::from_str(json).unwrap();
    println!("Parsed JSON as Value map: {:?}", effects);

    for (key, value) in &effects {
        let effect_result: Result<Effect, _> = serde_json::from_value(value.clone());
        match &effect_result {
            Ok(effect) => println!("Effect {:?} parsed: {:?}", key, effect),
            Err(e) => println!("Error parsing effect {:?}: {}", key, e),
        }
    }
}

#[test]
fn test_effect_with_condition() {
    // Test with conditions that have ValueExpr::Literal with float value
    let json = r#"{"e1":{"actions":[{"amount":1,"player":"Self_","type":"HealHp"}],"conditions":{"left":{"player":"Self_","type":"RealPoint"},"op":"Ge","right":{"type":"Literal","value":3.0},"type":"Compare"},"optional":false,"trigger":"OpponentMainPhase"}}"#;

    let effects: HashMap<EffectKey, Value> = serde_json::from_str(json).unwrap();
    println!("Parsed JSON as Value map: {:?}", effects);

    for (key, value) in &effects {
        let effect_result: Result<Effect, _> = serde_json::from_value(value.clone());
        match &effect_result {
            Ok(effect) => println!("Effect {:?} parsed successfully: {:?}", key, effect),
            Err(e) => {
                println!("Error parsing effect {:?}: {}", key, e);
                panic!("Failed to parse effect with condition");
            }
        }
    }
}
