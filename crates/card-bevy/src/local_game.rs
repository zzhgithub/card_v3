use crate::colors::*;
use crate::game_board::{
    bw, cleanup_game_board, setup_game_board, spawn_card_sprite, BoardLayout, GameBoardEntity,
};
use crate::local_game_setup::LocalGameSetupState;
use bevy::prelude::*;
use card_core::deck::DeckManager;
use card_core::engine::phase::{PhaseAction, PhaseClient, RecoveryCardOption};
use card_core::state::{CardInstance, GameState, Phase, PlayerZones};
use card_core::types::{CardId, InstanceId, PlayerId};
use card_script::loader::{ScriptIndex, ScriptLoader};
use card_server::ai_client::AiClient;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Resource)]
pub struct LocalGameSession {
    pub action_sender: Sender<PhaseAction>,
    pub recovery_sender: Sender<Vec<InstanceId>>,
    pub state_receiver: Mutex<Receiver<GameStateUpdate>>,
    pub game_thread: Option<thread::JoinHandle<()>>,
    pub pending_actions: Option<Vec<PhaseAction>>,
    pub pending_recovery: Option<(usize, Vec<RecoveryCardOption>)>,
    pub game_over: bool,
    pub winner: Option<String>,
}

#[derive(Clone, Debug)]
pub enum GameStateUpdate {
    ActionsRequested(Vec<PhaseAction>),
    RecoveryRequested(usize, Vec<RecoveryCardOption>),
    StateUpdate {
        turn_number: u32,
        current_player: PlayerId,
        current_phase: Phase,
        player_zones: [GamePlayerZones; 2],
    },
    GameOver(String),
}

#[derive(Clone, Debug, Default)]
pub struct GamePlayerZones {
    pub hp: u32,
    pub deck: Vec<CardDisplayInfo>,
    pub hand: Vec<CardDisplayInfo>,
    pub front: [Option<CardDisplayInfo>; 5],
    pub back: [Option<CardDisplayInfo>; 5],
    pub cost_zone: Vec<CardDisplayInfo>,
    pub grave: Vec<CardDisplayInfo>,
}

#[derive(Clone, Debug)]
pub struct CardDisplayInfo {
    pub instance_id: InstanceId,
    pub definition_id: CardId,
    pub current_attack: Option<i32>,
}

pub fn enter_local_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    setup_state: Res<LocalGameSetupState>,
    layout: Res<BoardLayout>,
) {
    info!("[LocalGame] Entering local game state");

    let font = asset_server.load(FONT_PATH);
    setup_game_board(&mut commands, &layout, &font);

    if let (Some(ref player_deck), Some(ref ai_deck)) =
        (setup_state.player_deck.clone(), setup_state.ai_deck.clone())
    {
        info!(
            "[LocalGame] Loading decks - Player: {:?}, AI: {:?}",
            player_deck.name, ai_deck.name
        );

        let player_cards = if let Ok(deck) = DeckManager::load(&player_deck.file_path) {
            info!("[LocalGame] Player deck loaded: {} cards", deck.cards.len());
            deck.cards
        } else {
            error!(
                "[LocalGame] Failed to load player deck: {:?}",
                player_deck.file_path
            );
            vec![]
        };

        let ai_cards = if let Ok(deck) = DeckManager::load(&ai_deck.file_path) {
            info!("[LocalGame] AI deck loaded: {} cards", deck.cards.len());
            deck.cards
        } else {
            error!(
                "[LocalGame] Failed to load AI deck: {:?}",
                ai_deck.file_path
            );
            vec![]
        };

        if !player_cards.is_empty() && !ai_cards.is_empty() {
            info!(
                "[LocalGame] Starting game with {} player cards and {} AI cards",
                player_cards.len(),
                ai_cards.len()
            );

            let (action_tx, action_rx) = channel::<PhaseAction>();
            let (recovery_tx, recovery_rx) = channel::<Vec<InstanceId>>();
            let (state_tx, state_rx) = channel::<GameStateUpdate>();
            let state_tx_clone = state_tx.clone();

            let scripts_root = find_scripts_root();
            info!("[LocalGame] Using scripts root: {:?}", scripts_root);

            if let Ok((game_state, registry)) =
                build_game_from_decks(player_cards.clone(), ai_cards.clone(), scripts_root)
            {
                info!(
                    "[LocalGame] Game state built successfully, seed: {}",
                    game_state.rng_seed
                );
                send_state_update(&state_tx_clone, &game_state);

                let game_thread = thread::spawn(move || {
                    info!("[LocalGame] Game engine thread started");

                    let player_client = BevyHumanClient {
                        action_receiver: Arc::new(Mutex::new(action_rx)),
                        recovery_receiver: Arc::new(Mutex::new(recovery_rx)),
                        state_sender: state_tx_clone.clone(),
                    };
                    let ai_client = AiClient::new(42);
                    let engine = card_core::engine::GameEngine::new(
                        game_state,
                        registry,
                        Box::new(player_client),
                        Box::new(ai_client),
                    );

                    info!("[LocalGame] Running game engine...");
                    let result = engine.run();
                    info!(
                        "[LocalGame] Game engine finished, winner: {:?}",
                        result.winner
                    );

                    let winner = match result.winner {
                        Some(PlayerId::Player1) => "玩家获胜!".to_string(),
                        Some(PlayerId::Player2) => "AI获胜!".to_string(),
                        None => "平局!".to_string(),
                    };
                    let _ = state_tx_clone.send(GameStateUpdate::GameOver(winner));
                });

                commands.insert_resource(LocalGameSession {
                    action_sender: action_tx,
                    recovery_sender: recovery_tx,
                    state_receiver: Mutex::new(state_rx),
                    game_thread: Some(game_thread),
                    pending_actions: None,
                    pending_recovery: None,
                    game_over: false,
                    winner: None,
                });

                spawn_initial_cards(&mut commands, &layout, &player_cards, &ai_cards);
                info!("[LocalGame] Local game session initialized");
            } else {
                error!("[LocalGame] Failed to build game from decks");
                spawn_error_message(&mut commands, &font, "无法加载卡组，请检查卡组文件");
            }
        } else {
            error!("[LocalGame] Empty decks detected");
            spawn_error_message(&mut commands, &font, "卡组为空，请选择有效的卡组");
        }
    } else {
        error!("[LocalGame] Decks not selected");
        spawn_error_message(&mut commands, &font, "请先选择卡组和AI卡组");
    }
}

fn find_scripts_root() -> std::path::PathBuf {
    let candidates = [
        std::path::PathBuf::from("scripts"),
        std::path::PathBuf::from("../../scripts"),
    ];
    candidates
        .iter()
        .find(|p| p.join("S000").exists())
        .cloned()
        .unwrap_or(std::path::PathBuf::from("scripts"))
}

fn build_game_from_decks(
    player_cards: Vec<CardId>,
    ai_cards: Vec<CardId>,
    scripts_root: std::path::PathBuf,
) -> anyhow::Result<(
    card_core::state::GameState,
    card_core::state::CardRegistryImpl,
)> {
    let index =
        ScriptIndex::scan(&scripts_root).map_err(|e| anyhow::anyhow!("扫描脚本目录失败: {e}"))?;
    let loader = ScriptLoader::new(index);
    let registry = loader
        .load_for_game(&player_cards, &ai_cards)
        .map_err(|e| anyhow::anyhow!("加载卡片定义失败: {e}"))?;

    let rules = card_core::rules::GameRules::default();
    let mut state = card_core::state::GameState::new(rules, 20260322);
    let mut next_id = 1u32;

    for card_id in &player_cards {
        let base_attack = registry
            .get(card_id)
            .and_then(|def| def.attack.map(|a| a as i32));
        state.players[0].zones.deck.push(CardInstance::new(
            InstanceId(next_id),
            card_id.clone(),
            base_attack,
        ));
        next_id += 1;
    }
    for card_id in &ai_cards {
        let base_attack = registry
            .get(card_id)
            .and_then(|def| def.attack.map(|a| a as i32));
        state.players[1].zones.deck.push(CardInstance::new(
            InstanceId(next_id),
            card_id.clone(),
            base_attack,
        ));
        next_id += 1;
    }

    Ok((state, registry))
}

struct BevyHumanClient {
    action_receiver: Arc<Mutex<Receiver<PhaseAction>>>,
    recovery_receiver: Arc<Mutex<Receiver<Vec<InstanceId>>>>,
    state_sender: Sender<GameStateUpdate>,
}

impl PhaseClient for BevyHumanClient {
    fn choose_action(&self, available: &[PhaseAction], _timeout: Duration) -> Option<PhaseAction> {
        let _ = self
            .state_sender
            .send(GameStateUpdate::ActionsRequested(available.to_vec()));
        self.action_receiver.lock().ok()?.recv().ok()
    }

    fn choose_recovery_cards(
        &self,
        options: &[RecoveryCardOption],
        count: usize,
        _timeout: Duration,
    ) -> Option<Vec<InstanceId>> {
        let _ = self
            .state_sender
            .send(GameStateUpdate::RecoveryRequested(count, options.to_vec()));
        self.recovery_receiver.lock().ok()?.recv().ok()
    }
}

fn spawn_error_message(commands: &mut Commands, font: &Handle<Font>, message: &str) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new(message),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.3, 0.3)),
            ));
        });
}

#[derive(Component)]
pub struct ActionPanel;

#[derive(Component)]
pub struct GameOverPanel;

#[derive(Component)]
pub struct WinnerText;

#[derive(Component)]
pub struct ActionButton {
    pub action_index: usize,
}

pub fn update_local_game(
    mut session: ResMut<LocalGameSession>,
    mut commands: Commands,
    mut visibility_set: ParamSet<(
        Query<&mut Visibility, With<ActionPanel>>,
        Query<&mut Visibility, With<GameOverPanel>>,
    )>,
    mut winner_text_query: Query<&mut Text, With<WinnerText>>,
    asset_server: Res<AssetServer>,
) {
    let updates: Vec<GameStateUpdate> = {
        let receiver = session.state_receiver.lock().unwrap();
        std::iter::from_fn(|| receiver.try_recv().ok()).collect()
    };

    for update in updates {
        match update {
            GameStateUpdate::ActionsRequested(actions) => {
                session.pending_actions = Some(actions.clone());
                if let Ok(mut visibility) = visibility_set.p0().single_mut() {
                    *visibility = Visibility::Visible;
                }
                spawn_action_buttons(&mut commands, &asset_server, &actions);
            }
            GameStateUpdate::RecoveryRequested(count, options) => {
                session.pending_recovery = Some((count, options));
            }
            GameStateUpdate::StateUpdate { .. } => {}
            GameStateUpdate::GameOver(winner) => {
                session.game_over = true;
                session.winner = Some(winner.clone());
                if let Ok(mut visibility) = visibility_set.p1().single_mut() {
                    *visibility = Visibility::Visible;
                }
                for mut text in winner_text_query.iter_mut() {
                    text.0 = winner.clone();
                }
            }
        }
    }
}

fn spawn_action_buttons(
    commands: &mut Commands,
    asset_server: &AssetServer,
    actions: &[PhaseAction],
) {
    let font = asset_server.load(FONT_PATH);
    for (index, action) in actions.iter().enumerate() {
        let action_text = action_label(action);
        commands
            .spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(40.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::vertical(Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.231, 0.51, 0.965)),
                ActionButton {
                    action_index: index,
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(action_text),
                    TextFont {
                        font: font.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                ));
            });
    }
}

fn action_label(action: &PhaseAction) -> String {
    match action {
        PhaseAction::PlayCard { .. } => "使用卡牌".to_string(),
        PhaseAction::DeclareAttack { .. } => "宣告攻击".to_string(),
        PhaseAction::Pass => "结束当前阶段".to_string(),
        PhaseAction::Surrender => "认输".to_string(),
    }
}

pub fn handle_action_button_click(
    mut interaction_query: Query<(&Interaction, &ActionButton), Changed<Interaction>>,
    session: Res<LocalGameSession>,
    mut action_panel_query: Query<&mut Visibility, With<ActionPanel>>,
) {
    for (interaction, button) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            if let Some(ref actions) = session.pending_actions {
                if button.action_index < actions.len() {
                    let action = actions[button.action_index].clone();
                    let _ = session.action_sender.send(action);
                    if let Ok(mut visibility) = action_panel_query.single_mut() {
                        *visibility = Visibility::Hidden;
                    }
                }
            }
        }
    }
}

pub fn cleanup_local_game(
    mut commands: Commands,
    ui_query: Query<Entity, With<Node>>,
    board_query: Query<Entity, With<GameBoardEntity>>,
) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in board_query.iter() {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<LocalGameSession>();
}

fn send_state_update(sender: &Sender<GameStateUpdate>, state: &GameState) {
    let player_zones = zones_to_display(&state.players[0].zones);
    let ai_zones = zones_to_display(&state.players[1].zones);
    let update = GameStateUpdate::StateUpdate {
        turn_number: state.turn_number,
        current_player: state.turn_player,
        current_phase: state.phase.clone(),
        player_zones: [player_zones, ai_zones],
    };
    let _ = sender.send(update);
}

fn zones_to_display(zones: &PlayerZones) -> GamePlayerZones {
    GamePlayerZones {
        hp: zones.cost_zone.len() as u32,
        deck: zones
            .deck
            .iter()
            .map(|c| CardDisplayInfo {
                instance_id: c.instance_id,
                definition_id: c.definition_id.clone(),
                current_attack: c.current_attack,
            })
            .collect(),
        hand: zones
            .hand
            .iter()
            .map(|c| CardDisplayInfo {
                instance_id: c.instance_id,
                definition_id: c.definition_id.clone(),
                current_attack: c.current_attack,
            })
            .collect(),
        front: zones.front.clone().map(|opt| {
            opt.map(|c| CardDisplayInfo {
                instance_id: c.instance_id,
                definition_id: c.definition_id.clone(),
                current_attack: c.current_attack,
            })
        }),
        back: zones.back.clone().map(|opt| {
            opt.map(|c| CardDisplayInfo {
                instance_id: c.instance_id,
                definition_id: c.definition_id.clone(),
                current_attack: c.current_attack,
            })
        }),
        cost_zone: zones
            .cost_zone
            .iter()
            .map(|c| CardDisplayInfo {
                instance_id: c.instance_id,
                definition_id: c.definition_id.clone(),
                current_attack: c.current_attack,
            })
            .collect(),
        grave: zones
            .grave
            .iter()
            .map(|c| CardDisplayInfo {
                instance_id: c.instance_id,
                definition_id: c.definition_id.clone(),
                current_attack: c.current_attack,
            })
            .collect(),
    }
}

fn spawn_initial_cards(
    commands: &mut Commands,
    layout: &BoardLayout,
    player_cards: &[CardId],
    ai_cards: &[CardId],
) {
    info!(
        "[LocalGame] Spawning initial card backs - Player: {} cards, AI: {} cards",
        player_cards.len(),
        ai_cards.len()
    );

    let p1 = &layout.player_areas[0];
    let p2 = &layout.player_areas[1];
    let card_back = Color::srgb(0.18, 0.13, 0.28);

    for (i, _) in player_cards.iter().enumerate() {
        let off = (i as f32) * 0.5;
        let pos = bw(p1.deck_pos + Vec2::splat(off));
        commands.spawn((
            Sprite {
                color: card_back,
                custom_size: Some(Vec2::new(75.0, 110.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 1.0 + i as f32 * 0.01),
            GameBoardEntity,
        ));
    }

    for (i, _) in ai_cards.iter().enumerate() {
        let off = (i as f32) * 0.5;
        let pos = bw(p2.deck_pos + Vec2::splat(off));
        commands.spawn((
            Sprite {
                color: card_back,
                custom_size: Some(Vec2::new(75.0, 110.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 1.0 + i as f32 * 0.01),
            GameBoardEntity,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_action_uses_readable_label() {
        assert_eq!(action_label(&PhaseAction::Pass), "结束当前阶段");
    }

    #[test]
    fn surrender_action_uses_readable_label() {
        assert_eq!(action_label(&PhaseAction::Surrender), "认输");
    }
}
