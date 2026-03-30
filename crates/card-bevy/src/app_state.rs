use bevy::prelude::*;

#[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    SplashScreen,
    MainMenu,
    LocalGame,
    OnlineGame,
    DeckEditor,
    DeckEditorDetail,
    Settings,
}

#[derive(Clone, Copy)]
pub enum MenuAction {
    Exit,
}

#[derive(Component)]
pub struct MenuButton {
    pub target_state: Option<AppState>,
    pub action: Option<MenuAction>,
}

#[derive(Component)]
pub struct SplashTimer(pub Timer);

#[derive(Component)]
pub struct VersionText;

#[derive(Component)]
pub struct StatusBarText;

#[derive(Component)]
pub struct ReturnButton;

#[derive(Component)]
pub struct DeckListItem {
    pub deck_name: String,
    pub card_count: usize,
    pub file_path: String,
}

#[derive(Component)]
pub struct DeckDetailReturnButton;

#[derive(Resource, Default)]
pub struct SelectedDeck {
    pub name: String,
    pub cards: Vec<String>,
}
