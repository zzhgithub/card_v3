use crate::app_state::{AppState, SelectedDeck};
use crate::colors::*;
use crate::ui_components::{spawn_return_button, spawn_status_bar, spawn_version_display};
use bevy::prelude::*;
use card_core::deck::{DeckManager, DeckSummary};
use std::path::PathBuf;

#[derive(Resource, Default)]
pub struct LocalGameSetupState {
    pub player_deck: Option<DeckSummary>,
    pub ai_deck: Option<DeckSummary>,
}

#[derive(Component)]
pub struct PlayerDeckList;

#[derive(Component)]
pub struct AiDeckList;

#[derive(Component)]
pub struct StartGameButton;

#[derive(Component)]
pub struct DeckSelectionItem {
    pub deck_summary: DeckSummary,
    pub is_player: bool,
}

pub fn enter_local_game_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    setup_state: Res<LocalGameSetupState>,
) {
    spawn_version_display(&mut commands, &asset_server);
    spawn_status_bar(&mut commands, &asset_server, "本地游戏 - 选择卡组");
    spawn_return_button(&mut commands, &asset_server);

    let font = asset_server.load(FONT_PATH);

    // Get available decks
    let desk_dir = get_desks_directory();
    let decks = DeckManager::list_decks(&desk_dir).unwrap_or_default();

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(80.0), Val::Px(20.0)),
            ..default()
        })
        .with_children(|parent| {
            // Two-column layout
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(40.0),
                    ..default()
                })
                .with_children(|parent| {
                    // Left column - Player deck selection
                    spawn_deck_column(
                        parent,
                        &font,
                        "选择你的卡组",
                        true,
                        &decks,
                        &setup_state.player_deck,
                    );

                    // Right column - AI deck selection
                    spawn_deck_column(
                        parent,
                        &font,
                        "选择AI卡组",
                        false,
                        &decks,
                        &setup_state.ai_deck,
                    );
                });

            // Start game button at bottom
            parent
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(40.0),
                    left: Val::Percent(50.0),
                    ..default()
                })
                .with_children(|parent| {
                    parent
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(200.0),
                                height: Val::Px(50.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(
                                if setup_state.player_deck.is_some()
                                    && setup_state.ai_deck.is_some()
                                {
                                    Color::srgb(0.2, 0.8, 0.3)
                                } else {
                                    Color::srgb(0.3, 0.3, 0.3)
                                },
                            ),
                            StartGameButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("开始游戏"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT),
                            ));
                        });
                });
        });
}

fn spawn_deck_column(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    title: &str,
    is_player: bool,
    decks: &[DeckSummary],
    selected_deck: &Option<DeckSummary>,
) {
    // Create the base node
    let mut entity_commands = parent.spawn((Node {
        width: Val::Percent(50.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        ..default()
    },));

    // Add the marker component based on is_player
    if is_player {
        entity_commands.insert(PlayerDeckList);
    } else {
        entity_commands.insert(AiDeckList);
    }

    // Add children
    entity_commands.with_children(|parent| {
        // Title
        parent.spawn((
            Text::new(title),
            TextFont {
                font: font.clone(),
                font_size: 28.0,
                ..default()
            },
            TextColor(COLOR_TEXT),
        ));

        parent.spawn(Node {
            height: Val::Px(20.0),
            ..default()
        });

        // Deck list
        if decks.is_empty() {
            parent.spawn((
                Text::new("暂无卡组，请先创建卡组"),
                TextFont {
                    font: font.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(COLOR_TEXT_DIM),
            ));
        } else {
            for deck in decks.iter() {
                let is_selected = selected_deck
                    .as_ref()
                    .map(|d| d.file_path == deck.file_path)
                    .unwrap_or(false);

                spawn_deck_item(parent, font, deck, is_player, is_selected);
            }
        }
    });
}

fn spawn_deck_item(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    deck: &DeckSummary,
    is_player: bool,
    is_selected: bool,
) {
    let bg_color = if is_selected {
        Color::srgb(0.3, 0.6, 0.9)
    } else {
        COLOR_BUTTON
    };

    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(90.0),
                height: Val::Px(60.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(20.0)),
                margin: UiRect::vertical(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(bg_color),
            DeckSelectionItem {
                deck_summary: deck.clone(),
                is_player,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(deck.name.clone()),
                TextFont {
                    font: font.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));

            parent.spawn((
                Text::new(format!("{} 张", deck.card_count)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(COLOR_TEXT_DIM),
            ));
        });
}

pub fn handle_deck_selection(
    mut interaction_query: Query<(&Interaction, &DeckSelectionItem), Changed<Interaction>>,
    mut setup_state: ResMut<LocalGameSetupState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, item) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            if item.is_player {
                setup_state.player_deck = Some(item.deck_summary.clone());
            } else {
                setup_state.ai_deck = Some(item.deck_summary.clone());
            }
            // Refresh the UI to show selection
            next_state.set(AppState::LocalGameSetup);
        }
    }
}

pub fn handle_start_game_button(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<StartGameButton>)>,
    setup_state: Res<LocalGameSetupState>,
    mut next_state: ResMut<NextState<AppState>>,
    mut selected_deck: ResMut<SelectedDeck>,
) {
    for interaction in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            if let Some(ref player_deck) = setup_state.player_deck {
                // Load player deck into SelectedDeck resource
                selected_deck.name = player_deck.name.clone();
                selected_deck.file_path = player_deck.file_path.to_string_lossy().to_string();
                if let Ok(deck) = DeckManager::load(&player_deck.file_path) {
                    selected_deck.cards = deck.cards.iter().map(|c| c.to_string()).collect();
                }

                // Transition to LocalGame state
                next_state.set(AppState::LocalGame);
            }
        }
    }
}

pub fn cleanup_local_game_setup(query: Query<Entity, With<Node>>, mut commands: Commands) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn get_desks_directory() -> PathBuf {
    let candidates = [PathBuf::from("desks"), PathBuf::from("../../desks")];

    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    PathBuf::from("desks")
}
