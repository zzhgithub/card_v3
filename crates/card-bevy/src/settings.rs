use crate::app_state::{AppState, ReturnButton};
use crate::colors::*;
use crate::ui_components::{spawn_return_button, spawn_status_bar, spawn_version_display};
use crate::GameServerInfo;
use bevy::prelude::*;
use bevy::text::Justify;

pub fn enter_settings(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    server_info: Res<GameServerInfo>,
) {
    spawn_version_display(&mut commands, &asset_server);
    spawn_status_bar(&mut commands, &asset_server, "设置");
    spawn_return_button(&mut commands, &asset_server);

    let font = asset_server.load(FONT_PATH);
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(30.0),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new("设置"),
                TextFont {
                    font: font.clone(),
                    font_size: 36.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
                TextLayout::new_with_justify(Justify::Center),
            ));

            parent.spawn((
                Text::new(format!("服务器地址: {}", server_info.bind_addr)),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
                TextLayout::new_with_justify(Justify::Center),
            ));

            parent.spawn((
                Text::new(format!("端口: {}", server_info.bind_addr.port())),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
                TextLayout::new_with_justify(Justify::Center),
            ));
        });
}

pub fn handle_return_button(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<ReturnButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<AppState>>,
) {
    for interaction in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::MainMenu);
            return;
        }
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        if **current_state != AppState::MainMenu && **current_state != AppState::SplashScreen {
            next_state.set(AppState::MainMenu);
        }
    }
}
