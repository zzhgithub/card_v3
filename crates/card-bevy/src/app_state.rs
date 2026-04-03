use bevy::prelude::*;
use card_core::state::CardRegistryImpl;
use card_core::types::CardId;

#[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    SplashScreen,
    MainMenu,
    LocalGameSetup,
    LocalGame,
    OnlineGame,
    DeckEditor,
    DeckEditorDetail,
    Settings,
}

#[derive(Resource)]
pub struct AvailableCards {
    pub registry: CardRegistryImpl,
}

impl Default for AvailableCards {
    fn default() -> Self {
        Self {
            registry: CardRegistryImpl::new(),
        }
    }
}

#[derive(Resource, Default)]
pub struct SelectedCard {
    pub id: Option<CardId>,
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
    pub file_path: String,
}

#[derive(Component)]
pub struct DeckDeleteButton {
    pub file_path: String,
}

#[derive(Component)]
pub struct DeckListContainer;

#[derive(Component)]
pub struct CreateDeckButton;

#[derive(Component)]
pub struct CreateDeckModal;

#[derive(Component)]
pub struct ConfirmCreateButton;

#[derive(Component)]
pub struct CancelCreateButton;

#[derive(Component)]
pub struct ImeTextInput;

#[derive(Component)]
pub struct DeckDetailReturnButton;

#[derive(Component)]
pub struct SaveDeckButton;

#[derive(Component)]
pub struct CardListItem {
    pub card_id: CardId,
    pub is_in_deck: bool,
}

#[derive(Component)]
pub struct DeckCardItem {
    pub card_id: CardId,
    pub index: usize,
}

#[derive(Component)]
pub struct AddCardButton {
    pub card_id: CardId,
}

#[derive(Component)]
pub struct RemoveCardButton {
    pub index: usize,
}

#[derive(Component)]
pub struct CardDetailPanel;

#[derive(Component)]
pub struct CardImagePlaceholder;

#[derive(Component)]
pub struct CardInfoText;

#[derive(Component)]
pub struct CardEffectsText;

#[derive(Component)]
pub struct LeftPanel;

#[derive(Component)]
pub struct MiddlePanel;

#[derive(Component)]
pub struct RightPanel;

#[derive(Component)]
pub struct AvailableCardItem {
    pub card_id: CardId,
}

#[derive(Component)]
pub struct DeckCountText;

#[derive(Resource, Default)]
pub struct SelectedDeck {
    pub name: String,
    pub cards: Vec<String>,
    pub file_path: String,
}

#[derive(Resource, Default)]
pub struct CreateDeckState {
    pub is_inputting: bool,
}
