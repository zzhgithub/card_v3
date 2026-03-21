return {
    id = "S000-S-003",
    name = "灵光一现",
    card_type = "Strategy",
    strategy_kind = "Instant",
    property = "Rational",
    category = "Literature",
    cost = 2,
    tags = {},
    effects = {
        e1 = {
            trigger = "BothMainPhase",
            optional = false,
            actions = {
                { type = "Draw", player = "Self_", count = 2 }
            }
        }
    }
}
