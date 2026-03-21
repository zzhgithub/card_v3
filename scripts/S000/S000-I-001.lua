return {
    id = "S000-I-001",
    name = "能量水晶",
    card_type = "Item",
    item_kind = "Normal",
    property = "Divine",
    category = "Science",
    cost = 2,
    tags = {},
    effects = {
        e1 = {
            trigger = "OwnMainPhase",
            optional = false,
            actions = {
                { type = "GainRealPoint", player = "Self_", amount = 1 }
            }
        }
    }
}
