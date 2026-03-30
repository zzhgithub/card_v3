use crate::colors::*;
use crate::ui_components::{spawn_return_button, spawn_status_bar, spawn_version_display};
use bevy::prelude::*;

pub fn enter_local_game(mut commands: Commands, asset_server: Res<AssetServer>) {
    spawn_version_display(&mut commands, &asset_server);
    spawn_status_bar(&mut commands, &asset_server, "本地游戏");
    spawn_return_button(&mut commands, &asset_server);

    let font = asset_server.load(FONT_PATH);
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
                Text::new("本地游戏页面\n（功能开发中）"),
                TextFont {
                    font,
                    font_size: 36.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
                TextLayout::new_with_justify(Justify::Center),
            ));
        });
}
