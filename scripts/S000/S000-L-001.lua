return {
    id = "S000-L-001",
    name = "传奇智者",
    card_type = "Legendary",
    property = "Rational",
    category = "Philosophy",
    cost = 6,
    attack = 2500,
    tags = {"传奇"},
    effects = {
        e1 = {
            trigger = "OnSummon",
            optional = false,
            actions = {
                { type = "Damage", player = "Opponent", amount = 2 }
            }
        }
    }
}
