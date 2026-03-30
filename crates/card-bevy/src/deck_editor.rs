use crate::app_state::{AppState, DeckListItem, SelectedDeck};
use crate::colors::*;
use crate::ui_components::{spawn_return_button, spawn_status_bar, spawn_version_display};
use bevy::prelude::*;
use card_core::deck::{DeckManager, DeckSummary};
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
            parent.spawn((
                Text::new("选择卡组进行编辑"),
                TextFont {
                    font: font.clone(),
                    font_size: 32.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));

            parent.spawn(Node {
                height: Val::Px(30.0),
                ..default()
            });

            if deck_list_data.decks.is_empty() {
                parent.spawn((
                    Text::new("暂无卡组，请在 desks 目录下添加 .json 卡组文件"),
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

fn spawn_deck_list_item(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    deck: &DeckSummary,
) {
    let font = asset_server.load(FONT_PATH);
    let deck_name = deck.name.clone();
    let card_count = deck.card_count;
    let file_path = deck.file_path.clone();

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
                card_count,
                file_path: file_path.to_string_lossy().to_string(),
            },
        ))
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

            parent.spawn((
                Text::new(format!("{} 张卡牌", card_count)),
                TextFont {
                    font: font.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(COLOR_TEXT_DIM),
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

fn get_desks_directory() -> PathBuf {
    let candidates = [PathBuf::from("desks"), PathBuf::from("../../desks")];

    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    PathBuf::from("desks")
}
