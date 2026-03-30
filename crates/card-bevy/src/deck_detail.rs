use crate::app_state::{AppState, DeckDetailReturnButton, SelectedDeck};
use crate::colors::*;
use crate::ui_components::{spawn_status_bar, spawn_version_display};
use bevy::prelude::*;

pub fn enter_deck_detail(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selected_deck: Res<SelectedDeck>,
) {
    spawn_version_display(&mut commands, &asset_server);
    spawn_status_bar(
        &mut commands,
        &asset_server,
        &format!("卡组详情 - {}", selected_deck.name),
    );

    let font = asset_server.load(FONT_PATH);

    commands
        .spawn((
            Button,
            Node {
                width: Val::Px(100.0),
                height: Val::Px(40.0),
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Px(5.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(COLOR_BUTTON),
            DeckDetailReturnButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("返回"),
                TextFont {
                    font: font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));
        });

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
                Text::new(format!("卡组名称: {}", selected_deck.name)),
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
                Text::new(format!("卡牌数量: {}", selected_deck.cards.len())),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(COLOR_TEXT_DIM),
            ));

            parent.spawn(Node {
                height: Val::Px(30.0),
                ..default()
            });

            if selected_deck.cards.is_empty() {
                parent.spawn((
                    Text::new("卡组为空"),
                    TextFont {
                        font: font.clone(),
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_DIM),
                ));
            } else {
                parent.spawn((
                    Text::new("卡牌列表:"),
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

                for (index, card_id) in selected_deck.cards.iter().enumerate() {
                    parent.spawn((
                        Text::new(format!("{}. {}", index + 1, card_id)),
                        TextFont {
                            font: font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_DIM),
                    ));
                }
            }
        });
}

pub fn handle_deck_detail_return_button(
    mut interaction_query: Query<
        &Interaction,
        (Changed<Interaction>, With<DeckDetailReturnButton>),
    >,
    mut next_state: ResMut<NextState<AppState>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for interaction in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::DeckEditor);
            return;
        }
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::DeckEditor);
    }
}
