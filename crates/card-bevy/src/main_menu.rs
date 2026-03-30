use crate::app_state::{AppState, MenuAction, MenuButton};
use crate::colors::*;
use crate::ui_components::{spawn_menu_button, spawn_status_bar, spawn_version_display};
use bevy::prelude::*;

pub fn enter_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    spawn_version_display(&mut commands, &asset_server);
    spawn_status_bar(&mut commands, &asset_server, "主菜单");

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(20.0),
            ..default()
        })
        .with_children(|parent| {
            spawn_menu_button(
                parent,
                &asset_server,
                "本地游戏",
                Some(AppState::LocalGame),
                None,
            );
            spawn_menu_button(
                parent,
                &asset_server,
                "联机游戏",
                Some(AppState::OnlineGame),
                None,
            );
            spawn_menu_button(
                parent,
                &asset_server,
                "卡组编辑",
                Some(AppState::DeckEditor),
                None,
            );
            spawn_menu_button(
                parent,
                &asset_server,
                "设置",
                Some(AppState::Settings),
                None,
            );
            spawn_menu_button(parent, &asset_server, "退出", None, Some(MenuAction::Exit));
        });
}

pub fn handle_menu_buttons(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &MenuButton),
        Changed<Interaction>,
    >,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    for (interaction, mut color, button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                if let Some(target) = button.target_state {
                    next_state.set(target);
                }
                if let Some(MenuAction::Exit) = button.action {
                    commands.write_message(AppExit::Success);
                }
            }
            Interaction::Hovered => {
                let is_exit = matches!(button.action, Some(MenuAction::Exit));
                let hover_color = if is_exit {
                    COLOR_BUTTON_EXIT_HOVER
                } else {
                    COLOR_BUTTON_HOVER
                };
                *color = BackgroundColor(hover_color);
            }
            Interaction::None => {
                let is_exit = matches!(button.action, Some(MenuAction::Exit));
                let normal_color = if is_exit {
                    COLOR_BUTTON_EXIT
                } else {
                    COLOR_BUTTON
                };
                *color = BackgroundColor(normal_color);
            }
        }
    }
}

pub fn handle_keyboard_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut button_query: Query<(Entity, &mut BackgroundColor, &MenuButton), With<Button>>,
    mut selected: Local<usize>,
) {
    let button_count = button_query.iter().count();
    if button_count == 0 {
        return;
    }

    let mut changed = false;
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        *selected = (*selected + 1) % button_count;
        changed = true;
    }
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        if *selected == 0 {
            *selected = button_count - 1;
        } else {
            *selected -= 1;
        }
        changed = true;
    }

    if changed {
        for (i, (_, mut color, button)) in button_query.iter_mut().enumerate() {
            if i == *selected {
                let is_exit = matches!(button.action, Some(MenuAction::Exit));
                let hover_color = if is_exit {
                    COLOR_BUTTON_EXIT_HOVER
                } else {
                    COLOR_BUTTON_HOVER
                };
                *color = BackgroundColor(hover_color);
            } else {
                let is_exit = matches!(button.action, Some(MenuAction::Exit));
                let normal_color = if is_exit {
                    COLOR_BUTTON_EXIT
                } else {
                    COLOR_BUTTON
                };
                *color = BackgroundColor(normal_color);
            }
        }
    }
}
