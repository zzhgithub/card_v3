#!/usr/bin/env python3
"""Test two-player game start flow."""

import asyncio
import json
import websockets

SERVER_URL = "ws://localhost:8080/ws"
ROOM_ID = "1"

TEST_DECK = {
    "deck_id": "Example",
    # 40 cards total using the available cards
    "cards": [
        # 3 copies each of 13 different cards = 39 cards
        "S000-C-001", "S000-C-001", "S000-C-001",
        "S000-C-002", "S000-C-002", "S000-C-002",
        "S000-C-003", "S000-C-003", "S000-C-003",
        "S000-C-004", "S000-C-004", "S000-C-004",
        "S000-C-005", "S000-C-005", "S000-C-005",
        "S000-C-006", "S000-C-006", "S000-C-006",
        "S000-C-007", "S000-C-007", "S000-C-007",
        "S000-C-008", "S000-C-008", "S000-C-008",
        "S000-C-009", "S000-C-009", "S000-C-009",
        "S000-C-010", "S000-C-010", "S000-C-010",
        "S000-S-001", "S000-S-001", "S000-S-001",
        "S000-S-002", "S000-S-002", "S000-S-002",
        "S000-S-003", "S000-S-003", "S000-S-003",
        # Plus 1 more card to reach 40
        "S000-I-001"
    ]
}


async def player_client(player_name: str, is_second: bool = False):
    """Simulate a player client."""
    print(f"[{player_name}] Connecting...")

    # Disable any proxy settings
    import os
    for key in list(os.environ.keys()):
        if 'proxy' in key.lower():
            del os.environ[key]

    async with websockets.connect(SERVER_URL, proxy=None) as ws:
        print(f"[{player_name}] Connected!")

        # Join room
        await ws.send(json.dumps({
            "type": "join_room",
            "room_id": ROOM_ID,
            "player_name": player_name
        }))
        print(f"[{player_name}] Sent join_room")

        game_started_received = False

        while True:
            try:
                msg = await asyncio.wait_for(ws.recv(), timeout=15.0)
                data = json.loads(msg)
                msg_type = data.get("type")

                print(f"[{player_name}] Received: {msg_type}")

                if msg_type == "joined":
                    print(f"[{player_name}] Joined room as {data.get('player_id')}")

                elif msg_type == "room_state":
                    players = data.get("players", [])
                    all_ready = data.get("all_ready", False)
                    print(f"[{player_name}] Room has {len(players)} players, all_ready={all_ready}")
                    for p in players:
                        print(f"  - {p.get('name')}: ready={p.get('is_ready')}")

                    # If second player and room has 2 players, submit deck
                    if is_second and len(players) == 2:
                        await asyncio.sleep(0.5)
                        await ws.send(json.dumps({
                            "type": "submit_deck",
                            "deck_id": TEST_DECK["deck_id"],
                            "cards": TEST_DECK["cards"]
                        }))
                        print(f"[{player_name}] Submitted deck")

                elif msg_type == "opponent_joined":
                    print(f"[{player_name}] Opponent joined: {data.get('player_name')}")
                    # First player submits deck when opponent joins
                    if not is_second:
                        await asyncio.sleep(0.5)
                        await ws.send(json.dumps({
                            "type": "submit_deck",
                            "deck_id": TEST_DECK["deck_id"],
                            "cards": TEST_DECK["cards"]
                        }))
                        print(f"[{player_name}] Submitted deck")

                elif msg_type == "player_ready":
                    print(f"[{player_name}] Player ready: {data.get('player_name')} with {data.get('deck_id')}")

                elif msg_type == "game_starting":
                    print(f"[{player_name}] *** GAME STARTING ***")

                elif msg_type == "game_started":
                    print(f"[{player_name}] *** GAME STARTED! ***")
                    print(f"[{player_name}] Game state: {json.dumps(data.get('state', {}), indent=2)[:500]}...")
                    game_started_received = True
                    return True

                elif msg_type == "error":
                    print(f"[{player_name}] ERROR: {data.get('message')}")
                    return False

            except asyncio.TimeoutError:
                print(f"[{player_name}] Timeout waiting for message")
                return False


async def main():
    print("=" * 60)
    print("Two-Player Game Start Test (Player 2 only)")
    print("=" * 60)
    print("Assuming Godot client is already Player 1...")

    # Only run Player B (Player 2) since Godot is Player 1
    result = await player_client("PlayerB", is_second=True)

    print("\n" + "=" * 60)
    if result is True:
        print("SUCCESS! Player 2 received game_started!")
    else:
        print(f"Result: {result}")
    print("=" * 60)


if __name__ == "__main__":
    asyncio.run(main())
