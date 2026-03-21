return {
    id = "S000-S-002",
    name = "思维混乱",
    card_type = "Strategy",
    strategy_kind = "Trick",
    property = "Spiritual",
    category = "Mystery",
    cost = 1,
    tags = {},
    effects = {
        e1 = {
            trigger = "OwnMainPhase",
            optional = false,
            actions = {
                { type = "Discard", player = "Opponent", count = 1 }
            }
        }
    }
}
