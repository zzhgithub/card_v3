#[cfg(test)]
mod integration_tests {
    use crate::types::*;

    #[test]
    fn test_all_types_serialize() {
        let card_id = CardId::new("S001-C-001");
        let json = serde_json::to_string(&card_id).unwrap();
        let deserialized: CardId = serde_json::from_str(&json).unwrap();
        assert_eq!(card_id, deserialized);
    }

    #[test]
    fn test_player_id_opponent() {
        assert_eq!(PlayerId::Player1.opponent(), PlayerId::Player2);
        assert_eq!(PlayerId::Player2.opponent(), PlayerId::Player1);
    }

    #[test]
    fn test_player_ref_resolve() {
        assert_eq!(
            PlayerRef::Self_.resolve(PlayerId::Player1),
            PlayerId::Player1
        );
        assert_eq!(
            PlayerRef::Opponent.resolve(PlayerId::Player1),
            PlayerId::Player2
        );
    }

    #[test]
    fn test_zone_location() {
        let loc = ZoneLocation::new(PlayerId::Player1, Zone::Hand);
        assert_eq!(loc.player, PlayerId::Player1);
        assert_eq!(loc.zone, Zone::Hand);
    }

    #[test]
    fn test_card_definition_builder() {
        let def = CardDefinition::new(
            CardId::new("S001-C-001"),
            "Test Character".to_string(),
            CardType::Character,
            Property::Rational,
            Category::Math,
            3,
        )
        .with_attack(100)
        .with_tags(vec!["tag1".to_string(), "tag2".to_string()]);

        assert_eq!(def.name, "Test Character");
        assert_eq!(def.card_type, CardType::Character);
        assert_eq!(def.attack, Some(100));
        assert_eq!(def.tags.len(), 2);
    }

    #[test]
    fn test_card_filter_matches() {
        let card = CardDefinition::new(
            CardId::new("S001-C-001"),
            "Test".to_string(),
            CardType::Character,
            Property::Rational,
            Category::Math,
            3,
        );

        let filter = CardFilter::new()
            .with_card_type(CardType::Character)
            .with_property(Property::Rational);

        assert!(filter.matches(&card));

        let filter_no_match = CardFilter::new().with_card_type(CardType::Strategy);
        assert!(!filter_no_match.matches(&card));
    }

    #[test]
    fn test_card_filter_cost_range() {
        let card = CardDefinition::new(
            CardId::new("S001-C-001"),
            "Test".to_string(),
            CardType::Character,
            Property::Rational,
            Category::Math,
            5,
        );

        let filter = CardFilter::new().with_cost_range(3, 7);
        assert!(filter.matches(&card));

        let filter_too_low = CardFilter::new().with_cost_range(6, 10);
        assert!(!filter_too_low.matches(&card));
    }

    #[test]
    fn test_instance_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(InstanceId(1));
        set.insert(InstanceId(2));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_effect_key_serialize() {
        let key = EffectKey("e1".to_string());
        let json = serde_json::to_string(&key).unwrap();
        let deserialized: EffectKey = serde_json::from_str(&json).unwrap();
        assert_eq!(key, deserialized);
    }

    #[test]
    fn test_card_ref_variants() {
        let ref1 = CardRef::This;
        let ref2 = CardRef::ByInstanceId(InstanceId(1));
        let ref3 = CardRef::BySlot(Zone::Hand, 0);

        assert_eq!(ref1, CardRef::This);
        assert_eq!(ref2, CardRef::ByInstanceId(InstanceId(1)));
        assert_eq!(ref3, CardRef::BySlot(Zone::Hand, 0));
    }

    #[test]
    fn test_target_ref_resolve_player() {
        let target = TargetRef::Player(PlayerRef::Self_);
        assert_eq!(
            target.resolve_player(PlayerId::Player1),
            Some(PlayerId::Player1)
        );

        let target_opponent = TargetRef::Player(PlayerRef::Opponent);
        assert_eq!(
            target_opponent.resolve_player(PlayerId::Player1),
            Some(PlayerId::Player2)
        );

        let target_card = TargetRef::Card(CardRef::This);
        assert_eq!(target_card.resolve_player(PlayerId::Player1), None);
    }

    #[test]
    fn test_all_card_types() {
        let types = vec![
            CardType::Character,
            CardType::Strategy,
            CardType::Item,
            CardType::Legendary,
        ];
        assert_eq!(types.len(), 4);
    }

    #[test]
    fn test_all_properties() {
        let properties = vec![Property::Rational, Property::Divine, Property::Spiritual];
        assert_eq!(properties.len(), 3);
    }

    #[test]
    fn test_all_categories() {
        let categories = vec![
            Category::Math,
            Category::Science,
            Category::Literature,
            Category::Philosophy,
            Category::Mystery,
        ];
        assert_eq!(categories.len(), 5);
    }

    #[test]
    fn test_strategy_kinds() {
        let kinds = vec![
            StrategyKind::Normal,
            StrategyKind::Trick,
            StrategyKind::Instant,
        ];
        assert_eq!(kinds.len(), 3);
    }

    #[test]
    fn test_item_kinds() {
        let kinds = vec![ItemKind::Normal, ItemKind::Persistent];
        assert_eq!(kinds.len(), 2);
    }

    #[test]
    fn test_zone_variants() {
        let zones = vec![
            Zone::Deck,
            Zone::Hand,
            Zone::Front(0),
            Zone::Back(0),
            Zone::CostZone,
            Zone::Grave,
        ];
        assert_eq!(zones.len(), 6);
    }
}
