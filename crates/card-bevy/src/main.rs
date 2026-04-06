mod app_state;
mod colors;
mod deck_detail;
mod deck_editor;
mod game_board;
mod local_game;
mod local_game_setup;
mod main_menu;
mod online_game;
mod settings;
mod splash;
mod ui_components;

#[derive(Resource)]
pub struct GameServerInfo {
    pub bind_addr: SocketAddr,
    pub runtime: Runtime,
    pub script_index: ScriptIndex,
}

use crate::app_state::{AppState, AvailableCards, CreateDeckState, SelectedCard, SelectedDeck};
use crate::deck_detail::{
    cleanup_deck_detail, enter_deck_detail, handle_add_card_button, handle_available_card_click,
    handle_available_card_hover, handle_deck_card_click, handle_deck_card_hover,
    handle_deck_detail_return_button, handle_remove_card_button, handle_save_deck_button,
    load_available_cards, on_scroll_handler, rebuild_middle_panel, send_scroll_events,
    update_deck_count, update_left_panel,
};
use crate::deck_editor::{
    cleanup_create_deck_modal, enter_deck_editor, handle_cancel_create, handle_confirm_create,
    handle_create_button, handle_deck_list_interaction, handle_delete_button,
    handle_ime_text_input, rebuild_deck_list, spawn_modal_on_demand, DeckListData,
};
use crate::game_board::GameBoardPlugin;
use crate::local_game::{
    cleanup_local_game, enter_local_game, handle_action_button_click, update_local_game,
};
use crate::local_game_setup::{
    cleanup_local_game_setup, enter_local_game_setup, handle_deck_selection,
    handle_start_game_button, LocalGameSetupState,
};
use crate::main_menu::{enter_main_menu, handle_keyboard_navigation, handle_menu_buttons};
use crate::online_game::enter_online_game;
use crate::settings::{enter_settings, handle_return_button};
use crate::splash::{enter_splash_screen, exit_splash_screen, update_splash_screen};
use crate::ui_components::cleanup_ui;
use bevy::prelude::*;
use bevy_simple_text_input::TextInputPlugin;
use card_core::rules::GameRules;
use card_script::loader::ScriptIndex;
use card_server::GameServer;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;

fn main() {
    // Create tokio runtime for async server operations
    let runtime = Runtime::new().expect("Failed to create Tokio runtime");

    // Create script index by scanning the scripts directory
    let script_index = ScriptIndex::scan(std::path::Path::new("scripts"))
        .expect("Failed to scan scripts directory");

    // Start TCP listener to get bind address for display
    let bind_addr = runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind to address");
        let addr = listener.local_addr().expect("Failed to get local address");
        drop(listener);
        addr
    });

    let server_info = GameServerInfo {
        bind_addr,
        runtime,
        script_index,
    };

    build_app(server_info).run();
}

fn build_app(server_info: GameServerInfo) -> App {
    let mut app = App::new();

    #[cfg(not(test))]
    app.add_plugins((
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_root().display().to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Card Game".to_string(),
                    mode: bevy::window::WindowMode::BorderlessFullscreen(
                        bevy::window::MonitorSelection::Current,
                    ),
                    ime_enabled: true,
                    ..default()
                }),
                ..default()
            }),
            TextInputPlugin,
            GameBoardPlugin,
        ));

    #[cfg(test)]
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin, GameBoardPlugin));

    app
        .init_state::<AppState>()
        .init_resource::<SelectedDeck>()
        .init_resource::<LocalGameSetupState>()
        .init_resource::<DeckListData>()
        .init_resource::<CreateDeckState>()
        .init_resource::<AvailableCards>()
        .init_resource::<SelectedCard>()
        .insert_resource(server_info)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            update_splash_screen.run_if(in_state(AppState::SplashScreen)),
        )
        .add_systems(OnEnter(AppState::SplashScreen), enter_splash_screen)
        .add_systems(OnExit(AppState::SplashScreen), exit_splash_screen)
        .add_systems(OnEnter(AppState::MainMenu), enter_main_menu)
        .add_systems(OnExit(AppState::MainMenu), cleanup_ui)
        .add_systems(OnEnter(AppState::LocalGame), enter_local_game)
        .add_systems(
            Update,
            (update_local_game, handle_action_button_click)
                .chain()
                .run_if(in_state(AppState::LocalGame)),
        )
        .add_systems(OnExit(AppState::LocalGame), cleanup_local_game)
        .add_systems(OnEnter(AppState::LocalGameSetup), enter_local_game_setup)
        .add_systems(OnExit(AppState::LocalGameSetup), cleanup_local_game_setup)
        .add_systems(OnEnter(AppState::OnlineGame), enter_online_game)
        .add_systems(OnExit(AppState::OnlineGame), cleanup_ui)
        .add_systems(OnEnter(AppState::DeckEditor), enter_deck_editor)
        .add_systems(OnExit(AppState::DeckEditor), cleanup_ui)
        .add_systems(
            OnEnter(AppState::DeckEditorDetail),
            (load_available_cards, enter_deck_detail).chain(),
        )
        .add_systems(OnExit(AppState::DeckEditorDetail), cleanup_deck_detail)
        .add_observer(on_scroll_handler)
        .add_systems(Update, send_scroll_events)
        .add_systems(OnEnter(AppState::Settings), enter_settings)
        .add_systems(OnExit(AppState::Settings), cleanup_ui)
        .add_systems(Update, handle_menu_buttons)
        .add_systems(
            Update,
            handle_deck_selection.run_if(in_state(AppState::LocalGameSetup)),
        )
        .add_systems(
            Update,
            handle_start_game_button.run_if(in_state(AppState::LocalGameSetup)),
        )
        .add_systems(Update, handle_return_button)
        .add_systems(Update, handle_keyboard_navigation)
        .add_systems(
            Update,
            handle_deck_list_interaction.run_if(in_state(AppState::DeckEditor)),
        )
        .add_systems(
            Update,
            handle_deck_detail_return_button.run_if(in_state(AppState::DeckEditorDetail)),
        )
        .add_systems(
            Update,
            handle_save_deck_button.run_if(in_state(AppState::DeckEditorDetail)),
        )
        .add_systems(
            Update,
            handle_add_card_button.run_if(in_state(AppState::DeckEditorDetail)),
        )
        .add_systems(
            Update,
            handle_remove_card_button.run_if(in_state(AppState::DeckEditorDetail)),
        )
        .add_systems(
            Update,
            handle_deck_card_click.run_if(in_state(AppState::DeckEditorDetail)),
        )
        .add_systems(
            Update,
            handle_available_card_click.run_if(in_state(AppState::DeckEditorDetail)),
        )
        .add_systems(
            Update,
            handle_available_card_hover.run_if(in_state(AppState::DeckEditorDetail)),
        )
        .add_systems(
            Update,
            handle_deck_card_hover.run_if(in_state(AppState::DeckEditorDetail)),
        )
        .add_systems(
            Update,
            update_deck_count.run_if(in_state(AppState::DeckEditorDetail)),
        )
        .add_systems(
            Update,
            rebuild_middle_panel.run_if(in_state(AppState::DeckEditorDetail)),
        )
        .add_systems(
            Update,
            update_left_panel.run_if(in_state(AppState::DeckEditorDetail)),
        )
        .add_systems(
            Update,
            handle_create_button.run_if(in_state(AppState::DeckEditor)),
        )
        .add_systems(
            Update,
            spawn_modal_on_demand.run_if(in_state(AppState::DeckEditor)),
        )
        .add_systems(
            Update,
            handle_ime_text_input.run_if(in_state(AppState::DeckEditor)),
        )
        .add_systems(
            Update,
            handle_confirm_create.run_if(in_state(AppState::DeckEditor)),
        )
        .add_systems(
            Update,
            handle_cancel_create.run_if(in_state(AppState::DeckEditor)),
        )
        .add_systems(
            Update,
            handle_delete_button.run_if(in_state(AppState::DeckEditor)),
        )
        .add_systems(
            Update,
            rebuild_deck_list.run_if(in_state(AppState::DeckEditor)),
        )
        .add_systems(
            Update,
            cleanup_create_deck_modal.run_if(in_state(AppState::DeckEditor)),
        );

    app
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn asset_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_board::BoardLayout;

    #[test]
    fn build_app_registers_board_layout_resource() {
        let runtime = Runtime::new().expect("runtime");
        let script_index = ScriptIndex::scan(Path::new("/definitely/missing/scripts")).expect("scan");
        let server_info = GameServerInfo {
            bind_addr: "127.0.0.1:0".parse().expect("addr"),
            runtime,
            script_index,
        };

        let app = build_app(server_info);

        assert!(app.world().contains_resource::<BoardLayout>());
    }

    #[test]
    fn asset_root_points_to_existing_directory() {
        assert!(asset_root().exists());
    }
}
