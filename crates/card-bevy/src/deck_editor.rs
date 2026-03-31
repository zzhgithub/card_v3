use crate::app_state::{
    CancelCreateButton, ConfirmCreateButton, CreateDeckButton, CreateDeckModal, CreateDeckState,
    DeckDeleteButton, DeckListContainer, DeckListItem, ImeTextInput, SelectedDeck,
};
use crate::colors::*;
use crate::ui_components::{spawn_return_button, spawn_status_bar, spawn_version_display};
use crate::AppState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::Ime;

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

            // Deck list container - will be rebuilt when data changes
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    DeckListContainer,
                ))
                .with_children(|parent| {
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
        });
}

pub fn spawn_modal_on_demand(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    create_state: Res<CreateDeckState>,
    modal_query: Query<Entity, With<CreateDeckModal>>,
) {
    if create_state.is_changed() {
        if create_state.is_inputting && modal_query.is_empty() {
            spawn_create_input_ui(&mut commands, &asset_server);
        }
    }
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

fn spawn_create_input_ui(commands: &mut Commands, asset_server: &AssetServer) {
    let font = asset_server.load(FONT_PATH);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            CreateDeckModal,
            ZIndex(100),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(30.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("创建新卡组"),
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

                    parent.spawn((
                        Text::new("输入卡组名称:"),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_DIM),
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
                                Text::new(""),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT),
                                ImeTextInput,
                            ));
                        });

                    parent.spawn(Node {
                        height: Val::Px(20.0),
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

pub fn handle_confirm_create(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<ConfirmCreateButton>)>,
    mut create_state: ResMut<CreateDeckState>,
    mut deck_list_data: ResMut<DeckListData>,
    mut selected_deck: ResMut<SelectedDeck>,
    mut next_state: ResMut<NextState<AppState>>,
    text_query: Query<&Text, With<ImeTextInput>>,
) {
    for interaction in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            for text in text_query.iter() {
                let deck_name = text.0.trim();
                if !deck_name.is_empty() {
                    if let Some(file_path) = create_deck(deck_name, &mut deck_list_data) {
                        selected_deck.name = deck_name.to_string();
                        if let Ok(deck) = DeckManager::load(&file_path) {
                            selected_deck.cards =
                                deck.cards.iter().map(|c| c.to_string()).collect();
                        }
                        next_state.set(AppState::DeckEditorDetail);
                    }
                }
            }
            create_state.is_inputting = false;
        }
    }
}

fn create_deck(deck_name: &str, deck_list_data: &mut ResMut<DeckListData>) -> Option<PathBuf> {
    let desk_dir = get_desks_directory();

    if !desk_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&desk_dir) {
            eprintln!("Failed to create desks directory: {}", e);
            return None;
        }
    }

    let file_name = format!("{}.json", deck_name);
    let file_path = desk_dir.join(&file_name);

    let new_deck = Deck::new(deck_name);
    if let Err(e) = DeckManager::save(&file_path, &new_deck) {
        eprintln!("Failed to create deck: {}", e);
        None
    } else {
        deck_list_data.decks = DeckManager::list_decks(&desk_dir).unwrap_or_default();
        Some(file_path)
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

pub fn cleanup_create_deck_modal(
    mut commands: Commands,
    create_state: Res<CreateDeckState>,
    modal_query: Query<Entity, With<CreateDeckModal>>,
) {
    if !create_state.is_inputting {
        for entity in modal_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn rebuild_deck_list(
    mut commands: Commands,
    deck_list_data: Res<DeckListData>,
    asset_server: Res<AssetServer>,
    container_query: Query<(Entity, Option<&Children>), With<DeckListContainer>>,
) {
    if deck_list_data.is_changed() {
        let font = asset_server.load(FONT_PATH);

        for (container_entity, children) in container_query.iter() {
            if let Some(children) = children {
                for child in children.iter() {
                    commands.entity(child).despawn();
                }
            }

            commands.entity(container_entity).with_children(|parent| {
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
    }
}

/// Handle keyboard and IME events for text input
/// Supports both regular keyboard input and Chinese IME composition
pub fn handle_ime_text_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut ime_events: MessageReader<Ime>,
    mut text_query: Query<&mut Text, With<ImeTextInput>>,
    key_input: Res<ButtonInput<KeyCode>>,
) {
    // Track if IME is currently composing (preedit state)
    static mut IME_COMPOSING: bool = false;

    for event in ime_events.read() {
        match event {
            Ime::Preedit { .. } => unsafe {
                IME_COMPOSING = true;
            },
            Ime::Commit { value, .. } => {
                unsafe {
                    IME_COMPOSING = false;
                }
                for mut text in text_query.iter_mut() {
                    text.0.push_str(value);
                }
            }
            Ime::Enabled { .. } => unsafe {
                IME_COMPOSING = true;
            },
            Ime::Disabled { .. } => unsafe {
                IME_COMPOSING = false;
            },
        }
    }

    // Only process keyboard input when not in IME composition mode
    let composing = unsafe { IME_COMPOSING };
    if !composing {
        for event in key_events.read() {
            if !event.state.is_pressed() {
                continue;
            }

            for mut text in text_query.iter_mut() {
                match event.key_code {
                    KeyCode::Backspace => {
                        text.0.pop();
                    }
                    _ => {
                        // Handle regular character input when not using IME
                        if let Some(char) = keycode_to_char(
                            event.key_code,
                            key_input.pressed(KeyCode::ShiftLeft)
                                || key_input.pressed(KeyCode::ShiftRight),
                        ) {
                            text.0.push(char);
                        }
                    }
                }
            }
        }
    }
}

/// Convert KeyCode to character (basic implementation for non-IME input)
fn keycode_to_char(key_code: KeyCode, shift: bool) -> Option<char> {
    match key_code {
        KeyCode::KeyA => Some(if shift { 'A' } else { 'a' }),
        KeyCode::KeyB => Some(if shift { 'B' } else { 'b' }),
        KeyCode::KeyC => Some(if shift { 'C' } else { 'c' }),
        KeyCode::KeyD => Some(if shift { 'D' } else { 'd' }),
        KeyCode::KeyE => Some(if shift { 'E' } else { 'e' }),
        KeyCode::KeyF => Some(if shift { 'F' } else { 'f' }),
        KeyCode::KeyG => Some(if shift { 'G' } else { 'g' }),
        KeyCode::KeyH => Some(if shift { 'H' } else { 'h' }),
        KeyCode::KeyI => Some(if shift { 'I' } else { 'i' }),
        KeyCode::KeyJ => Some(if shift { 'J' } else { 'j' }),
        KeyCode::KeyK => Some(if shift { 'K' } else { 'k' }),
        KeyCode::KeyL => Some(if shift { 'L' } else { 'l' }),
        KeyCode::KeyM => Some(if shift { 'M' } else { 'm' }),
        KeyCode::KeyN => Some(if shift { 'N' } else { 'n' }),
        KeyCode::KeyO => Some(if shift { 'O' } else { 'o' }),
        KeyCode::KeyP => Some(if shift { 'P' } else { 'p' }),
        KeyCode::KeyQ => Some(if shift { 'Q' } else { 'q' }),
        KeyCode::KeyR => Some(if shift { 'R' } else { 'r' }),
        KeyCode::KeyS => Some(if shift { 'S' } else { 's' }),
        KeyCode::KeyT => Some(if shift { 'T' } else { 't' }),
        KeyCode::KeyU => Some(if shift { 'U' } else { 'u' }),
        KeyCode::KeyV => Some(if shift { 'V' } else { 'v' }),
        KeyCode::KeyW => Some(if shift { 'W' } else { 'w' }),
        KeyCode::KeyX => Some(if shift { 'X' } else { 'x' }),
        KeyCode::KeyY => Some(if shift { 'Y' } else { 'y' }),
        KeyCode::KeyZ => Some(if shift { 'Z' } else { 'z' }),
        KeyCode::Digit0 => Some(if shift { ')' } else { '0' }),
        KeyCode::Digit1 => Some(if shift { '!' } else { '1' }),
        KeyCode::Digit2 => Some(if shift { '@' } else { '2' }),
        KeyCode::Digit3 => Some(if shift { '#' } else { '3' }),
        KeyCode::Digit4 => Some(if shift { '$' } else { '4' }),
        KeyCode::Digit5 => Some(if shift { '%' } else { '5' }),
        KeyCode::Digit6 => Some(if shift { '^' } else { '6' }),
        KeyCode::Digit7 => Some(if shift { '&' } else { '7' }),
        KeyCode::Digit8 => Some(if shift { '*' } else { '8' }),
        KeyCode::Digit9 => Some(if shift { '(' } else { '9' }),
        KeyCode::Space => Some(' '),
        KeyCode::Minus => Some(if shift { '_' } else { '-' }),
        KeyCode::Equal => Some(if shift { '+' } else { '=' }),
        KeyCode::BracketLeft => Some(if shift { '{' } else { '[' }),
        KeyCode::BracketRight => Some(if shift { '}' } else { ']' }),
        KeyCode::Backslash => Some(if shift { '|' } else { '\\' }),
        KeyCode::Semicolon => Some(if shift { ':' } else { ';' }),
        KeyCode::Quote => Some(if shift { '"' } else { '\'' }),
        KeyCode::Comma => Some(if shift { '<' } else { ',' }),
        KeyCode::Period => Some(if shift { '>' } else { '.' }),
        KeyCode::Slash => Some(if shift { '?' } else { '/' }),
        KeyCode::Backquote => Some(if shift { '~' } else { '`' }),
        _ => None,
    }
}
