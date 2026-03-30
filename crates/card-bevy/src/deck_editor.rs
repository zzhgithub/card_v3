use crate::app_state::{
    CancelCreateButton, ConfirmCreateButton, CreateDeckButton, CreateDeckState, DeckDeleteButton,
    DeckListItem, SelectedDeck,
};
use crate::colors::*;
use crate::ui_components::{spawn_return_button, spawn_status_bar, spawn_version_display};
use crate::AppState;
use bevy::prelude::*;
use bevy_simple_text_input::{
    TextInput, TextInputInactive, TextInputPlaceholder, TextInputSubmitMessage, TextInputValue,
};
use card_core::deck::{Deck, DeckManager, DeckSummary};
use std::path::PathBuf;

#[derive(Resource, Default)]
pub struct DeckListData {
    pub decks: Vec<DeckSummary>,
}

pub fn enter_deck_editor(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut deck_list_data: ResMut<DeckListData>,
    create_state: Res<CreateDeckState>,
) {
    spawn_version_display(&mut commands, &asset_server);
    spawn_status_bar(&mut commands, &asset_server, "卡组编辑");
    spawn_return_button(&mut commands, &asset_server);

    let desk_dir = get_desks_directory();
    deck_list_data.decks = DeckManager::list_decks(&desk_dir).unwrap_or_default();

    let font = asset_server.load(FONT_PATH);

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Center,
            padding: UiRect::top(Val::Px(80.0)),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("选择卡组进行编辑"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT),
                    ));
                });

            parent.spawn(Node {
                height: Val::Px(20.0),
                ..default()
            });

            spawn_create_deck_button(parent, &asset_server);

            parent.spawn(Node {
                height: Val::Px(20.0),
                ..default()
            });

            if create_state.is_inputting {
                spawn_create_input_ui(parent, &asset_server);
            }

            if deck_list_data.decks.is_empty() {
                parent.spawn((
                    Text::new("暂无卡组，点击上方按钮创建新卡组"),
                    TextFont {
                        font: font.clone(),
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_DIM),
                ));
            } else {
                for deck in deck_list_data.decks.iter() {
                    spawn_deck_list_item(parent, &asset_server, deck);
                }
            }
        });
}

fn spawn_create_deck_button(parent: &mut ChildSpawnerCommands, asset_server: &AssetServer) {
    let font = asset_server.load(FONT_PATH);

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
            BackgroundColor(Color::srgb(0.2, 0.8, 0.3)),
            CreateDeckButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("+ 创建新卡组"),
                TextFont {
                    font,
                    font_size: 24.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));
        });
}

fn spawn_create_input_ui(parent: &mut ChildSpawnerCommands, asset_server: &AssetServer) {
    let font = asset_server.load(FONT_PATH);

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(20.0)),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new("输入卡组名称:"),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));

            parent.spawn(Node {
                height: Val::Px(10.0),
                ..default()
            });

            parent
                .spawn((
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(50.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.2)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        TextInput,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        bevy_simple_text_input::TextInputTextFont(TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        }),
                        bevy_simple_text_input::TextInputTextColor(TextColor(COLOR_TEXT)),
                        TextInputPlaceholder {
                            value: "点击输入名称...".to_string(),
                            text_font: Some(TextFont {
                                font: font.clone(),
                                font_size: 20.0,
                                ..default()
                            }),
                            text_color: Some(TextColor(COLOR_TEXT_DIM)),
                            hide_on_focus: true,
                        },
                        TextInputInactive(true),
                    ));
                });

            parent.spawn(Node {
                height: Val::Px(10.0),
                ..default()
            });

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(20.0),
                    ..default()
                })
                .with_children(|parent| {
                    spawn_confirm_button(parent, asset_server);
                    spawn_cancel_button(parent, asset_server);
                });
        });
}

fn spawn_confirm_button(parent: &mut ChildSpawnerCommands, asset_server: &AssetServer) {
    let font = asset_server.load(FONT_PATH);

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(120.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(COLOR_BUTTON),
            ConfirmCreateButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("创建"),
                TextFont {
                    font,
                    font_size: 20.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));
        });
}

fn spawn_cancel_button(parent: &mut ChildSpawnerCommands, asset_server: &AssetServer) {
    let font = asset_server.load(FONT_PATH);

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(120.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(COLOR_BUTTON_EXIT),
            CancelCreateButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("取消"),
                TextFont {
                    font,
                    font_size: 20.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));
        });
}

fn spawn_deck_list_item(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    deck: &DeckSummary,
) {
    let font = asset_server.load(FONT_PATH);
    let deck_name = deck.name.clone();
    let card_count = deck.card_count;
    let file_path = deck.file_path.clone();
    let file_path_for_delete = file_path.clone();

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(500.0),
                height: Val::Px(60.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(20.0)),
                margin: UiRect::vertical(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(COLOR_BUTTON),
            DeckListItem {
                deck_name: deck_name.clone(),
                file_path: file_path.to_string_lossy().to_string(),
            },
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(deck_name),
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
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(15.0),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("{} 张", card_count)),
                        TextFont {
                            font: font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_DIM),
                    ));

                    spawn_delete_button(parent, asset_server, file_path_for_delete);
                });
        });
}

fn spawn_delete_button(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    file_path: PathBuf,
) {
    let font = asset_server.load(FONT_PATH);

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(60.0),
                height: Val::Px(35.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(COLOR_BUTTON_EXIT),
            DeckDeleteButton {
                file_path: file_path.to_string_lossy().to_string(),
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("删除"),
                TextFont {
                    font,
                    font_size: 16.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));
        });
}

pub fn handle_deck_list_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &DeckListItem),
        Changed<Interaction>,
    >,
    mut next_state: ResMut<NextState<AppState>>,
    mut selected_deck: ResMut<SelectedDeck>,
) {
    for (interaction, mut color, deck_item) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                selected_deck.name = deck_item.deck_name.clone();
                if let Ok(deck) = DeckManager::load(PathBuf::from(&deck_item.file_path).as_path()) {
                    selected_deck.cards = deck.cards.iter().map(|c| c.to_string()).collect();
                }
                next_state.set(AppState::DeckEditorDetail);
            }
            Interaction::Hovered => {
                *color = BackgroundColor(COLOR_BUTTON_HOVER);
            }
            Interaction::None => {
                *color = BackgroundColor(COLOR_BUTTON);
            }
        }
    }
}

pub fn handle_create_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<CreateDeckButton>),
    >,
    mut create_state: ResMut<CreateDeckState>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                create_state.is_inputting = true;
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.3, 0.9, 0.4));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.2, 0.8, 0.3));
            }
        }
    }
}

pub fn handle_text_input_submit(
    mut messages: MessageReader<TextInputSubmitMessage>,
    mut create_state: ResMut<CreateDeckState>,
    mut deck_list_data: ResMut<DeckListData>,
) {
    for message in messages.read() {
        let deck_name = message.value.trim();
        if !deck_name.is_empty() {
            create_deck(deck_name, &mut deck_list_data);
        }
        create_state.is_inputting = false;
    }
}

pub fn handle_confirm_create(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<ConfirmCreateButton>)>,
    mut create_state: ResMut<CreateDeckState>,
    mut deck_list_data: ResMut<DeckListData>,
    text_input_query: Query<&TextInputValue>,
) {
    for interaction in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            for text_value in text_input_query.iter() {
                let deck_name = text_value.0.trim();
                if !deck_name.is_empty() {
                    create_deck(deck_name, &mut deck_list_data);
                }
            }
            create_state.is_inputting = false;
        }
    }
}

fn create_deck(deck_name: &str, deck_list_data: &mut ResMut<DeckListData>) {
    let desk_dir = get_desks_directory();

    if !desk_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&desk_dir) {
            eprintln!("Failed to create desks directory: {}", e);
            return;
        }
    }

    let file_name = format!("{}.json", deck_name);
    let file_path = desk_dir.join(&file_name);

    let new_deck = Deck::new(deck_name);
    if let Err(e) = DeckManager::save(&file_path, &new_deck) {
        eprintln!("Failed to create deck: {}", e);
    } else {
        deck_list_data.decks = DeckManager::list_decks(&desk_dir).unwrap_or_default();
    }
}

pub fn handle_cancel_create(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<CancelCreateButton>)>,
    mut create_state: ResMut<CreateDeckState>,
) {
    for interaction in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            create_state.is_inputting = false;
        }
    }
}

pub fn handle_delete_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &DeckDeleteButton),
        Changed<Interaction>,
    >,
    mut deck_list_data: ResMut<DeckListData>,
) {
    for (interaction, mut color, delete_button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                let file_path = PathBuf::from(&delete_button.file_path);
                if let Err(e) = DeckManager::delete(&file_path) {
                    eprintln!("Failed to delete deck: {}", e);
                } else {
                    let desk_dir = get_desks_directory();
                    deck_list_data.decks = DeckManager::list_decks(&desk_dir).unwrap_or_default();
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(COLOR_BUTTON_EXIT_HOVER);
            }
            Interaction::None => {
                *color = BackgroundColor(COLOR_BUTTON_EXIT);
            }
        }
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
