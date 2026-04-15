#!/usr/bin/env python3
"""Test script for two-player game start flow."""

import asyncio
import json
import websockets

SERVER_URL = "ws://localhost:8080/ws"
ROOM_ID = "test_room_001"

# Sample deck for testing
TEST_DECK = {
    "deck_id": "Example",
    "cards": ["S000-C-001", "S000-C-001", "S000-C-002", "S000-C-002",
              "S000-C-003", "S000-C-003", "S000-C-004", "S000-C-004",
              "S000-C-005", "S000-C-005", "S000-C-006", "S000-C-006",
              "S000-C-007", "S000-C-007", "S000-C-008", "S000-C-008",
              "S000-C-009", "S000-C-009", "S000-C-010", "S000-C-010"]
}


async def player_client(player_name: str, ready_event: asyncio.Event, game_started_event: asyncio.Event):
    """Simulate a player client."""
    print(f"[{player_name}] Connecting to server...")

    async with websockets.connect(SERVER_URL) as ws:
        print(f"[{player_name}] Connected!")

        # Join room
        join_msg = {
            "type": "join_room",
            "room_id": ROOM_ID,
            "player_name": player_name
        }
        await ws.send(json.dumps(join_msg))
        print(f"[{player_name}] Sent join_room message")

        # Wait for messages
        opponent_joined = False

        while True:
            try:
                msg = await asyncio.wait_for(ws.recv(), timeout=10.0)
                data = json.loads(msg)
                msg_type = data.get("type")

                print(f"[{player_name}] Received: {msg_type}")

                if msg_type == "joined":
                    print(f"[{player_name}] Successfully joined room!")
                    # Signal that this player has joined
                    ready_event.set()

                elif msg_type == "opponent_joined":
                    opponent_name = data.get("player_name", "Unknown")
                    print(f"[{player_name}] Opponent joined: {opponent_name}")
                    opponent_joined = True

                    # Now we can submit our deck
                    await submit_deck(ws, player_name)

                elif msg_type == "player_ready":
                    ready_player = data.get("player_name", "Unknown")
                    deck_id = data.get("deck_id", "Unknown")
                    print(f"[{player_name}] Player ready: {ready_player} with deck {deck_id}")

                elif msg_type == "game_starting":
                    print(f"[{player_name}] Game is starting...")

                elif msg_type == "game_started":
                    print(f"[{player_name}] GAME STARTED!")
                    print(f"[{player_name}] Game data: {json.dumps(data.get('state', {}), indent=2)}")
                    game_started_event.set()
                    return True

                elif msg_type == "room_state":
                    players = data.get("players", [])
                    all_ready = data.get("all_ready", False)
                    print(f"[{player_name}] Room state: {len(players)} players, all_ready={all_ready}")
                    for p in players:
                        print(f"  - {p.get('name')}: ready={p.get('is_ready')}, deck={p.get('deck_id')}")

                elif msg_type == "error":
                    print(f"[{player_name}] ERROR: {data.get('message')}")
                    return False

            except asyncio.TimeoutError:
                print(f"[{player_name}] Timeout waiting for message")
                return False


async def submit_deck(ws, player_name: str):
    """Submit deck to server."""
    deck_msg = {
        "type": "submit_deck",
        "deck_id": TEST_DECK["deck_id"],
        "cards": TEST_DECK["cards"]
    }
    await ws.send(json.dumps(deck_msg))
    print(f"[{player_name}] Submitted deck: {TEST_DECK['deck_id']} ({len(TEST_DECK['cards'])} cards)")


async def main():
    """Run two-player test."""
    print("=" * 60)
    print("Two-Player Game Start Flow Test")
    print("=" * 60)

    # Events for synchronization
    player1_ready = asyncio.Event()
    player2_ready = asyncio.Event()
    game_started = asyncio.Event()

    # Create tasks for both players
    task1 = asyncio.create_task(
        player_client("Player1", player1_ready, game_started)
    )

    # Wait for player1 to join, then start player2
    print("\n[Main] Waiting for Player1 to join...")
    await player1_ready.wait()
    print("[Main] Player1 joined, starting Player2...")

    # Small delay to ensure proper ordering
    await asyncio.sleep(0.5)

    task2 = asyncio.create_task(
        player_client("Player2", player2_ready, game_started)
    )

    # Wait for game to start or timeout
    print("\n[Main] Waiting for game to start...")
    try:
        await asyncio.wait_for(game_started.wait(), timeout=15.0)
        print("\n" + "=" * 60)
        print("SUCCESS! Game started for both players!")
        print("=" * 60)
    except asyncio.TimeoutError:
        print("\n" + "=" * 60)
        print("TIMEOUT! Game did not start within 15 seconds")
        print("=" * 60)

    # Cancel any remaining tasks
    task1.cancel()
    task2.cancel()

    try:
        await task1
        await task2
    except asyncio.CancelledError:
        pass

    print("\nTest complete!")


if __name__ == "__main__":
    asyncio.run(main())
