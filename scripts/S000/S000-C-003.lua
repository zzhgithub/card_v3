return {
    id = "S000-C-003",
    name = "守护者",
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
