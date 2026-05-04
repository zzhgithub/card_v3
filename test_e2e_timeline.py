#!/usr/bin/env python3
"""
端到端双客户端交互时间线测试。
模拟两个玩家交替回合的完整流程，记录所有收发消息，
最后生成 HTML 报告显示游戏过程中的完整消息流。

流程：
  P1(T1): Main1登最便宜卡 → Battle不攻击 → Main2跳过 → End
  P2(T1): Main1登最便宜卡 → Battle不攻击 → Main2跳过 → End
  P1(T2): Recovery回收卡 → Main1跳过 → Battle攻击 → Main2跳过 → 断开
"""

import asyncio
import json
import os
import random
import sys
import time
import websockets

SERVER_URL = "ws://127.0.0.1:8080/ws"  # use IPv4 to avoid Java IPv6 conflict
ROOM_ID = f"e2e_test_{random.randint(1000,9999)}"
SCRIPTS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "scripts_json")

# 40-card deck：5 种链测试卡 ×3，其他卡均不超过3张
BASE_40 = (
    ["S000-S-004"] * 3 + ["S000-S-005"] * 3 + ["S000-S-006"] * 3
    + ["S000-I-003"] * 3 + ["S000-C-011"] * 3                          # 15 链测试卡
    + ["S000-C-002"] * 3 + ["S000-C-001"] * 3 + ["S000-C-010"] * 3    # 9
    + ["S000-S-001"] * 3 + ["S000-S-002"] * 3 + ["S000-I-001"] * 3    # 9
    + ["S000-C-003"] * 2 + ["S000-C-004"] * 2                          # 4
    + ["S000-C-005"] * 1 + ["S000-C-006"] * 1 + ["S000-C-007"] * 1    # 3 → total 40
)

# ---------------------------------------------------------------------------
# 卡片费用缓存：从本地 JSON 加载
# ---------------------------------------------------------------------------
_card_costs = {}

def load_card_costs():
    """从 scripts_json 目录加载所有卡片的费用。"""
    global _card_costs
    if not os.path.isdir(SCRIPTS_DIR):
        print(f"WARNING: scripts_json not found at {SCRIPTS_DIR}")
        return
    for set_dir in sorted(os.listdir(SCRIPTS_DIR)):
        set_path = os.path.join(SCRIPTS_DIR, set_dir)
        if not os.path.isdir(set_path):
            continue
        for fn in sorted(os.listdir(set_path)):
            if fn.endswith(".json"):
                try:
                    with open(os.path.join(set_path, fn)) as f:
                        d = json.load(f)
                    _card_costs[d["id"]] = {
                        "cost": d.get("cost", 0),
                        "name": d.get("name", d["id"]),
                        "attack": d.get("attack"),
                        "card_type": d.get("card_type", ""),
                    }
                except Exception:
                    pass
    print(f"Loaded costs for {len(_card_costs)} cards")

def card_cost(card_id: str) -> int:
    return _card_costs.get(card_id, {}).get("cost", 99)

def card_name(card_id: str) -> str:
    return _card_costs.get(card_id, {}).get("name", card_id)

# ---------------------------------------------------------------------------
# 消息日志
# ---------------------------------------------------------------------------
class MessageLog:
    """记录一条消息，含方向、时间戳、内容和注释。"""
    __slots__ = ("ts", "player", "direction", "msg_type", "data", "note")
    def __init__(self, player: str, direction: str, msg_type: str, data: dict, note: str = ""):
        self.ts = time.time()
        self.player = player
        self.direction = direction  # "SEND" or "RECV"
        self.msg_type = msg_type
        self.data = data
        self.note = note

    def to_dict(self):
        return {
            "ts": self.ts, "player": self.player, "direction": self.direction,
            "msg_type": self.msg_type, "data": self.data, "note": self.note,
        }


class PlayerState:
    """追踪一个玩家的最新可见状态。"""
    def __init__(self):
        self.turn = 0
        self.phase = ""
        self.current_player = ""
        self.my_hp = 5
        self.my_rp = 0
        self.my_hand = []       # [{instance_id, definition_id, current_attack}, ...]
        self.my_front = [None]*5
        self.my_back = [None]*5
        self.my_cost_zone = []
        self.my_grave = []
        self.my_deck_count = 0
        self.opp_hp = 5
        self.opp_rp = 0
        self.opp_hand_count = 0
        self.opp_front = [None]*5
        self.opp_back = [None]*5
        self.opp_cost_zone = []
        self.opp_grave = []
        self.opp_deck_count = 0
        self.recent_events = []

    def update(self, state: dict):
        if not state:
            return
        self.turn = state.get("turn_number", self.turn)
        self.phase = state.get("current_phase", self.phase)
        self.current_player = state.get("current_player", self.current_player)
        ys = state.get("your_state", {})
        if ys:
            self.my_hp = ys.get("hp", self.my_hp)
            self.my_rp = ys.get("real_point", self.my_rp)
            self.my_deck_count = ys.get("deck_count", self.my_deck_count)
            self.my_hand = ys.get("hand", self.my_hand)
            self.my_front = ys.get("front", self.my_front)
            self.my_back = ys.get("back", self.my_back)
            self.my_cost_zone = ys.get("cost_zone", self.my_cost_zone)
            self.my_grave = ys.get("grave", self.my_grave)
        os_ = state.get("opponent_state", {})
        if os_:
            self.opp_hp = os_.get("hp", self.opp_hp)
            self.opp_rp = os_.get("real_point", self.opp_rp)
            self.opp_deck_count = os_.get("deck_count", self.opp_deck_count)
            self.opp_hand_count = os_.get("hand_count", self.opp_hand_count)
            self.opp_front = os_.get("front", self.opp_front)
            self.opp_back = os_.get("back", self.opp_back)
            self.opp_cost_zone = os_.get("cost_zone", self.opp_cost_zone)
            self.opp_grave = os_.get("grave", self.opp_grave)
        self.recent_events = state.get("recent_events", [])

    @property
    def is_my_turn(self):
        return self.current_player == "Player1"  # simplified for test


# ---------------------------------------------------------------------------
# 玩家客户端
# ---------------------------------------------------------------------------
class PlayerClient:
    def __init__(self, name: str, player_id: str, deck: list, log: list, role: dict):
        self.name = name
        self.player_id = player_id  # "Player1" or "Player2"
        self.deck = deck
        self.log: list[MessageLog] = log
        self.role = role             # shared dict: {"turn_plan": ..., "turn_count": int}
        self.state = PlayerState()
        self.ws = None
        self._action_queue = asyncio.Queue()
        self._recovery_queue = asyncio.Queue()

    def add_log(self, direction: str, msg_type: str, data: dict, note: str = ""):
        self.log.append(MessageLog(self.name, direction, msg_type, data, note))

    async def connect_and_join(self):
        self.ws = await websockets.connect(SERVER_URL)
        self.add_log("SEND", "join_room", {"room_id": ROOM_ID, "player_name": self.name},
                      "加入房间")
        await self.ws.send(json.dumps({"type": "join_room", "room_id": ROOM_ID,
                                        "player_name": self.name}))

    async def submit_deck(self):
        msg = {"type": "submit_deck", "deck_id": "TestDeck", "cards": self.deck}
        self.add_log("SEND", "submit_deck", {"deck_id": "TestDeck", "cards_count": len(self.deck)},
                      "提交卡组")
        await self.ws.send(json.dumps(msg))

    def _find_cheapest_play(self, actions: list) -> dict | None:
        """在 available_actions 中找到费用最低的 PlayCard。"""
        best = None
        best_cost = 999
        for a in actions:
            if a.get("action_type") != "play_card":
                continue
            iid = a.get("instance_id", 0)
            # 从本地状态找这张卡的 definition_id
            def_id = ""
            for c in self.state.my_hand:
                if c.get("instance_id") == iid:
                    def_id = c.get("definition_id", "")
                    break
            cost = card_cost(def_id)
            if cost < best_cost:
                best_cost = cost
                best = a
        return best

    def _compute_cost_payment(self, instance_id: int, card_cost: int) -> dict:
        """计算出牌的费用支付：优先 RP，不够用手牌。"""
        rp = min(card_cost, self.state.my_rp)
        need_hand = card_cost - rp
        hand_cards = []
        for c in self.state.my_hand:
            if c.get("instance_id") == instance_id:
                continue  # 不能用自己的手牌支付自己
            if len(hand_cards) >= need_hand:
                break
            hand_cards.append(c.get("instance_id"))
        return {"hand_cards": hand_cards, "real_point": rp}

    def _find_attack_targets(self, actions: list) -> list:
        """收集所有可用的攻击目标 slot。"""
        targets = []
        for a in actions:
            if a.get("action_type") != "declare_attack":
                continue
            tgt = a.get("target", {})
            if tgt.get("target_type") == "slot":
                targets.append(tgt.get("slot_index"))
        return targets

    async def handle_action_request(self, available_actions: list, timeout: int):
        """根据当前阶段和预设计划选择行动。"""
        turn = self.state.turn
        plan_map = self.role.get("plan_map", {})
        plan = plan_map.get(turn, {})
        phase = self.state.phase
        # Scan available action types for decision-making
        action_types = {}
        has_play_card = False
        has_attack = False
        has_activate_effect = False
        for a in available_actions:
            at = a.get("action_type", "?")
            action_types[at] = action_types.get(at, 0) + 1
            if at == "play_card": has_play_card = True
            if at == "declare_attack": has_attack = True
            if at == "activate_effect": has_activate_effect = True
        turn_count = self.role.get("turn_count", 0)

        self.add_log("RECV", "action_request",
                      {"available_actions": available_actions, "timeout_secs": timeout,
                       "phase": phase, "turn": self.state.turn},
                      f"收到行动请求 (阶段={phase}, 回合={self.state.turn})")

        # ── Main1 / Main2: 登场或跳过（根据 available_actions 判断，不依赖 phase）──
        if has_play_card:
            # 检查计划：本轮是否要登场
            should_play = plan.get("play_card", False)
            if should_play:
                play = self._find_cheapest_play(available_actions)
                if play:
                    iid = play["instance_id"]
                    zone = play["target_zone"]
                    # 确定卡牌费用
                    def_id = ""
                    for c in self.state.my_hand:
                        if c.get("instance_id") == iid:
                            def_id = c.get("definition_id", "")
                            break
                    cost = card_cost(def_id)
                    payment = self._compute_cost_payment(iid, cost)
                    self.add_log("SEND", "action",
                                  {"action_type": "play_card", "instance_id": iid,
                                   "target_zone": zone, "cost_payment": payment,
                                   "card_name": card_name(def_id), "card_cost": cost},
                                  f"登场 {card_name(def_id)} (费用={cost}, RP={payment['real_point']}, 手牌={payment['hand_cards']})")
                    await self.ws.send(json.dumps({
                        "type": "action", "action": {
                            "action_type": "play_card", "instance_id": iid,
                            "target_zone": zone, "cost_payment": payment,
                        }
                    }))
                    return
            # 不登场 → 检查是否有可发动的效果（链测试）
            if has_activate_effect:
                # 找一个 activate_effect 动作
                for a in available_actions:
                    if a.get("action_type") == "activate_effect":
                        iid = a.get("instance_id", 0)
                        ekey = a.get("effect_key", "")
                        self.add_log("SEND", "action",
                                      {"action_type": "activate_effect", "instance_id": iid,
                                       "effect_key": ekey},
                                      f"发动效果: instance_id={iid}, effect_key={ekey}")
                        await self.ws.send(json.dumps({
                            "type": "action", "action": {
                                "action_type": "activate_effect",
                                "instance_id": iid,
                                "effect_key": ekey,
                                "cost_payment": {"hand_cards": [], "real_point": 0},
                            }
                        }))
                        return
            # 真的没有可做的 → Pass
            self.add_log("SEND", "action", {"action_type": "pass"}, "跳过主要阶段")
            await self.ws.send(json.dumps(
                {"type": "action", "action": {"action_type": "pass"}}))
            return

        # ── Battle: 攻击或跳过（根据 available_actions 判断）──
        if has_attack:
            should_attack = plan.get("attack", False)
            if should_attack:
                targets = self._find_attack_targets(available_actions)
                if targets:
                    slot = targets[0]  # 攻击第一个可用目标
                    # 从 state 找到 attacker instance_id
                    attacker_id = 0
                    for a in available_actions:
                        if a.get("action_type") == "declare_attack":
                            attacker_id = a.get("attacker", a.get("attacker_id", 0))
                            break
                    self.add_log("SEND", "action",
                                  {"action_type": "declare_attack", "attacker_id": attacker_id,
                                   "target": {"target_type": "slot", "slot_index": slot}},
                                  f"攻击宣言: attacker={attacker_id} → 对手前场 slot={slot}")
                    await self.ws.send(json.dumps({
                        "type": "action", "action": {
                            "action_type": "declare_attack", "attacker_id": attacker_id,
                            "target": {"target_type": "slot", "slot_index": slot},
                        }
                    }))
                    return
            self.add_log("SEND", "action", {"action_type": "pass"}, "跳过战斗")
            await self.ws.send(json.dumps(
                {"type": "action", "action": {"action_type": "pass"}}))
            return

        # ── Fallback ──
        self.add_log("SEND", "action", {"action_type": "pass"}, f"自动跳过 (阶段={phase})")
        await self.ws.send(json.dumps(
            {"type": "action", "action": {"action_type": "pass"}}))

    async def handle_recovery_request(self, count: int, options: list):
        self.add_log("RECV", "recovery_request",
                      {"count": count, "options": options},
                      f"回收请求: 需选择 {count} 张卡从费用区回收 (options={len(options)})")
        # 选择 min(count, len(options)) 张
        n = min(count, len(options))
        selected = [o["instance_id"] for o in options[:n]] if n > 0 else []
        self.add_log("SEND", "recovery",
                      {"cards": selected},
                      f"回收选择: {selected}")
        await self.ws.send(json.dumps({"type": "recovery", "cards": selected}))

    async def listen_loop(self):
        """主消息循环：接收并处理服务器消息直到游戏结束或断开。"""
        plan_map = self.role.get("plan_map", {})

        try:
            while True:
                raw = await asyncio.wait_for(self.ws.recv(), timeout=30.0)
                data = json.loads(raw)
                msg_type = data.get("type", "")

                if msg_type == "state_update":
                    st = data.get("state", {})
                    if st:
                        self.state.update(st)
                    # 提取手牌详情
                    hand_ids = [c.get("definition_id","?") for c in self.state.my_hand]
                    opp_hand = self.state.opp_hand_count
                    events = self.state.recent_events
                    evt_summary = ""
                    if events:
                        evt_types = []
                        for e in events:
                            if isinstance(e, dict):
                                evt_types.append(list(e.keys())[0] if e else "?")
                        evt_summary = f" events={evt_types}"
                    self.add_log("RECV", "state_update",
                                  st,  # full state JSON as data
                                  f"状态更新 (阶段={self.state.phase}, 手牌={len(self.state.my_hand)}张={hand_ids}, 对手手牌={opp_hand}张, RP={self.state.my_rp})")

                    # 根据回合编号检查停止条件
                    turn = self.state.turn
                    plan = plan_map.get(turn, {})
                    stop_after = plan.get("stop_after", "")
                    stop_turn = plan.get("stop_turn", 99)
                    if stop_after and self.state.phase == stop_after and turn >= stop_turn:
                        self.role["should_stop"] = True

                elif msg_type == "action_request":
                    actions = data.get("available_actions", [])
                    timeout = data.get("timeout_secs", 60)
                    await self.handle_action_request(actions, timeout)

                elif msg_type == "recovery_request":
                    count = data.get("count", 0)
                    options = data.get("options", [])
                    await self.handle_recovery_request(count, options)

                elif msg_type == "game_started":
                    self.add_log("RECV", "game_started", {}, "游戏开始通知（纯通知）")

                elif msg_type == "chain_action_request":
                    chain_size = data.get("chain_size", 0)
                    effects = data.get("available_effects", [])
                    timeout = data.get("timeout_secs", 0)
                    self.add_log("RECV", "chain_action_request",
                                  {"chain_size": chain_size, "available_effects": effects,
                                   "timeout_secs": timeout},
                                  f"连锁窗口请求: chain_size={chain_size}, effects={len(effects)}")
                    # Auto-pass for test (no chaining needed)
                    await self.ws.send(json.dumps({
                        "type": "action",
                        "action": {"action_type": "chain_pass"}
                    }))

                elif msg_type == "game_starting":
                    self.add_log("RECV", "game_starting", {}, "游戏即将开始")

                elif msg_type == "player_ready":
                    self.add_log("RECV", "player_ready", data, f"{data.get('player_name')} 已准备")

                elif msg_type == "player_disconnected":
                    self.add_log("RECV", "player_disconnected", data, f"{data.get('player_name')} 断线")

                elif msg_type == "opponent_joined":
                    self.add_log("RECV", "opponent_joined", data, f"对手 {data.get('player_name')} 加入")

                elif msg_type == "waiting_for_deck":
                    self.add_log("RECV", "waiting_for_deck", {}, "等待提交卡组")

                elif msg_type == "joined":
                    pid = data.get("player_id", "")
                    self.add_log("RECV", "joined", data, f"加入房间成功 player_id={pid}")
                    await self.submit_deck()

                elif msg_type == "room_state":
                    self.add_log("RECV", "room_state", data, "房间状态更新")

                elif msg_type == "game_over":
                    self.add_log("RECV", "game_over", data,
                                  f"游戏结束! winner={data.get('winner')} reason={data.get('reason')}")
                    self.role["game_over"] = True
                    break

                elif msg_type == "error":
                    self.add_log("RECV", "error", data, f"服务器错误: {data.get('message')}")

                else:
                    self.add_log("RECV", msg_type, data, "")

                # 检查停止条件
                if self.role.get("should_stop"):
                    self.add_log("SEND", "__disconnect__", {}, "主动断开连接")
                    break

        except asyncio.TimeoutError:
            self.add_log("RECV", "__timeout__", {}, "接收超时")
        except websockets.ConnectionClosed:
            self.add_log("RECV", "__closed__", {}, "连接关闭")


# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------
async def main():
    load_card_costs()

    # 共享日志
    all_logs: list[MessageLog] = []

    # Player1 的计划：T1 Main1 登场最便宜的卡，Battle 不攻击；T2 Recovery 回收卡 + Battle 攻击，Main2 后断开
    p1_plan = {
        1: {"play_card": True, "attack": False, "chain_test": True, "stop_after": "", "stop_turn": 99},
        3: {"play_card": False, "attack": True, "chain_test": False, "stop_after": "Main2", "stop_turn": 3},
    }
    p1_role = {"plan_map": p1_plan, "turn_count": 0, "should_stop": False, "game_over": False}
    p2_plan = {
        2: {"play_card": True, "attack": False, "chain_test": True, "stop_after": "", "stop_turn": 99},
    }
    p2_role = {"plan_map": p2_plan, "turn_count": 0, "should_stop": False, "game_over": False}

    # 根据回合动态更新计划
    # 我们在 state_update 中检测回合变化来更新 turn_plan
    p1_role["plan_map"] = p1_plan
    p2_role["plan_map"] = p2_plan

    p1 = PlayerClient("Alice(P1)", "Player1", BASE_40, all_logs, p1_role)
    p2 = PlayerClient("Bob(P2)", "Player2", BASE_40, all_logs, p2_role)

    # 阶段 1：连接和加入
    print("=" * 60)
    print("E2E Timeline Test")
    print("=" * 60)

    print("\n[Setup] Connecting players...")
    await p1.connect_and_join()
    await asyncio.sleep(0.3)
    await p2.connect_and_join()
    await asyncio.sleep(1.0)

    # 启动两个客户端的消息循环
    async def run_player(client: PlayerClient):
        await client.listen_loop()
        # 在每次 state_update 后更新计划
        # 我们用一个包装器来处理

    # 需要动态切换计划：每个客户端的 listen_loop 需要在 state_update 中检测回合变化
    # 简化方案：在 listen_loop 中用 last_turn 检测回合切换
    # 实际已经在 PlayerClient 中实现了基础逻辑

    # 手动运行顺序：先让两个客户端并行运行
    # p1 先运行到第一个 state_update，然后 p2 启动
    # 实际可以并行，因为 action_request 只发给对应玩家

    task1 = asyncio.create_task(p1.listen_loop())
    await asyncio.sleep(0.5)
    task2 = asyncio.create_task(p2.listen_loop())

    # 等待两个任务完成或超时
    done, pending = await asyncio.wait([task1, task2], timeout=120.0)

    for t in pending:
        t.cancel()

    print(f"\n[Done] Collected {len(all_logs)} message log entries.")

    # 关闭 WebSocket
    for c in [p1, p2]:
        try:
            if c.ws:
                await c.ws.close()
        except Exception:
            pass

    # 生成 HTML 报告
    generate_html(all_logs, p1.state, p2.state)


# ---------------------------------------------------------------------------
# HTML 报告生成
# ---------------------------------------------------------------------------
def generate_html(logs: list, state1: PlayerState, state2: PlayerState):
    html_path = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                              "docs", "e2e_timeline_report.html")
    os.makedirs(os.path.dirname(html_path), exist_ok=True)

    rows = []
    for i, m in enumerate(logs):
        d = m.to_dict()
        direction_cls = "send" if m.direction == "SEND" else "recv"
        player_cls = "p1" if "P1" in m.player or "Alice" in m.player else "p2"
        ts_str = f"{d['ts'] - logs[0].ts:.3f}s" if logs else "0.000s"

        data_str = json.dumps(d["data"], ensure_ascii=False, indent=2)

        rows.append(f"""
        <tr class="{player_cls}">
          <td class="idx">{i+1}</td>
          <td class="ts">{ts_str}</td>
          <td class="player">{d['player']}</td>
          <td class="dir {direction_cls}">{d['direction']}</td>
          <td class="type">{d['msg_type']}</td>
          <td class="note">{d['note']}</td>
          <td class="data"><pre>{data_str}</pre></td>
        </tr>""")

    html = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>E2E 交互时间线测试报告</title>
<style>
:root {{ --bg: #0d1117; --card: #161b22; --border: #30363d; --text: #c9d1d9;
  --muted: #8b949e; --accent: #58a6ff; --green: #3fb950; --orange: #d2991d;
  --send: #1c283a; --recv: #1c2128; --p1-bg: rgba(63,185,80,.04); --p2-bg: rgba(210,153,29,.04); }}
* {{ margin:0; padding:0; box-sizing:border-box; }}
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: var(--bg); color: var(--text); }}
header {{ background: linear-gradient(135deg, #1a2332, #0d1117); border-bottom:1px solid var(--border); padding:24px; text-align:center; position:sticky; top:0; z-index:100; }}
header h1 {{ font-size:20px; }}
header p {{ color:var(--muted); font-size:13px; margin-top:4px; }}
.summary {{ display:flex; gap:12px; justify-content:center; margin-top:16px; flex-wrap:wrap; }}
.summary .stat {{ background:var(--card); border:1px solid var(--border); border-radius:8px; padding:12px 20px; text-align:center; min-width:100px; }}
.summary .stat .num {{ font-size:24px; font-weight:700; color:var(--accent); }}
.summary .stat .label {{ font-size:11px; color:var(--muted); }}
main {{ max-width:1400px; margin:0 auto; padding:20px; }}
table {{ width:100%; border-collapse:collapse; font-size:12px; }}
th {{ background:#21262d; padding:8px 10px; text-align:left; font-weight:600; font-size:11px; text-transform:uppercase; position:sticky; top:120px; z-index:10; }}
td {{ padding:8px 10px; border-top:1px solid var(--border); vertical-align:top; }}
tr.p1 td {{ background:var(--p1-bg); }}
tr.p2 td {{ background:var(--p2-bg); }}
.dir {{ font-weight:700; font-size:11px; }}
.send {{ color:var(--accent); }}
.recv {{ color:var(--green); }}
.idx {{ width:40px; text-align:center; color:var(--muted); }}
.ts {{ width:70px; color:var(--muted); font-family:monospace; }}
.player {{ width:90px; }}
.dir {{ width:55px; }}
.type {{ width:130px; font-family:monospace; font-weight:600; }}
.note {{ max-width:300px; font-size:12px; }}
.data {{ max-width:350px; }}
.data pre {{ font-size:10px; color:var(--muted); max-height:120px; overflow-y:auto; white-space:pre-wrap; word-break:break-all; }}
.legend {{ display:flex; gap:16px; margin-bottom:16px; font-size:12px; }}
.legend span {{ display:flex; align-items:center; gap:6px; }}
.legend .dot {{ width:10px; height:10px; border-radius:50%; }}
.legend .dot.p1-dot {{ background:var(--green); }}
.legend .dot.p2-dot {{ background:var(--orange); }}
.legend .dot.send-dot {{ background:var(--accent); }}
.legend .dot.recv-dot {{ background:var(--green); }}
.phase-marker {{ background:#1a2332; text-align:center; font-weight:700; font-size:13px; padding:12px; }}
.toolbar {{ display:flex; gap:8px; margin-bottom:12px; flex-wrap:wrap; }}
.toolbar button {{ padding:6px 14px; border:1px solid var(--border); border-radius:6px; background:var(--card); color:var(--text); cursor:pointer; font-size:12px; }}
.toolbar button:hover {{ background:var(--accent); border-color:var(--accent); color:#fff; }}
</style>
</head>
<body>
<header>
  <h1>E2E 交互时间线测试报告</h1>
  <p>双客户端模拟完整回合流程 &middot; 消息收发顺序 &middot; 数据内容</p>
  <div class="summary">
    <div class="stat"><div class="num">{len(logs)}</div><div class="label">消息总量</div></div>
    <div class="stat"><div class="num">{sum(1 for m in logs if m.direction == 'SEND')}</div><div class="label">客户端发送</div></div>
    <div class="stat"><div class="num">{sum(1 for m in logs if m.direction == 'RECV')}</div><div class="label">客户端接收</div></div>
  </div>
</header>
<main>
<div class="legend">
  <span><span class="dot p1-dot"></span> Alice (P1)</span>
  <span><span class="dot p2-dot"></span> Bob (P2)</span>
  <span><span class="dot send-dot"></span> SEND (上行)</span>
  <span><span class="dot recv-dot"></span> RECV (下行)</span>
</div>
<div class="toolbar">
  <button onclick="filterAll()">全部</button>
  <button onclick="filter('p1')">仅 P1 (Alice)</button>
  <button onclick="filter('p2')">仅 P2 (Bob)</button>
  <button onclick="filter('send')">仅 SEND</button>
  <button onclick="filter('recv')">仅 RECV</button>
  <button onclick="filter('action')">仅 action_request</button>
  <button onclick="filter('state')">仅 state_update</button>
</div>
<table id="log-table">
<thead><tr>
  <th>#</th><th>时间</th><th>玩家</th><th>方向</th><th>消息类型</th><th>注释</th><th>数据</th>
</tr></thead>
<tbody>
{''.join(rows)}
</tbody>
</table>
</main>
<script>
function filter(t) {{
  document.querySelectorAll('#log-table tbody tr').forEach(tr => {{
    if (t === 'all') {{ tr.style.display = ''; return; }}
    if (t === 'p1') {{ tr.style.display = tr.classList.contains('p1') ? '' : 'none'; return; }}
    if (t === 'p2') {{ tr.style.display = tr.classList.contains('p2') ? '' : 'none'; return; }}
    if (t === 'send') {{ tr.style.display = tr.querySelector('.dir.send') ? '' : 'none'; return; }}
    if (t === 'recv') {{ tr.style.display = tr.querySelector('.dir.recv') ? '' : 'none'; return; }}
    if (t === 'action') {{ tr.style.display = tr.querySelector('.type').textContent.includes('action_request') ? '' : 'none'; return; }}
    if (t === 'state') {{ tr.style.display = tr.querySelector('.type').textContent.includes('state_update') ? '' : 'none'; return; }}
  }});
}}
function filterAll(){{ filter('all'); }}
</script>
</body>
</html>"""

    with open(html_path, "w", encoding="utf-8") as f:
        f.write(html)
    print(f"\n[Report] HTML report written to: {html_path}")
    return html_path


if __name__ == "__main__":
    asyncio.run(main())
