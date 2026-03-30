use crate::app_state::{
    AppState, MenuAction, MenuButton, ReturnButton, StatusBarText, VersionText,
};
use crate::colors::*;
use bevy::prelude::*;

pub fn spawn_version_display(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn((
        Text::new(GAME_VERSION),
        TextFont {
            font: asset_server.load(FONT_PATH),
            font_size: 20.0,
            ..default()
        },
        TextColor(COLOR_TEXT_DIM),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(20.0),
            ..default()
        },
        VersionText,
    ));
}

pub fn spawn_status_bar(commands: &mut Commands, asset_server: &AssetServer, title: &str) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(50.0),
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new(title),
                TextFont {
                    font: asset_server.load(FONT_PATH),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
                StatusBarText,
            ));
        });
}

pub fn spawn_return_button(commands: &mut Commands, asset_server: &AssetServer) {
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
            ReturnButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("返回"),
                TextFont {
                    font: asset_server.load(FONT_PATH),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));
        });
}

pub fn spawn_menu_button(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    text: &str,
    target_state: Option<AppState>,
    action: Option<MenuAction>,
) {
    let is_exit = matches!(action, Some(MenuAction::Exit));
    let bg_color = if is_exit {
        COLOR_BUTTON_EXIT
    } else {
        COLOR_BUTTON
    };

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(300.0),
                height: Val::Px(60.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(bg_color),
            MenuButton {
                target_state,
                action,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(text),
                TextFont {
                    font: asset_server.load(FONT_PATH),
                    font_size: 28.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));
        });
}

pub fn cleanup_ui(
    mut commands: Commands,
    ui_query: Query<Entity, Or<(With<Node>, With<Text>, With<Button>)>>,
) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn();
    }
}
