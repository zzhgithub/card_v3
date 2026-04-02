mod app_state;
mod colors;
mod deck_detail;
mod deck_editor;
mod local_game;
mod main_menu;
mod online_game;
mod settings;
mod splash;
mod ui_components;

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
use crate::local_game::enter_local_game;
use crate::main_menu::{enter_main_menu, handle_keyboard_navigation, handle_menu_buttons};
use crate::online_game::enter_online_game;
use crate::settings::{enter_settings, handle_return_button};
use crate::splash::{enter_splash_screen, exit_splash_screen, update_splash_screen};
use crate::ui_components::cleanup_ui;
use bevy::prelude::*;
use bevy_simple_text_input::TextInputPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Card Game".to_string(),
                    mode: bevy::window::WindowMode::BorderlessFullscreen(
                        bevy::window::MonitorSelection::Current,
                    ),
                    // Enable IME support for Chinese input
                    ime_enabled: true,
                    ..default()
                }),
                ..default()
            }),
            TextInputPlugin,
        ))
        .init_state::<AppState>()
        .init_resource::<SelectedDeck>()
        .init_resource::<DeckListData>()
        .init_resource::<CreateDeckState>()
        .init_resource::<AvailableCards>()
        .init_resource::<SelectedCard>()
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
        .add_systems(OnExit(AppState::LocalGame), cleanup_ui)
        .add_systems(OnEnter(AppState::OnlineGame), enter_online_game)
        .add_systems(OnExit(AppState::OnlineGame), cleanup_ui)
        .add_systems(OnEnter(AppState::DeckEditor), enter_deck_editor)
        .add_systems(OnExit(AppState::DeckEditor), cleanup_ui)
        .add_systems(OnExit(AppState::DeckEditorDetail), cleanup_deck_detail)
        .add_observer(on_scroll_handler)
        .add_systems(Update, send_scroll_events)
        .add_systems(OnExit(AppState::DeckEditorDetail), cleanup_deck_detail)
        .add_systems(OnEnter(AppState::Settings), enter_settings)
        .add_systems(OnExit(AppState::Settings), cleanup_ui)
        .add_systems(Update, handle_menu_buttons)
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
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
