use crate::app_state::{AppState, SplashTimer, VersionText};
use crate::colors::*;
use crate::ui_components::spawn_version_display;
use bevy::prelude::*;
use bevy::text::Justify;

pub fn enter_splash_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(FONT_PATH);

    commands
        .spawn((
            SplashTimer(Timer::from_seconds(SPLASH_DURATION, TimerMode::Once)),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(COLOR_BACKGROUND),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("此游戏为测试阶段\n不代表最后品质"),
                TextFont {
                    font,
                    font_size: 48.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
                TextLayout::new_with_justify(Justify::Center),
            ));
        });

    spawn_version_display(&mut commands, &asset_server);
}

pub fn update_splash_screen(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut SplashTimer)>,
    mut next_state: ResMut<NextState<AppState>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space)
        || keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Escape)
    {
        next_state.set(AppState::MainMenu);
        return;
    }

    for (entity, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            commands.entity(entity).despawn();
            next_state.set(AppState::MainMenu);
        }
    }
}

pub fn exit_splash_screen(mut commands: Commands, query: Query<Entity, With<VersionText>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
