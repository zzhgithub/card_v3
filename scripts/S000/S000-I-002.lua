return {
    id = "S000-I-002",
    name = "生命之泉",
    card_type = "Item",
    item_kind = "Persistent",
    property = "Divine",
    category = "Mystery",
    cost = 3,
    tags = {},
    effects = {
        e1 = {
            trigger = "TurnStart",
            optional = false,
            actions = {
                { type = "HealHp", player = "Self_", amount = 1 }
            }
        }
    }
}
