use crate::colors::*;
use crate::local_game_setup::LocalGameSetupState;
use crate::ui_components::{spawn_return_button, spawn_status_bar, spawn_version_display};
use bevy::prelude::*;
use card_core::deck::DeckManager;
use card_core::engine::phase::{PhaseAction, PhaseClient, RecoveryCardOption};
use card_core::state::CardInstance;
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
    StateUpdate(crate::local_game::GameDisplayState),
    GameOver(String),
}

#[derive(Clone, Debug, Default)]
pub struct GameDisplayState {
    pub player_hp: u32,
    pub ai_hp: u32,
    pub player_hand_count: usize,
    pub ai_hand_count: usize,
    pub player_deck_count: usize,
    pub ai_deck_count: usize,
    pub player_forend_count: usize,
    pub ai_forend_count: usize,
    pub player_backend_count: usize,
    pub ai_backend_count: usize,
    pub player_grave_count: usize,
    pub ai_grave_count: usize,
    pub player_cost_count: usize,
    pub ai_cost_count: usize,
    pub current_phase: String,
    pub turn_number: u32,
}

pub fn enter_local_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    setup_state: Res<LocalGameSetupState>,
) {
    spawn_version_display(&mut commands, &asset_server);
    spawn_status_bar(&mut commands, &asset_server, "本地游戏进行中");
    spawn_return_button(&mut commands, &asset_server);

    let font = asset_server.load(FONT_PATH);

    if let (Some(ref player_deck), Some(ref ai_deck)) =
        (setup_state.player_deck.clone(), setup_state.ai_deck.clone())
    {
        let player_cards = if let Ok(deck) = DeckManager::load(&player_deck.file_path) {
            deck.cards
        } else {
            vec![]
        };

        let ai_cards = if let Ok(deck) = DeckManager::load(&ai_deck.file_path) {
            deck.cards
        } else {
            vec![]
        };

        if !player_cards.is_empty() && !ai_cards.is_empty() {
            let (action_tx, action_rx) = channel::<PhaseAction>();
            let (recovery_tx, recovery_rx) = channel::<Vec<InstanceId>>();
            let (state_tx, state_rx) = channel::<GameStateUpdate>();

            let state_tx_clone = state_tx.clone();

            let scripts_root = find_scripts_root();
            if let Ok((game_state, registry)) =
                build_game_from_decks(player_cards.clone(), ai_cards.clone(), scripts_root)
            {
                let game_thread = thread::spawn(move || {
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

                    let result = engine.run();

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

                spawn_game_ui(&mut commands, &font);
            } else {
                spawn_error_message(&mut commands, &font, "无法加载卡组，请检查卡组文件");
            }
        } else {
            spawn_error_message(&mut commands, &font, "卡组为空，请选择有效的卡组");
        }
    } else {
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

fn spawn_game_ui(commands: &mut Commands, font: &Handle<Font>) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(80.0), Val::Px(20.0)),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    GameUI,
                ))
                .with_children(|parent| {
                    parent
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("AI 对手"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT),
                            ));
                        });

                    parent
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(40.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("游戏进行中..."),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 28.0,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT),
                                GameStatusText,
                            ));
                        });

                    parent
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("玩家"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT),
                            ));
                        });
                });

            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        left: Val::Px(0.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                    ActionPanel,
                    Visibility::Hidden,
                ))
                .with_children(|parent| {
                    parent
                        .spawn(Node {
                            width: Val::Percent(60.0),
                            height: Val::Percent(70.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(20.0)),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("选择行动"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 32.0,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT),
                            ));

                            parent.spawn(Node {
                                height: Val::Px(20.0),
                                ..default()
                            });

                            parent.spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(10.0),
                                    ..default()
                                },
                                ActionButtonsContainer,
                            ));
                        });
                });

            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        left: Val::Px(0.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.9)),
                    GameOverPanel,
                    Visibility::Hidden,
                ))
                .with_children(|parent| {
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("游戏结束"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 48.0,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT),
                            ));

                            parent.spawn(Node {
                                height: Val::Px(20.0),
                                ..default()
                            });

                            parent.spawn((
                                Text::new(""),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 36.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.8, 0.0)),
                                WinnerText,
                            ));
                        });
                });
        });
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
struct GameUI;

#[derive(Component)]
struct GameStatusText;

#[derive(Component)]
pub struct ActionPanel;

#[derive(Component)]
struct ActionButtonsContainer;

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
            GameStateUpdate::StateUpdate(_state) => {}
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
        let action_text = format!("{:?}", action);
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
                BackgroundColor(COLOR_BUTTON),
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
                    TextColor(COLOR_TEXT),
                ));
            });
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

pub fn cleanup_local_game(query: Query<Entity, With<Node>>, mut commands: Commands) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<LocalGameSession>();
}
