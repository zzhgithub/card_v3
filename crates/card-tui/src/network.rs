use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use card_core::engine::phase::PhaseAction;
use card_core::rules::GameRules;
use card_core::types::{CardId, InstanceId, Zone};
use card_protocol::codec::TcpConnection;
use card_protocol::message::{
    AttackTarget, AvailableAction, Command, CostPayment, GameEvent, NetworkMessage,
};
use card_script::loader::ScriptIndex;
use card_server::GameServer;

use crate::app::UiEvent;
use crate::matchmaker_client::{find_match, MatchResult};

pub async fn run_online_game_session(
    action_rx: mpsc::Receiver<PhaseAction>,
    recovery_rx: mpsc::Receiver<Vec<InstanceId>>,
    ui_event_tx: mpsc::Sender<UiEvent>,
) -> Result<()> {
    let _ = ui_event_tx.send(UiEvent::Log("连接匹配服务器...".to_string()));
    let match_result = find_match("ws://127.0.0.1:9090", "Player", "0.1.0").await?;
    let _ = ui_event_tx.send(UiEvent::Log(format!(
        "匹配成功：对手={}，主机={}，我方是否主机={}",
        match_result.opponent_name, match_result.host_addr, match_result.is_host
    )));

    if match_result.is_host {
        run_host_flow(match_result, action_rx, recovery_rx, ui_event_tx).await
    } else {
        run_guest_flow(match_result, action_rx, recovery_rx, ui_event_tx).await
    }
}

async fn run_host_flow(
    match_result: MatchResult,
    action_rx: mpsc::Receiver<PhaseAction>,
    recovery_rx: mpsc::Receiver<Vec<InstanceId>>,
    ui_event_tx: mpsc::Sender<UiEvent>,
) -> Result<()> {
    let bind_addr = derive_host_bind_addr(&match_result.host_addr);
    let script_index = ScriptIndex::scan(Path::new("/tmp/card_tui_empty_scripts"))
        .map_err(|err| anyhow::anyhow!("扫描脚本目录失败: {err}"))?;

    let server = GameServer::new(
        bind_addr,
        script_index,
        GameRules::default(),
        "0.1.0".to_string(),
        20260322,
    );
    let server_tx = ui_event_tx.clone();
    tokio::spawn(async move {
        if let Err(err) = server.start().await {
            let _ = server_tx.send(UiEvent::Log(format!("游戏服务器错误: {err}")));
        }
    });

    let _ = ui_event_tx.send(UiEvent::Log(format!(
        "已启动本地游戏服务器，监听地址: {bind_addr}"
    )));

    tokio::time::sleep(Duration::from_millis(200)).await;
    run_network_client(
        &bind_addr.to_string(),
        &match_result,
        action_rx,
        recovery_rx,
        ui_event_tx,
    )
    .await
}

async fn run_guest_flow(
    match_result: MatchResult,
    action_rx: mpsc::Receiver<PhaseAction>,
    recovery_rx: mpsc::Receiver<Vec<InstanceId>>,
    ui_event_tx: mpsc::Sender<UiEvent>,
) -> Result<()> {
    run_network_client(
        &match_result.host_addr,
        &match_result,
        action_rx,
        recovery_rx,
        ui_event_tx,
    )
    .await
}

fn derive_host_bind_addr(host_addr: &str) -> SocketAddr {
    if let Ok(addr) = SocketAddr::from_str(host_addr) {
        return SocketAddr::new(addr.ip(), 10000);
    }
    SocketAddr::from(([127, 0, 0, 1], 10000))
}

async fn run_network_client(
    server_addr: &str,
    match_result: &MatchResult,
    action_rx: mpsc::Receiver<PhaseAction>,
    recovery_rx: mpsc::Receiver<Vec<InstanceId>>,
    ui_event_tx: mpsc::Sender<UiEvent>,
) -> Result<()> {
    let stream = tokio::net::TcpStream::connect(server_addr)
        .await
        .map_err(|err| anyhow::anyhow!("无法连接游戏服务器 {server_addr}: {err}"))?;
    let mut conn = TcpConnection::from_stream(stream);

    let server_version = match conn.recv().await? {
        NetworkMessage::Hello {
            version,
            player_name: _,
        } => version,
        other => {
            anyhow::bail!("握手失败，收到无效消息: {other:?}")
        }
    };

    conn.send(&NetworkMessage::Hello {
        version: server_version,
        player_name: "Player".to_string(),
    })
    .await?;

    match conn.recv().await? {
        NetworkMessage::HelloAck { player_id } => {
            let _ = ui_event_tx.send(UiEvent::Log(format!("握手成功，分配身份: {player_id:?}")));
        }
        other => anyhow::bail!("握手确认失败: {other:?}"),
    }

    conn.send(&NetworkMessage::DeckSubmit {
        card_ids: build_demo_deck_ids(),
    })
    .await?;

    match conn.recv().await? {
        NetworkMessage::DeckAccepted => {
            let _ = ui_event_tx.send(UiEvent::Log("卡组校验通过".to_string()));
        }
        NetworkMessage::DeckRejected { reason } => {
            anyhow::bail!("卡组被拒绝: {reason}")
        }
        other => anyhow::bail!("等待卡组校验结果时收到无效消息: {other:?}"),
    }

    match conn.recv().await? {
        NetworkMessage::GameStart { .. } => {
            let _ = ui_event_tx.send(UiEvent::EnterOnlineBattle {
                opponent_name: match_result.opponent_name.clone(),
            });
        }
        other => anyhow::bail!("等待开局消息时收到无效消息: {other:?}"),
    }

    loop {
        match conn.recv().await? {
            NetworkMessage::EventNotification { event } => {
                if let GameEvent::GameOver { winner, reason } = event {
                    let _ = ui_event_tx.send(UiEvent::Log(format!(
                        "对局结束，胜者: {winner:?}，原因: {reason:?}"
                    )));
                    break;
                } else {
                    let _ = ui_event_tx.send(UiEvent::Log(format!("游戏事件: {event:?}")));
                }
            }
            NetworkMessage::RequestAction {
                available_actions, ..
            } => {
                let actions = available_actions_to_phase_actions(&available_actions);
                let _ = ui_event_tx.send(UiEvent::ActionPrompt(actions));
                let chosen = action_rx
                    .recv()
                    .map_err(|_| anyhow::anyhow!("无法获取操作输入"))?;
                let command = phase_action_to_command(chosen);
                conn.send(&NetworkMessage::CommandResponse { command }).await?;
            }
            NetworkMessage::RequestCardSelection {
                candidates, count, ..
            } => {
                let _ = ui_event_tx.send(UiEvent::RecoveryPrompt {
                    count,
                    options: candidates.clone(),
                });
                let selected = recovery_rx
                    .recv()
                    .map_err(|_| anyhow::anyhow!("无法获取选牌输入"))?;
                conn.send(&NetworkMessage::CardResponse {
                    instance_ids: selected,
                })
                .await?;
            }
            NetworkMessage::RequestTargetSelection { .. } => {
                conn.send(&NetworkMessage::TargetResponse { targets: Vec::new() })
                    .await?;
            }
            NetworkMessage::Ping => {
                conn.send(&NetworkMessage::Pong).await?;
            }
            NetworkMessage::Disconnect { reason } => {
                anyhow::bail!("服务器断开连接: {reason}")
            }
            NetworkMessage::Hello { .. }
            | NetworkMessage::HelloAck { .. }
            | NetworkMessage::DeckSubmit { .. }
            | NetworkMessage::DeckAccepted
            | NetworkMessage::DeckRejected { .. }
            | NetworkMessage::GameStart { .. }
            | NetworkMessage::CommandResponse { .. }
            | NetworkMessage::TargetResponse { .. }
            | NetworkMessage::CardResponse { .. }
            | NetworkMessage::Pong => {}
        }
    }

    Ok(())
}

fn available_actions_to_phase_actions(available: &[AvailableAction]) -> Vec<PhaseAction> {
    let mut actions = Vec::new();
    for action in available {
        match action {
            AvailableAction::PlayCard { instance_id } => actions.push(PhaseAction::PlayCard {
                instance_id: *instance_id,
                target_zone: Zone::Front(0),
                cost_payment: Vec::new(),
            }),
            AvailableAction::DeclareAttack {
                attacker,
                possible_targets,
            } => {
                let target_slot = possible_targets.iter().find_map(|target| match target {
                    AttackTarget::FrontSlot(slot) => Some(*slot),
                    AttackTarget::DirectAttack => None,
                });
                actions.push(PhaseAction::DeclareAttack {
                    attacker: *attacker,
                    target_slot,
                });
            }
            AvailableAction::ChainPass => actions.push(PhaseAction::Pass),
            AvailableAction::Surrender => actions.push(PhaseAction::Surrender),
            AvailableAction::ActivateEffect { .. }
            | AvailableAction::ChainActivate { .. }
            | AvailableAction::SelectRecoveryCards { .. } => {}
        }
    }

    if actions.is_empty() {
        actions.push(PhaseAction::Pass);
    }
    actions
}

fn phase_action_to_command(action: PhaseAction) -> Command {
    match action {
        PhaseAction::PlayCard {
            instance_id,
            target_zone,
            ..
        } => Command::PlayCard {
            instance_id,
            target_zone,
            cost_payment: CostPayment {
                hand_cards: Vec::new(),
                real_point: 0,
            },
        },
        PhaseAction::DeclareAttack {
            attacker,
            target_slot,
        } => Command::DeclareAttack {
            attacker,
            target: target_slot
                .map(AttackTarget::FrontSlot)
                .unwrap_or(AttackTarget::DirectAttack),
        },
        PhaseAction::Pass => Command::ChainPass,
        PhaseAction::Surrender => Command::Surrender,
    }
}

fn build_demo_deck_ids() -> Vec<CardId> {
    (0..40).map(|_| CardId::new("S000-C-001")).collect()
}
