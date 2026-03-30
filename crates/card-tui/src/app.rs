use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use card_client::api::{game_state_to_visible, VisibleGameState};
use card_core::deck::{Deck, DeckManager, DeckSummary};
use card_core::engine::phase::{PhaseAction, PhaseClient, RecoveryCardOption};
use card_core::engine::GameResult;
use card_core::rules::GameRules;
use card_core::types::{CardId, InstanceId, PlayerId, Zone};
use card_script::loader::{ScriptIndex, ScriptLoader};
use card_server::AiClient;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::backend::Backend;
use ratatui::{Frame, Terminal};
use tracing::{error, info};

use crate::cursor::FieldCursor;
use crate::game_setup;
use crate::input::{self, InputAction, InputMode};
use crate::network;
use crate::ui::log::LogState;
use crate::ui::overlay::OverlayState;
use crate::ui::{self, ActionOverlayData, RecoveryOverlayData};

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum UiEvent {
    ActionPrompt(Vec<PhaseAction>),
    RecoveryPrompt {
        count: usize,
        options: Vec<InstanceId>,
    },
    StateUpdate(VisibleGameState),
    Log(String),
    EnterOnlineBattle {
        opponent_name: String,
    },
}

#[derive(Debug, Clone)]
pub enum AppMode {
    MainMenu,
    DeckBrowser,
    DeckSelection,
    DeckEditor,
    LocalGame,
    OnlineGame,
    InGame,
    GameOver(Option<PlayerId>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionPhase {
    PickPlayerDeck,
    PickAiDeck,
}

pub struct TuiClient {
    action_rx: Mutex<mpsc::Receiver<PhaseAction>>,
    recovery_rx: Mutex<mpsc::Receiver<Vec<InstanceId>>>,
    event_tx: mpsc::Sender<UiEvent>,
}

impl TuiClient {
    pub fn new(
        action_rx: mpsc::Receiver<PhaseAction>,
        recovery_rx: mpsc::Receiver<Vec<InstanceId>>,
        event_tx: mpsc::Sender<UiEvent>,
    ) -> Self {
        Self {
            action_rx: Mutex::new(action_rx),
            recovery_rx: Mutex::new(recovery_rx),
            event_tx,
        }
    }
}

impl PhaseClient for TuiClient {
    fn choose_action(&self, available: &[PhaseAction], _timeout: Duration) -> Option<PhaseAction> {
        let _ = self
            .event_tx
            .send(UiEvent::ActionPrompt(available.to_vec()));
        self.action_rx.lock().expect("action_rx lock").recv().ok()
    }

    fn choose_recovery_cards(
        &self,
        options: &[RecoveryCardOption],
        count: usize,
        _timeout: Duration,
    ) -> Option<Vec<InstanceId>> {
        let option_ids = options.iter().map(|o| o.instance_id).collect::<Vec<_>>();
        let _ = self.event_tx.send(UiEvent::RecoveryPrompt {
            count,
            options: option_ids,
        });
        self.recovery_rx
            .lock()
            .expect("recovery_rx lock")
            .recv()
            .ok()
    }
}

pub struct App {
    mode: AppMode,
    selected_index: usize,
    log_state: LogState,
    should_quit: bool,
    ui_event_rx: Option<mpsc::Receiver<UiEvent>>,
    action_tx: Option<mpsc::Sender<PhaseAction>>,
    recovery_tx: Option<mpsc::Sender<Vec<InstanceId>>>,
    game_result_rx: Option<mpsc::Receiver<GameResult>>,
    visible_state: Option<VisibleGameState>,
    pending_actions: Option<Vec<PhaseAction>>,
    pending_recovery: Option<(usize, Vec<InstanceId>)>,
    selected_action_index: usize,
    selected_recovery_index: usize,
    selected_recovery_cards: HashSet<InstanceId>,
    game_title: String,
    deck_list: Vec<DeckSummary>,
    deck_selected: usize,
    selection_phase: SelectionPhase,
    player_deck_path: Option<PathBuf>,
    ai_deck_path: Option<PathBuf>,
    editing_deck: Option<Deck>,
    editing_deck_path: Option<PathBuf>,
    all_card_defs: Vec<card_core::types::CardDefinition>,
    editor_focus_right: bool,
    editor_left_index: usize,
    editor_right_index: usize,
    field_cursor: FieldCursor,
    overlay_state: OverlayState,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: AppMode::MainMenu,
            selected_index: 0,
            log_state: LogState::new(),
            should_quit: false,
            ui_event_rx: None,
            action_tx: None,
            recovery_tx: None,
            game_result_rx: None,
            visible_state: None,
            pending_actions: None,
            pending_recovery: None,
            selected_action_index: 0,
            selected_recovery_index: 0,
            selected_recovery_cards: HashSet::new(),
            game_title: String::new(),
            deck_list: Vec::new(),
            deck_selected: 0,
            selection_phase: SelectionPhase::PickPlayerDeck,
            player_deck_path: None,
            ai_deck_path: None,
            editing_deck: None,
            editing_deck_path: None,
            all_card_defs: Vec::new(),
            editor_focus_right: false,
            editor_left_index: 0,
            editor_right_index: 0,
            field_cursor: FieldCursor::new(),
            overlay_state: OverlayState::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        loop {
            self.drain_game_events();
            self.drain_game_result();

            terminal.draw(|frame| self.render(frame))?;

            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
            {
                self.handle_key(key)?;
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame<'_>) {
        match self.mode {
            AppMode::MainMenu => ui::render_main_menu(frame, self.selected_index),
            AppMode::DeckBrowser => {
                ui::render_deck_browser(frame, &self.deck_list, self.deck_selected)
            }
            AppMode::DeckSelection => {
                let player_deck_name = self.player_deck_path.as_ref().and_then(|p| {
                    self.deck_list
                        .iter()
                        .find(|s| &s.file_path == p)
                        .map(|s| s.name.as_str())
                });
                ui::render_deck_selection(
                    frame,
                    &self.selection_phase,
                    player_deck_name,
                    &self.deck_list,
                    self.deck_selected,
                )
            }
            AppMode::DeckEditor => ui::render_deck_editor(
                frame,
                self.editing_deck.as_ref(),
                &self.all_card_defs,
                self.editor_focus_right,
                self.editor_left_index,
                self.editor_right_index,
            ),
            AppMode::LocalGame | AppMode::OnlineGame | AppMode::InGame | AppMode::GameOver(_) => {
                let action_descriptions = self.pending_actions.as_ref().map(|actions| {
                    actions
                        .iter()
                        .map(|action| self.describe_action(action))
                        .collect::<Vec<_>>()
                });
                let recovery_descriptions = self.pending_recovery.as_ref().map(|(_, options)| {
                    options
                        .iter()
                        .map(|instance_id| self.describe_instance(*instance_id))
                        .collect::<Vec<_>>()
                });

                let action_overlay =
                    self.pending_actions
                        .as_deref()
                        .map(|actions| ActionOverlayData {
                            actions,
                            selected_action_index: self.selected_action_index,
                            action_descriptions: action_descriptions.as_deref().unwrap_or(&[]),
                        });
                let recovery_overlay =
                    self.pending_recovery
                        .as_ref()
                        .map(|(count, options)| RecoveryOverlayData {
                            count: *count,
                            options,
                            selected_recovery_index: self.selected_recovery_index,
                            selected_recovery_cards: &self.selected_recovery_cards,
                            option_descriptions: recovery_descriptions.as_deref().unwrap_or(&[]),
                        });

                ui::render_game_screen(
                    frame,
                    &self.mode,
                    &self.game_title,
                    self.visible_state.as_ref(),
                    &self.log_state,
                    &self.field_cursor,
                    &self.all_card_defs,
                    action_overlay,
                    recovery_overlay,
                    &self.overlay_state,
                );
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if let AppMode::GameOver(_) = self.mode {
            match key.code {
                KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                    self.reset_to_main_menu();
                }
                _ => {}
            }
            return Ok(());
        }

        if matches!(key.code, KeyCode::Char('q')) {
            if let AppMode::MainMenu = self.mode {
                self.should_quit = true;
            }
            return Ok(());
        }

        if self.overlay_state.visible && matches!(key.code, KeyCode::Char('m') | KeyCode::Char('M'))
        {
            self.overlay_state.toggle_minimized();
            return Ok(());
        }

        if self.pending_actions.is_some()
            && !(self.overlay_state.visible && self.overlay_state.minimized)
        {
            self.handle_action_key(key);
            return Ok(());
        }

        if self.pending_recovery.is_some()
            && !(self.overlay_state.visible && self.overlay_state.minimized)
        {
            self.handle_recovery_key(key);
            return Ok(());
        }

        if self.is_field_cursor_mode_available() && self.handle_field_cursor_key(key) {
            return Ok(());
        }

        match self.mode {
            AppMode::MainMenu => match key.code {
                KeyCode::Up => {
                    self.selected_index = self.selected_index.saturating_sub(1);
                }
                KeyCode::Down => {
                    self.selected_index = (self.selected_index + 1).min(3);
                }
                KeyCode::Enter => match self.selected_index {
                    0 => self.enter_deck_selection()?,
                    1 => self.start_online_game()?,
                    2 => self.open_deck_browser()?,
                    _ => self.should_quit = true,
                },
                _ => {}
            },
            AppMode::DeckBrowser => match key.code {
                KeyCode::Up => {
                    self.deck_selected = self.deck_selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    let max = self.deck_list.len().saturating_sub(1);
                    self.deck_selected = (self.deck_selected + 1).min(max);
                }
                KeyCode::Enter => {
                    self.open_deck_editor()?;
                }
                KeyCode::Esc => {
                    self.reset_to_main_menu();
                }
                _ => {}
            },
            AppMode::DeckEditor => match key.code {
                KeyCode::Tab => {
                    self.editor_focus_right = !self.editor_focus_right;
                }
                KeyCode::Up => {
                    if self.editor_focus_right {
                        self.editor_right_index = self.editor_right_index.saturating_sub(1);
                    } else {
                        self.editor_left_index = self.editor_left_index.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if self.editor_focus_right {
                        let max = self.all_card_defs.len().saturating_sub(1);
                        self.editor_right_index = (self.editor_right_index + 1).min(max);
                    } else {
                        let max = self
                            .editing_deck
                            .as_ref()
                            .map(|d| {
                                let mut seen = std::collections::HashSet::new();
                                let count = d.cards.iter().filter(|id| seen.insert(&id.0)).count();
                                count.saturating_sub(1)
                            })
                            .unwrap_or(0);
                        self.editor_left_index = (self.editor_left_index + 1).min(max);
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.editor_add_card();
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.editor_remove_card();
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.editor_save_deck();
                }
                KeyCode::Esc => {
                    self.mode = AppMode::DeckBrowser;
                }
                _ => {}
            },
            AppMode::DeckSelection => match key.code {
                KeyCode::Up => {
                    self.deck_selected = self.deck_selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    let max = self.deck_list.len().saturating_sub(1);
                    self.deck_selected = (self.deck_selected + 1).min(max);
                }
                KeyCode::Enter => {
                    self.confirm_deck_selection()?;
                }
                KeyCode::Esc => {
                    self.reset_to_main_menu();
                }
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }

    fn reset_to_main_menu(&mut self) {
        self.mode = AppMode::MainMenu;
        self.selected_index = 0;
        self.log_state = LogState::new();
        self.visible_state = None;
        self.pending_actions = None;
        self.pending_recovery = None;
        self.ui_event_rx = None;
        self.action_tx = None;
        self.recovery_tx = None;
        self.game_result_rx = None;
        self.field_cursor = FieldCursor::new();
        self.overlay_state.clear();
    }

    fn open_deck_browser(&mut self) -> Result<()> {
        self.deck_list = DeckManager::list_decks(std::path::Path::new("desks")).unwrap_or_default();
        self.deck_selected = 0;
        self.mode = AppMode::DeckBrowser;
        Ok(())
    }

    fn open_deck_editor(&mut self) -> Result<()> {
        let Some(summary) = self.deck_list.get(self.deck_selected) else {
            return Ok(());
        };
        let deck = DeckManager::load(&summary.file_path)
            .map_err(|e| anyhow::anyhow!("加载卡组失败: {e}"))?;
        let path = summary.file_path.clone();

        let scripts_root = game_setup::find_scripts_root();
        let all_defs = if scripts_root.exists() {
            let index = ScriptIndex::scan(&scripts_root)
                .map_err(|e| anyhow::anyhow!("扫描脚本失败: {e}"))?;
            let loader = ScriptLoader::new(index);
            let registry = loader
                .load_all()
                .map_err(|e| anyhow::anyhow!("加载卡片失败: {e}"))?;
            let mut defs: Vec<_> = registry.iter().map(|(_, d)| d.clone()).collect();
            defs.sort_by(|a, b| a.id.0.cmp(&b.id.0));
            defs
        } else {
            Vec::new()
        };

        self.editing_deck = Some(deck);
        self.editing_deck_path = Some(path);
        self.all_card_defs = all_defs;
        self.editor_focus_right = false;
        self.editor_left_index = 0;
        self.editor_right_index = 0;
        self.mode = AppMode::DeckEditor;
        Ok(())
    }

    fn editor_add_card(&mut self) {
        let Some(def) = self.all_card_defs.get(self.editor_right_index) else {
            return;
        };
        let card_id = def.id.clone();
        if let Some(deck) = &mut self.editing_deck {
            let count = deck.cards.iter().filter(|id| *id == &card_id).count();
            if count < 3 {
                deck.cards.push(card_id);
            }
        }
    }

    fn editor_remove_card(&mut self) {
        let Some(deck) = &mut self.editing_deck else {
            return;
        };
        let unique_ids: Vec<CardId> = {
            let mut seen = std::collections::HashSet::new();
            let mut result = Vec::new();
            for id in &deck.cards {
                if seen.insert(id.0.clone()) {
                    result.push(id.clone());
                }
            }
            result.sort_by(|a, b| a.0.cmp(&b.0));
            result
        };
        if let Some(selected_id) = unique_ids.get(self.editor_left_index)
            && let Some(pos) = deck.cards.iter().rposition(|id| id == selected_id)
        {
            deck.cards.remove(pos);
        }
        let unique_count = unique_ids.len();
        if self.editor_left_index > 0 && self.editor_left_index >= unique_count {
            self.editor_left_index -= 1;
        }
    }

    fn editor_save_deck(&mut self) {
        let Some(path) = &self.editing_deck_path else {
            return;
        };
        let Some(deck) = &self.editing_deck else {
            return;
        };
        if let Err(e) = DeckManager::save(path, deck) {
            error!("save deck failed: {e}");
        }
    }

    fn enter_deck_selection(&mut self) -> Result<()> {
        self.deck_list = DeckManager::list_decks(std::path::Path::new("desks")).unwrap_or_default();
        self.deck_selected = 0;
        self.selection_phase = SelectionPhase::PickPlayerDeck;
        self.player_deck_path = None;
        self.ai_deck_path = None;
        self.mode = AppMode::DeckSelection;
        Ok(())
    }

    fn confirm_deck_selection(&mut self) -> Result<()> {
        let Some(summary) = self.deck_list.get(self.deck_selected) else {
            return Ok(());
        };
        let path = summary.file_path.clone();
        match self.selection_phase {
            SelectionPhase::PickPlayerDeck => {
                self.player_deck_path = Some(path);
                self.selection_phase = SelectionPhase::PickAiDeck;
                self.deck_selected = 0;
            }
            SelectionPhase::PickAiDeck => {
                self.ai_deck_path = Some(path);
                let player_path = self.player_deck_path.clone().unwrap();
                let ai_path = self.ai_deck_path.clone().unwrap();
                self.start_game_with_decks(player_path, ai_path)?;
            }
        }
        Ok(())
    }

    fn start_game_with_decks(
        &mut self,
        player_deck_path: PathBuf,
        ai_deck_path: PathBuf,
    ) -> Result<()> {
        let player_deck = DeckManager::load(&player_deck_path)
            .map_err(|e| anyhow::anyhow!("加载玩家卡组失败: {e}"))?;
        let ai_deck =
            DeckManager::load(&ai_deck_path).map_err(|e| anyhow::anyhow!("加载AI卡组失败: {e}"))?;

        self.mode = AppMode::LocalGame;
        self.game_title = format!("本地对战 ({} vs {})", player_deck.name, ai_deck.name);
        self.log_state = LogState::new();
        self.log_state.push(crate::ui::log::LogEntry {
            message: format!("正在加载卡组: {} / {}", player_deck.name, ai_deck.name),
            source: crate::ui::log::LogSource::System,
        });
        self.visible_state = None;
        self.pending_actions = None;
        self.pending_recovery = None;
        self.selected_action_index = 0;
        self.selected_recovery_index = 0;
        self.selected_recovery_cards.clear();
        self.field_cursor = FieldCursor::new();
        self.overlay_state.clear();

        let (action_tx, action_rx) = mpsc::channel::<PhaseAction>();
        let (recovery_tx, recovery_rx) = mpsc::channel::<Vec<InstanceId>>();
        let (ui_event_tx, ui_event_rx) = mpsc::channel::<UiEvent>();
        let (result_tx, result_rx) = mpsc::channel::<GameResult>();

        self.action_tx = Some(action_tx);
        self.recovery_tx = Some(recovery_tx);
        self.ui_event_rx = Some(ui_event_rx);
        self.game_result_rx = Some(result_rx);

        let tui_client = TuiClient::new(action_rx, recovery_rx, ui_event_tx.clone());

        thread::spawn(move || {
            info!("local game thread started with real decks");
            let ai_client = AiClient::new(20260322);

            let scripts_root = game_setup::find_scripts_root();
            match game_setup::build_game_from_decks(player_deck.cards, ai_deck.cards, scripts_root)
            {
                Ok((state, registry)) => {
                    let result = game_setup::run_local_game(
                        state,
                        registry,
                        Box::new(tui_client),
                        Box::new(ai_client),
                    );
                    match result {
                        Ok(game_result) => {
                            let visible =
                                game_state_to_visible(&game_result.final_state, PlayerId::Player1);
                            let _ = ui_event_tx.send(UiEvent::StateUpdate(visible));
                            let _ = ui_event_tx.send(UiEvent::Log("游戏线程已结束".to_string()));
                            let _ = result_tx.send(game_result);
                        }
                        Err(err) => {
                            error!(?err, "local game failed");
                            let _ = ui_event_tx.send(UiEvent::Log(format!("对战启动失败: {err}")));
                        }
                    }
                }
                Err(err) => {
                    let _ = ui_event_tx.send(UiEvent::Log(format!("加载脚本失败: {err}")));
                }
            }
        });

        self.mode = AppMode::InGame;
        Ok(())
    }

    #[allow(dead_code)]
    fn start_local_game(&mut self) -> Result<()> {
        self.mode = AppMode::LocalGame;
        self.game_title = "本地对战进行中 (Player1: TUI, Player2: AI)".to_string();
        self.log_state = LogState::new();
        self.log_state.push(crate::ui::log::LogEntry {
            message: "准备启动本地对战...".to_string(),
            source: crate::ui::log::LogSource::System,
        });
        self.visible_state = None;

        let rules = GameRules::default();
        let (state, registry) = game_setup::build_demo_state(rules);
        self.visible_state = Some(game_state_to_visible(&state, PlayerId::Player1));
        self.pending_actions = None;
        self.pending_recovery = None;
        self.selected_action_index = 0;
        self.selected_recovery_index = 0;
        self.selected_recovery_cards.clear();
        self.field_cursor = FieldCursor::new();
        self.overlay_state.clear();

        let (action_tx, action_rx) = mpsc::channel::<PhaseAction>();
        let (recovery_tx, recovery_rx) = mpsc::channel::<Vec<InstanceId>>();
        let (ui_event_tx, ui_event_rx) = mpsc::channel::<UiEvent>();
        let (result_tx, result_rx) = mpsc::channel::<GameResult>();

        self.action_tx = Some(action_tx.clone());
        self.recovery_tx = Some(recovery_tx.clone());
        self.ui_event_rx = Some(ui_event_rx);
        self.game_result_rx = Some(result_rx);

        let tui_client = TuiClient::new(action_rx, recovery_rx, ui_event_tx.clone());

        thread::spawn(move || {
            info!("local game thread started");
            let ai_client = AiClient::new(20260322);

            let result = game_setup::run_local_game(
                state,
                registry,
                Box::new(tui_client),
                Box::new(ai_client),
            );
            match result {
                Ok(game_result) => {
                    let visible =
                        game_state_to_visible(&game_result.final_state, PlayerId::Player1);
                    let _ = ui_event_tx.send(UiEvent::StateUpdate(visible));
                    let _ = ui_event_tx.send(UiEvent::Log("游戏线程已结束".to_string()));
                    let _ = result_tx.send(game_result);
                }
                Err(err) => {
                    error!(?err, "local game failed");
                    let _ = ui_event_tx.send(UiEvent::Log(format!("本地对战启动失败: {err}")));
                }
            }
        });

        self.mode = AppMode::InGame;
        Ok(())
    }

    fn start_online_game(&mut self) -> Result<()> {
        self.mode = AppMode::OnlineGame;
        self.game_title = "联网对战进行中".to_string();
        self.log_state = LogState::new();
        self.log_state.push(crate::ui::log::LogEntry {
            message: "连接匹配服务器...".to_string(),
            source: crate::ui::log::LogSource::System,
        });
        self.visible_state = None;
        self.pending_actions = None;
        self.pending_recovery = None;
        self.selected_action_index = 0;
        self.selected_recovery_index = 0;
        self.selected_recovery_cards.clear();
        self.field_cursor = FieldCursor::new();
        self.overlay_state.clear();

        let (action_tx, action_rx) = mpsc::channel::<PhaseAction>();
        let (recovery_tx, recovery_rx) = mpsc::channel::<Vec<InstanceId>>();
        let (ui_event_tx, ui_event_rx) = mpsc::channel::<UiEvent>();

        self.action_tx = Some(action_tx);
        self.recovery_tx = Some(recovery_tx);
        self.ui_event_rx = Some(ui_event_rx);
        self.game_result_rx = None;

        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            match runtime {
                Ok(rt) => {
                    if let Err(err) = rt.block_on(network::run_online_game_session(
                        action_rx,
                        recovery_rx,
                        ui_event_tx.clone(),
                    )) {
                        let _ = ui_event_tx.send(UiEvent::Log(format!("联网对战失败: {err}")));
                    }
                }
                Err(err) => {
                    let _ = ui_event_tx.send(UiEvent::Log(format!("无法创建异步运行时: {err}")));
                }
            }
        });

        Ok(())
    }

    fn handle_action_key(&mut self, key: KeyEvent) {
        let Some(actions) = self.pending_actions.as_ref() else {
            return;
        };

        match input::map_key(InputMode::ActionSelect, key) {
            InputAction::MoveUp => {
                input::move_cursor_up(&mut self.selected_action_index, actions.len());
            }
            InputAction::MoveDown => {
                input::move_cursor_down(&mut self.selected_action_index, actions.len());
            }
            InputAction::Confirm => {
                if let Some(action) = actions.get(self.selected_action_index) {
                    let action = self.normalize_action(action.clone());
                    self.submit_action(action);
                }
            }
            InputAction::Escape => {
                if let Some(pass) = actions
                    .iter()
                    .find(|action| matches!(action, PhaseAction::Pass))
                {
                    self.submit_action(pass.clone());
                }
            }
            InputAction::MinimizeOverlay => {
                self.overlay_state.toggle_minimized();
            }
            _ => {}
        }
    }

    fn handle_recovery_key(&mut self, key: KeyEvent) {
        let Some((count, options)) = self.pending_recovery.as_ref() else {
            return;
        };

        match input::map_key(InputMode::RecoverySelect, key) {
            InputAction::MoveUp => {
                input::move_cursor_up(&mut self.selected_recovery_index, options.len());
            }
            InputAction::MoveDown => {
                input::move_cursor_down(&mut self.selected_recovery_index, options.len());
            }
            InputAction::Toggle => {
                if let Some(instance_id) = options.get(self.selected_recovery_index) {
                    if self.selected_recovery_cards.contains(instance_id) {
                        self.selected_recovery_cards.remove(instance_id);
                    } else if self.selected_recovery_cards.len() < *count {
                        self.selected_recovery_cards.insert(*instance_id);
                    }
                }
            }
            InputAction::Confirm => {
                let selected = options
                    .iter()
                    .filter(|id| self.selected_recovery_cards.contains(id))
                    .take(*count)
                    .copied()
                    .collect::<Vec<_>>();
                self.submit_recovery(selected);
            }
            InputAction::Escape => {
                self.submit_recovery(Vec::new());
            }
            InputAction::MinimizeOverlay => {
                self.overlay_state.toggle_minimized();
            }
            _ => {}
        }
    }

    fn submit_action(&mut self, action: PhaseAction) {
        if let Some(action_tx) = &self.action_tx
            && action_tx.send(action.clone()).is_ok()
        {
            self.log_state.push(crate::ui::log::LogEntry {
                message: format!("已确认操作: {}", self.describe_action(&action)),
                source: crate::ui::log::LogSource::Me,
            });
        }
        self.pending_actions = None;
        self.selected_action_index = 0;
        self.overlay_state.clear();
    }

    fn submit_recovery(&mut self, selected: Vec<InstanceId>) {
        if let Some(recovery_tx) = &self.recovery_tx
            && recovery_tx.send(selected.clone()).is_ok()
        {
            self.log_state.push(crate::ui::log::LogEntry {
                message: format!("已确认回收张数: {}", selected.len()),
                source: crate::ui::log::LogSource::Me,
            });
        }
        self.pending_recovery = None;
        self.selected_recovery_cards.clear();
        self.selected_recovery_index = 0;
        self.overlay_state.clear();
    }

    fn is_field_cursor_mode_available(&self) -> bool {
        matches!(
            self.mode,
            AppMode::LocalGame | AppMode::OnlineGame | AppMode::InGame
        )
    }

    fn handle_field_cursor_key(&mut self, key: KeyEvent) -> bool {
        let action = input::map_key(InputMode::FieldBrowse, key);

        match action {
            InputAction::ToggleFieldBrowse => {
                if self.field_cursor.active {
                    self.field_cursor.deactivate();
                } else {
                    self.field_cursor.activate();
                }
                true
            }
            _ if !self.field_cursor.active => false,
            InputAction::Escape => {
                self.field_cursor.deactivate();
                true
            }
            InputAction::MoveLeft => {
                self.field_cursor.move_left();
                true
            }
            InputAction::MoveRight => {
                self.field_cursor.move_right();
                true
            }
            InputAction::MoveUp => {
                self.field_cursor.move_up();
                true
            }
            InputAction::MoveDown => {
                self.field_cursor.move_down();
                true
            }
            InputAction::QuickJump(zone) => {
                self.field_cursor.jump_to(zone);
                true
            }
            InputAction::MinimizeOverlay => {
                self.overlay_state.toggle_minimized();
                true
            }
            InputAction::FocusLog => true,
            _ => false,
        }
    }

    fn normalize_action(&self, action: PhaseAction) -> PhaseAction {
        match action {
            PhaseAction::PlayCard {
                instance_id,
                target_zone: _,
                cost_payment: _,
            } => PhaseAction::PlayCard {
                instance_id,
                target_zone: self.first_empty_front_zone().unwrap_or(Zone::Front(0)),
                cost_payment: Vec::new(),
            },
            other => other,
        }
    }

    fn first_empty_front_zone(&self) -> Option<Zone> {
        self.visible_state.as_ref().and_then(|state| {
            state
                .my_state
                .zones
                .front
                .iter()
                .position(Option::is_none)
                .map(Zone::Front)
        })
    }

    fn describe_action(&self, action: &PhaseAction) -> String {
        match action {
            PhaseAction::PlayCard { instance_id, .. } => {
                format!("出牌 ({})", self.describe_instance(*instance_id))
            }
            PhaseAction::DeclareAttack {
                attacker,
                target_slot,
            } => {
                let target = target_slot
                    .map(|slot| format!("前场槽位{}", slot + 1))
                    .unwrap_or_else(|| "直接攻击".to_string());
                format!("攻击 ({} -> {target})", self.describe_instance(*attacker))
            }
            PhaseAction::Pass => "跳过".to_string(),
            PhaseAction::Surrender => "投降".to_string(),
        }
    }

    fn describe_instance(&self, instance_id: InstanceId) -> String {
        if let Some(state) = &self.visible_state {
            for card in &state.my_state.zones.hand {
                if card.instance_id == instance_id {
                    return card.definition_id.0.clone();
                }
            }
            for card in state.my_state.zones.front.iter().flatten() {
                if card.instance_id == instance_id {
                    return card.definition_id.0.clone();
                }
            }
            for card in state.my_state.zones.back.iter().flatten() {
                if card.instance_id == instance_id {
                    return card.definition_id.0.clone();
                }
            }
            for card in &state.my_state.zones.cost_zone {
                if card.instance_id == instance_id {
                    return card.definition_id.0.clone();
                }
            }
        }
        format!("实例#{:?}", instance_id)
    }

    fn drain_game_events(&mut self) {
        if let Some(rx) = &self.ui_event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    UiEvent::ActionPrompt(actions) => {
                        self.pending_actions = Some(actions.clone());
                        self.pending_recovery = None;
                        self.selected_action_index = 0;
                        self.selected_recovery_cards.clear();
                        self.overlay_state = OverlayState::for_action();
                        self.log_state.push(crate::ui::log::LogEntry {
                            message: format!("等待行动选择，候选数: {}", actions.len()),
                            source: crate::ui::log::LogSource::System,
                        });
                    }
                    UiEvent::RecoveryPrompt { count, options } => {
                        self.pending_recovery = Some((count, options.clone()));
                        self.pending_actions = None;
                        self.selected_recovery_index = 0;
                        self.selected_recovery_cards.clear();
                        self.overlay_state = OverlayState::for_recovery();
                        self.log_state.push(crate::ui::log::LogEntry {
                            message: format!(
                                "等待恢复选择，需选择 {} 张（可选 {}）",
                                count,
                                options.len()
                            ),
                            source: crate::ui::log::LogSource::System,
                        });
                    }
                    UiEvent::StateUpdate(state) => {
                        self.visible_state = Some(state);
                        self.log_state.push(crate::ui::log::LogEntry {
                            message: "板面状态已刷新".to_string(),
                            source: crate::ui::log::LogSource::System,
                        });
                    }
                    UiEvent::Log(message) => {
                        self.log_state.push(crate::ui::log::LogEntry {
                            message,
                            source: crate::ui::log::LogSource::System,
                        });
                    }
                    UiEvent::EnterOnlineBattle { opponent_name } => {
                        self.mode = AppMode::InGame;
                        self.game_title = format!("联网对战进行中 (你 vs {opponent_name})");
                        self.log_state.push(crate::ui::log::LogEntry {
                            message: format!("匹配成功，对手: {opponent_name}"),
                            source: crate::ui::log::LogSource::System,
                        });
                    }
                }
            }
        }
    }

    fn drain_game_result(&mut self) {
        if let Some(rx) = &self.game_result_rx
            && let Ok(result) = rx.try_recv()
        {
            self.visible_state = Some(game_state_to_visible(
                &result.final_state,
                PlayerId::Player1,
            ));
            self.log_state.push(crate::ui::log::LogEntry {
                message: format!("胜者: {:?}", result.winner),
                source: crate::ui::log::LogSource::System,
            });
            self.log_state.push(crate::ui::log::LogEntry {
                message: format!("结束原因: {:?}", result.reason),
                source: crate::ui::log::LogSource::System,
            });
            self.log_state.push(crate::ui::log::LogEntry {
                message: format!("回合数: {}", result.final_state.turn_number),
                source: crate::ui::log::LogSource::System,
            });
            self.log_state.push(crate::ui::log::LogEntry {
                message: format!("事件总数: {}", result.event_log.len()),
                source: crate::ui::log::LogSource::System,
            });
            self.mode = AppMode::GameOver(result.winner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::FieldZone;
    use crate::ui::overlay::OverlayType;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    #[test]
    fn tab_toggles_browse_mode_and_esc_exits() {
        let mut app = App::new();
        app.mode = AppMode::InGame;

        app.handle_key(key(KeyCode::Tab)).expect("tab enter");
        assert!(app.field_cursor.active);

        app.handle_key(key(KeyCode::Tab)).expect("tab exit");
        assert!(!app.field_cursor.active);

        app.handle_key(key(KeyCode::Tab)).expect("tab re-enter");
        assert!(app.field_cursor.active);
        app.handle_key(key(KeyCode::Esc)).expect("esc exit");
        assert!(!app.field_cursor.active);
    }

    #[test]
    fn arrows_move_cursor_only_in_browse_mode() {
        let mut app = App::new();
        app.mode = AppMode::InGame;

        app.handle_key(key(KeyCode::Right)).expect("right noop");
        assert_eq!(*app.field_cursor.current_zone(), FieldZone::MyFront(0));

        app.handle_key(key(KeyCode::Tab)).expect("activate");
        app.handle_key(key(KeyCode::Right)).expect("move right");
        assert_eq!(*app.field_cursor.current_zone(), FieldZone::MyFront(1));

        app.handle_key(key(KeyCode::Up)).expect("move up");
        assert_eq!(
            *app.field_cursor.current_zone(),
            FieldZone::OpponentFront(1)
        );
    }

    #[test]
    fn quick_keys_jump_to_expected_zones() {
        let mut app = App::new();
        app.mode = AppMode::InGame;
        app.handle_key(key(KeyCode::Tab)).expect("activate");

        app.handle_key(key(KeyCode::Char('3'))).expect("my front 3");
        assert_eq!(*app.field_cursor.current_zone(), FieldZone::MyFront(2));

        app.handle_key(key(KeyCode::Char('%'))).expect("my back 5");
        assert_eq!(*app.field_cursor.current_zone(), FieldZone::MyBack(4));

        app.handle_key(key(KeyCode::Char('h'))).expect("my hand");
        assert_eq!(*app.field_cursor.current_zone(), FieldZone::MyHand(0));

        app.handle_key(key(KeyCode::Char('c'))).expect("my cost");
        assert_eq!(*app.field_cursor.current_zone(), FieldZone::MyCostZone(0));

        app.handle_key(key(KeyCode::Char('f')))
            .expect("opponent front");
        assert_eq!(
            *app.field_cursor.current_zone(),
            FieldZone::OpponentFront(0)
        );

        app.handle_key(key(KeyCode::Char('b')))
            .expect("opponent back");
        assert_eq!(*app.field_cursor.current_zone(), FieldZone::OpponentBack(0));
    }

    #[test]
    fn pending_action_overlay_keeps_popup_first_key_path() {
        let mut app = App::new();
        app.mode = AppMode::InGame;
        app.pending_actions = Some(vec![PhaseAction::Pass]);

        app.handle_key(key(KeyCode::Tab))
            .expect("tab while overlay");

        assert!(!app.field_cursor.active);
        assert!(app.pending_actions.is_some());
    }

    #[test]
    fn overlay_minimize_toggle_with_m_key() {
        let mut app = App::new();
        app.mode = AppMode::InGame;
        app.pending_actions = Some(vec![PhaseAction::Pass]);
        app.overlay_state.visible = true;
        app.overlay_state.minimized = false;
        app.overlay_state.overlay_type = OverlayType::ActionSelect;

        app.handle_key(key(KeyCode::Char('m')))
            .expect("minimize overlay");
        assert!(app.overlay_state.minimized);

        app.handle_key(key(KeyCode::Char('m')))
            .expect("restore overlay");
        assert!(!app.overlay_state.minimized);
    }

    #[test]
    fn minimized_overlay_allows_field_cursor_browsing() {
        let mut app = App::new();
        app.mode = AppMode::InGame;
        app.pending_actions = Some(vec![PhaseAction::Pass]);
        app.overlay_state.visible = true;
        app.overlay_state.minimized = true;
        app.overlay_state.overlay_type = OverlayType::ActionSelect;

        app.handle_key(key(KeyCode::Tab))
            .expect("tab enters field browse when minimized");

        assert!(app.field_cursor.active);
        assert!(app.pending_actions.is_some());
    }

    #[test]
    fn non_minimized_overlay_still_blocks_field_cursor_tab() {
        let mut app = App::new();
        app.mode = AppMode::InGame;
        app.pending_actions = Some(vec![PhaseAction::Pass]);
        app.overlay_state.visible = true;
        app.overlay_state.minimized = false;
        app.overlay_state.overlay_type = OverlayType::ActionSelect;

        app.handle_key(key(KeyCode::Tab))
            .expect("tab while overlay not minimized");

        assert!(!app.field_cursor.active);
        assert!(app.pending_actions.is_some());
    }
}
