return {
    id = "S000-C-002",
    name = "知识探索者",
    card_type = "Character",
    property = "Rational",
    category = "Science",
    cost = 4,
    attack = 1200,
    tags = {},
    effects = {
        e1 = {
            trigger = "OnSummon",
            optional = false,
            actions = {
                { type = "Draw", player = "Self_", count = 1 }
            }
        }
    }
}
