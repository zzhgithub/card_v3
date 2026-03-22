use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use card_core::rules::GameRules;
use card_core::types::PlayerId;
use card_protocol::codec::TcpConnection;
use card_protocol::message::{Command, NetworkMessage};
use card_script::loader::ScriptIndex;
use card_server::GameServer;
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

fn reserve_local_addr() -> SocketAddr {
    let listener = StdTcpListener::bind("127.0.0.1:0").expect("failed to reserve local addr");
    listener.local_addr().expect("failed to get local addr")
}

fn make_empty_script_index() -> ScriptIndex {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("card_server_network_game_test_{nanos}"));
    ScriptIndex::scan(Path::new(&path)).expect("failed to build empty ScriptIndex")
}

fn make_test_rules() -> GameRules {
    let mut rules = GameRules::default();
    rules.deck_size_range = (0, 60);
    rules.initial_hand_size = 0;
    rules.operation_timeout = Duration::from_millis(300);
    rules.total_game_timeout = Duration::from_secs(3);
    rules
}

async fn connect_with_retry(addr: SocketAddr) -> TcpConnection {
    let mut last_error = None;
    for _ in 0..50 {
        match TcpStream::connect(addr).await {
            Ok(stream) => return TcpConnection::from_stream(stream),
            Err(err) => {
                last_error = Some(err);
                sleep(Duration::from_millis(20)).await;
            }
        }
    }

    panic!("failed to connect to {addr}: {last_error:?}");
}

async fn complete_hello_handshake(
    conn: &mut TcpConnection,
    expected_version: &str,
    player_name: &str,
) -> PlayerId {
    let hello = timeout(Duration::from_secs(2), conn.recv())
        .await
        .expect("timed out waiting for server hello")
        .expect("failed to receive server hello");

    match hello {
        NetworkMessage::Hello {
            version,
            player_name,
        } => {
            assert_eq!(version, expected_version);
            assert_eq!(player_name, "Server");
        }
        other => panic!("expected Hello, got {other:?}"),
    }

    conn.send(&NetworkMessage::Hello {
        version: expected_version.to_string(),
        player_name: player_name.to_string(),
    })
    .await
    .expect("failed to send client hello");

    let ack = timeout(Duration::from_secs(2), conn.recv())
        .await
        .expect("timed out waiting for hello ack")
        .expect("failed to receive hello ack");

    match ack {
        NetworkMessage::HelloAck { player_id } => player_id,
        other => panic!("expected HelloAck, got {other:?}"),
    }
}

async fn expect_deck_accepted_and_game_start(conn: &mut TcpConnection) {
    let accepted = timeout(Duration::from_secs(2), conn.recv())
        .await
        .expect("timed out waiting for DeckAccepted")
        .expect("failed to receive DeckAccepted");
    assert!(matches!(accepted, NetworkMessage::DeckAccepted));

    let start = timeout(Duration::from_secs(2), conn.recv())
        .await
        .expect("timed out waiting for GameStart")
        .expect("failed to receive GameStart");

    match start {
        NetworkMessage::GameStart { rules_json } => {
            assert!(!rules_json.is_empty(), "rules_json should not be empty");
        }
        other => panic!("expected GameStart, got {other:?}"),
    }
}

async fn send_surrender_on_first_action_request(conn: &mut TcpConnection) {
    for _ in 0..20 {
        let msg = timeout(Duration::from_secs(2), conn.recv())
            .await
            .expect("timed out waiting for post-start message")
            .expect("failed to receive post-start message");

        match msg {
            NetworkMessage::RequestAction { .. } => {
                conn.send(&NetworkMessage::CommandResponse {
                    command: Command::Surrender,
                })
                .await
                .expect("failed to send surrender command");
                return;
            }
            NetworkMessage::EventNotification { .. } => continue,
            NetworkMessage::Disconnect { .. } => return,
            _ => continue,
        }
    }

    panic!("did not receive RequestAction after game start");
}

#[tokio::test]
async fn network_game_tcp_handshake_success_and_deck_submit() {
    let addr = reserve_local_addr();
    let server = GameServer::new(
        addr,
        make_empty_script_index(),
        make_test_rules(),
        "1.0.0".to_string(),
        42,
    );
    let server_task = tokio::spawn(async move { server.start().await });

    let mut conn1 = connect_with_retry(addr).await;
    let mut conn2 = connect_with_retry(addr).await;

    let player1 = complete_hello_handshake(&mut conn1, "1.0.0", "Client-A").await;
    let player2 = complete_hello_handshake(&mut conn2, "1.0.0", "Client-B").await;
    assert_eq!(player1, PlayerId::Player1);
    assert_eq!(player2, PlayerId::Player2);

    conn1
        .send(&NetworkMessage::DeckSubmit { card_ids: vec![] })
        .await
        .expect("failed to submit deck for player1");
    conn2
        .send(&NetworkMessage::DeckSubmit { card_ids: vec![] })
        .await
        .expect("failed to submit deck for player2");

    expect_deck_accepted_and_game_start(&mut conn1).await;
    expect_deck_accepted_and_game_start(&mut conn2).await;

    send_surrender_on_first_action_request(&mut conn1).await;

    let server_result = timeout(Duration::from_secs(5), server_task)
        .await
        .expect("game server task did not finish")
        .expect("game server task panicked");
    assert!(server_result.is_ok(), "game server returned error: {server_result:?}");
}

#[tokio::test]
async fn network_game_version_mismatch_rejected() {
    let addr = reserve_local_addr();
    let server = GameServer::new(
        addr,
        make_empty_script_index(),
        make_test_rules(),
        "2.0.0".to_string(),
        7,
    );
    let server_task = tokio::spawn(async move { server.start().await });

    let mut conn = connect_with_retry(addr).await;

    let hello = timeout(Duration::from_secs(2), conn.recv())
        .await
        .expect("timed out waiting for server hello")
        .expect("failed to receive server hello");
    assert!(matches!(hello, NetworkMessage::Hello { .. }));

    conn.send(&NetworkMessage::Hello {
        version: "1.0.0".to_string(),
        player_name: "OldClient".to_string(),
    })
    .await
    .expect("failed to send mismatched hello");

    let disconnect = timeout(Duration::from_secs(2), conn.recv())
        .await
        .expect("timed out waiting for disconnect")
        .expect("failed to receive disconnect");
    match disconnect {
        NetworkMessage::Disconnect { reason } => {
            assert!(
                reason.contains("version mismatch"),
                "unexpected disconnect reason: {reason}"
            );
        }
        other => panic!("expected Disconnect, got {other:?}"),
    }

    let server_result = timeout(Duration::from_secs(3), server_task)
        .await
        .expect("game server task did not finish")
        .expect("game server task panicked");
    assert!(server_result.is_err(), "expected version mismatch to fail");
}

#[tokio::test]
async fn network_game_wrong_first_message_rejected() {
    let addr = reserve_local_addr();
    let server = GameServer::new(
        addr,
        make_empty_script_index(),
        make_test_rules(),
        "1.0.0".to_string(),
        9,
    );
    let server_task = tokio::spawn(async move { server.start().await });

    let mut conn = connect_with_retry(addr).await;

    let hello = timeout(Duration::from_secs(2), conn.recv())
        .await
        .expect("timed out waiting for server hello")
        .expect("failed to receive server hello");
    assert!(matches!(hello, NetworkMessage::Hello { .. }));

    conn.send(&NetworkMessage::DeckSubmit { card_ids: vec![] })
        .await
        .expect("failed to send wrong first message");

    let disconnect_or_error = timeout(Duration::from_secs(2), conn.recv())
        .await
        .expect("timed out waiting for disconnect or socket close");

    match disconnect_or_error {
        Ok(NetworkMessage::Disconnect { reason }) => {
            assert!(
                reason.contains("expected Hello"),
                "unexpected disconnect reason: {reason}"
            );
        }
        Ok(other) => panic!("expected Disconnect or recv error, got {other:?}"),
        Err(_) => {}
    }

    let server_result = timeout(Duration::from_secs(3), server_task)
        .await
        .expect("game server task did not finish")
        .expect("game server task panicked");
    assert!(server_result.is_err(), "expected protocol violation to fail");
}
