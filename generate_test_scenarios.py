#!/usr/bin/env python3
"""Generate 12 test scenario JSONs for the replay system."""

import json, os

OUT = "godot-client/assets/replay/test_scenarios"

def card(iid, def_id, atk=None):
    c = {"instance_id": iid, "definition_id": def_id}
    if atk is not None:
        c["current_attack"] = atk
    return c

def empty_front(n=5): return [None]*n
def empty_back(n=5): return [None]*n

def player_state(hp=5, rp=0, deck=35, hand=None, front=None, back=None, cost=None, grave=None):
    return {
        "hp": hp, "real_point": rp, "deck_count": deck,
        "hand": hand or [], "front": front or empty_front(),
        "back": back or empty_back(), "cost_zone": cost or [], "grave": grave or []
    }

def opponent_state(hp=5, rp=0, deck=35, hand_count=5, front=None, back=None, cost=None, grave=None):
    return {
        "hp": hp, "real_point": rp, "deck_count": deck, "hand_count": hand_count,
        "front": front or empty_front(), "back": back or empty_back(),
        "cost_zone": cost or [], "grave": grave or []
    }

def msg(state, events=None, delay=0):
    return {
        "type": "state_update",
        "data": {
            "turn_number": state.get("turn", 1),
            "current_phase": state.get("phase", "TurnStart"),
            "current_player": state.get("player", "Player1"),
            "recent_events": events or [],
            "your_state": state["your"],
            "opponent_state": state["opponent"]
        },
        "delay_ms": delay
    }


scenarios = []

# ===========================================================================
# Scenario 1: 初始状态
# ===========================================================================
scenarios.append({
    "scenario": "1. 初始状态",
    "description": "游戏开始，双方各5手牌，HP=5，卡组=35，场上全空",
    "messages": [
        msg({
            "turn": 1, "phase": "TurnStart", "player": "Player1",
            "your": player_state(hand=[
                card(1,"S000-C-001",1500), card(2,"S000-C-002",800),
                card(3,"S000-C-003",1200), card(4,"S000-S-001"),
                card(5,"S000-C-010",2000)
            ]),
            "opponent": opponent_state()
        })
    ],
    "verification": {
        "expected_checks": [
            "HAND self ADD[1,2,3,4,5] | REMOVE[] | KEEP[]",
            "FRONT self: no change",
            "BACK self: no change",
            "DECK self: 35"
        ]
    }
})

# ===========================================================================
# Scenario 2: 抽卡
# ===========================================================================
scenarios.append({
    "scenario": "2. 抽卡",
    "description": "P1 抽到 S000-C-011 (iid=6)，卡组 35→34",
    "messages": [
        msg({
            "turn": 1, "phase": "Draw", "player": "Player1",
            "your": player_state(deck=34, hand=[
                card(1,"S000-C-001",1500), card(2,"S000-C-002",800),
                card(3,"S000-C-003",1200), card(4,"S000-S-001"),
                card(5,"S000-C-010",2000), card(6,"S000-C-011",500)
            ]),
            "opponent": opponent_state()
        }, events=[
            {"PhaseChanged": {"new_phase": "TurnStart"}},
            {"PhaseChanged": {"new_phase": "Draw"}},
            {"DrawCard": {"player": "Player1", "instance_id": 6}}
        ])
    ],
    "verification": {
        "expected_checks": [
            "HAND self ADD[6] | KEEP[1,2,3,4,5]",
            "DECK self: 34"
        ]
    }
})

# ===========================================================================
# Scenario 3: 登场(前场) + 费用支付
# ===========================================================================
scenarios.append({
    "scenario": "3. 登场(前场) + 费用支付",
    "description": "P1 将 S000-C-001(iid=1) 登场到前场[0]，用 S000-C-002(iid=2) 支付1费",
    "messages": [
        msg({
            "turn": 1, "phase": "Main1", "player": "Player1",
            "your": player_state(deck=34, hand=[
                card(3,"S000-C-003",1200), card(4,"S000-S-001"),
                card(5,"S000-C-010",2000), card(6,"S000-C-011",500)
            ],
            front=[card(1,"S000-C-001",1500), None, None, None, None],
            cost=[card(2,"S000-C-002",800)]),
            "opponent": opponent_state()
        }, events=[
            {"CardExposed": {"instance_id": 2}},
            {"CardMoved": {"instance_id": 2, "from": {"player":"Player1","zone":"Hand"}, "to": {"player":"Player1","zone":"CostZone"}}},
            {"CardSummoned": {"instance_id": 1, "to": {"player":"Player1","zone":{"Front":0}}}}
        ])
    ],
    "verification": {
        "expected_checks": [
            "HAND self REMOVE[1,2] | KEEP[3,4,5,6]",
            "FRONT self ADD[1] slot=0",
            "COST self ADD[2]"
        ]
    }
})

# ===========================================================================
# Scenario 4: 登场(后场) + 费用支付
# ===========================================================================
scenarios.append({
    "scenario": "4. 登场(后场) + 费用支付",
    "description": "P1 将 S000-S-001(iid=4) 放到后场[1]，用 S000-C-003(iid=3) 支付1费",
    "messages": [
        msg({
            "turn": 1, "phase": "Main1", "player": "Player1",
            "your": player_state(deck=34, hand=[
                card(5,"S000-C-010",2000), card(6,"S000-C-011",500)
            ],
            front=[card(1,"S000-C-001",1500), None, None, None, None],
            back=[None, card(4,"S000-S-001"), None, None, None],
            cost=[card(2,"S000-C-002",800), card(3,"S000-C-003",1200)]),
            "opponent": opponent_state()
        }, events=[
            {"CardExposed": {"instance_id": 3}},
            {"CardMoved": {"instance_id": 3, "from": {"player":"Player1","zone":"Hand"}, "to": {"player":"Player1","zone":"CostZone"}}},
            {"CardSummoned": {"instance_id": 4, "to": {"player":"Player1","zone":{"Back":1}}}}
        ])
    ],
    "verification": {
        "expected_checks": [
            "HAND self REMOVE[3,4] | KEEP[5,6]",
            "BACK self ADD[4] slot=1",
            "COST self ADD[3]"
        ]
    }
})

# ===========================================================================
# Scenario 5: 战斗 — 对手卡登场
# ===========================================================================
scenarios.append({
    "scenario": "5. 对手前场登场",
    "description": "对手 P2 在前场[0] 登场 S000-C-011(iid=46, ATK=500)",
    "messages": [
        msg({
            "turn": 1, "phase": "Battle", "player": "Player1",
            "your": player_state(deck=34, hand=[
                card(5,"S000-C-010",2000), card(6,"S000-C-011",500)
            ],
            front=[card(1,"S000-C-001",1500), None, None, None, None],
            back=[None, card(4,"S000-S-001"), None, None, None],
            cost=[card(2,"S000-C-002",800), card(3,"S000-C-003",1200)]),
            "opponent": opponent_state(front=[
                card(46,"S000-C-011",500), None, None, None, None
            ])
        }, events=[{"PhaseChanged": {"new_phase": "Battle"}}])
    ],
    "verification": {
        "expected_checks": [
            "FRONT opponent ADD[46] slot=0",
            "PHASE: Battle"
        ]
    }
})

# ===========================================================================
# Scenario 6: 攻击 + 破坏
# ===========================================================================
scenarios.append({
    "scenario": "6. 攻击宣言 + 卡牌破坏 + HP变化",
    "description": "P1 用 iid=1(ATK=1500) 攻击对手 iid=46(ATK=500)，对手卡破坏，HP 5→4",
    "messages": [
        msg({
            "turn": 1, "phase": "Battle", "player": "Player1",
            "your": player_state(deck=34, hand=[
                card(5,"S000-C-010",2000), card(6,"S000-C-011",500)
            ],
            front=[card(1,"S000-C-001",1500), None, None, None, None],
            back=[None, card(4,"S000-S-001"), None, None, None],
            cost=[card(2,"S000-C-002",800), card(3,"S000-C-003",1200)],
            grave=[card(46,"S000-C-011",500)]),
            "opponent": opponent_state(hp=4, front=[None, None, None, None, None])
        }, events=[
            {"AttackDeclared": {"attacker": 1}},
            {"HpChanged": {"player": "Player2", "old_hp": 5, "new_hp": 4}},
            {"CardDestroyed": {"instance_id": 46}}
        ])
    ],
    "verification": {
        "expected_checks": [
            "FRONT opponent REMOVE[46] slot=0",
            "GRAVE self ADD[46]",
            "HP opponent: 4"
        ]
    }
})

# ===========================================================================
# Scenario 7: 自己卡被破坏
# ===========================================================================
scenarios.append({
    "scenario": "7. 自己被攻击 — 卡破坏 + HP下降",
    "description": "P2 用 iid=48(ATK=2000) 攻击 P1 iid=1(ATK=1500)，P1卡破坏，HP 5→3",
    "messages": [
        msg({
            "turn": 2, "phase": "Battle", "player": "Player2",
            "your": player_state(deck=33, rp=1, hp=3, hand=[
                card(7,"S000-S-002")
            ],
            front=[None, None, None, None, None],
            back=[None, card(4,"S000-S-001"), None, None, None],
            cost=[card(2,"S000-C-002",800), card(3,"S000-C-003",1200)],
            grave=[card(46,"S000-C-011",500), card(1,"S000-C-001",1500)]),
            "opponent": opponent_state(hp=4,
                front=[card(48,"S000-C-010",2000), None, None, None, None])
        }, events=[
            {"AttackDeclared": {"attacker": 48}},
            {"HpChanged": {"player": "Player1", "old_hp": 5, "new_hp": 3}},
            {"CardDestroyed": {"instance_id": 1}}
        ])
    ],
    "verification": {
        "expected_checks": [
            "FRONT self REMOVE[1] slot=0",
            "GRAVE self ADD[1]",
            "HP self: 3"
        ]
    }
})

# ===========================================================================
# Scenario 8: 回合切换
# ===========================================================================
scenarios.append({
    "scenario": "8. 回合切换 + P2抽卡",
    "description": "回合切换到 P2，对手抽卡 hand_count=6，Phase=Main1",
    "messages": [
        msg({
            "turn": 2, "phase": "Main1", "player": "Player2",
            "your": player_state(deck=33, rp=1, hp=3, hand=[card(7,"S000-S-002")],
                front=[None, None, None, None, None],
                back=[None, card(4,"S000-S-001"), None, None, None],
                cost=[card(2,"S000-C-002",800), card(3,"S000-C-003",1200)],
                grave=[card(46,"S000-C-011",500), card(1,"S000-C-001",1500)]),
            "opponent": opponent_state(hp=4, hand_count=6,
                front=[card(48,"S000-C-010",2000), None, None, None, None])
        }, events=[
            {"TurnChanged": {"new_active_player": "Player2", "turn_number": 2}},
            {"PhaseChanged": {"new_phase": "TurnStart"}},
            {"PhaseChanged": {"new_phase": "Draw"}},
            {"DrawCard": {"player": "Player2", "instance_id": 49}},
            {"PhaseChanged": {"new_phase": "Recovery"}},
            {"PhaseChanged": {"new_phase": "Main1"}}
        ])
    ],
    "verification": {
        "expected_checks": [
            "TURN: 2, PLAYER: Player2",
            "HAND opponent count: 6"
        ]
    }
})

# ===========================================================================
# Scenario 9: RP 变化
# ===========================================================================
scenarios.append({
    "scenario": "9. RP 变化 — 回合结束 RP+1",
    "description": "P1 回合结束进入 TurnEnd，RP 0→1",
    "messages": [
        msg({
            "turn": 1, "phase": "TurnEnd", "player": "Player1",
            "your": player_state(deck=34, rp=1, hand=[
                card(5,"S000-C-010",2000), card(6,"S000-C-011",500)
            ],
            front=[card(1,"S000-C-001",1500), None, None, None, None],
            back=[None, card(4,"S000-S-001"), None, None, None],
            cost=[card(2,"S000-C-002",800), card(3,"S000-C-003",1200)]),
            "opponent": opponent_state(hp=4)
        }, events=[
            {"PhaseChanged": {"new_phase": "Main2"}},
            {"PhaseChanged": {"new_phase": "TurnEnd"}},
            {"RealPointChanged": {"player": "Player1", "old_rp": 0, "new_rp": 1}}
        ])
    ],
    "verification": {
        "expected_checks": [
            "RP self: 1",
            "PHASE: TurnEnd"
        ]
    }
})

# ===========================================================================
# Scenario 10: GameOver
# ===========================================================================
scenarios.append({
    "scenario": "10. GameOver — DeckOut",
    "description": "DeckOut 触发 game_over，回放循环回开头",
    "messages": [
        {
            "type": "game_over",
            "data": {"type": "game_over", "winner": "Player2", "reason": "DeckOut"},
            "delay_ms": 0
        }
    ],
    "verification": {
        "expected_checks": [
            "GAME_OVER: winner=Player2, reason=DeckOut"
        ]
    }
})

# ===========================================================================
# Scenario 11: 多事件混合
# ===========================================================================
scenarios.append({
    "scenario": "11. 多事件混合",
    "description": "单次 state_update 同时包含 PhaseChanged + DrawCard + CardSummoned 三种事件",
    "messages": [
        msg({
            "turn": 3, "phase": "Main1", "player": "Player1",
            "your": player_state(deck=32, rp=2, hp=3, hand=[
                card(7,"S000-S-002"), card(8,"S000-C-001",1500)
            ],
            front=[card(9,"S000-C-011",500), None, None, None, None],
            back=[None, card(4,"S000-S-001"), None, None, None],
            cost=[card(2,"S000-C-002",800), card(3,"S000-C-003",1200)],
            grave=[card(46,"S000-C-011",500), card(1,"S000-C-001",1500)]),
            "opponent": opponent_state(hp=3, hand_count=4,
                front=[card(48,"S000-C-010",2000), None, None, None, None])
        }, events=[
            {"PhaseChanged": {"new_phase": "TurnStart"}},
            {"PhaseChanged": {"new_phase": "Draw"}},
            {"DrawCard": {"player": "Player1", "instance_id": 8}},
            {"PhaseChanged": {"new_phase": "Recovery"}},
            {"PhaseChanged": {"new_phase": "Main1"}},
            {"CardSummoned": {"instance_id": 9, "to": {"player":"Player1","zone":{"Front":0}}}}
        ])
    ],
    "verification": {
        "expected_checks": [
            "HAND self ADD[8]",
            "FRONT self ADD[9] slot=0",
            "TURN: 3, PHASE: Main1",
            "DECK self: 32",
            "RP self: 2"
        ]
    }
})

# ===========================================================================
# Scenario 12: 费用区多卡
# ===========================================================================
scenarios.append({
    "scenario": "12. 费用区多卡",
    "description": "多次费用支付后费用区有4张卡",
    "messages": [
        msg({
            "turn": 2, "phase": "Main1", "player": "Player2",
            "your": player_state(deck=33, rp=1, hp=3, hand=[card(7,"S000-S-002")],
                front=[None, None, None, None, None],
                back=[None, card(4,"S000-S-001"), None, None, None],
                cost=[
                    card(2,"S000-C-002",800), card(3,"S000-C-003",1200),
                    card(5,"S000-C-010",2000), card(6,"S000-C-011",500)
                ],
                grave=[card(46,"S000-C-011",500), card(1,"S000-C-001",1500)]),
            "opponent": opponent_state(hp=4, hand_count=6,
                front=[card(48,"S000-C-010",2000), None, None, None, None])
        }, events=[
            {"CardExposed": {"instance_id": 5}},
            {"CardMoved": {"instance_id": 5, "from": {"player":"Player1","zone":"Hand"}, "to": {"player":"Player1","zone":"CostZone"}}},
            {"CardExposed": {"instance_id": 6}},
            {"CardMoved": {"instance_id": 6, "from": {"player":"Player1","zone":"Hand"}, "to": {"player":"Player1","zone":"CostZone"}}}
        ])
    ],
    "verification": {
        "expected_checks": [
            "COST self ADD[5,6]",
            "HAND self REMOVE[5,6]"
        ]
    }
})

# ===========================================================================
# Write files
# ===========================================================================
for i, s in enumerate(scenarios):
    fname = "scenario_%02d.json" % (i + 1)
    path = os.path.join(OUT, fname)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(s, f, ensure_ascii=False, indent=2)
    print(f"Written: {fname} — {s['scenario']}")

print(f"\nTotal: {len(scenarios)} test scenarios in {OUT}")
