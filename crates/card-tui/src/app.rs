use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use std::path::PathBuf;

use anyhow::Result;
use card_client::api::{game_state_to_visible, VisibleGameState};
use card_core::deck::{Deck, DeckManager, DeckSummary};
use card_core::engine::phase::{PhaseAction, PhaseClient, RecoveryCardOption};
use card_core::engine::{GameEngine, GameResult};
use card_core::rules::GameRules;
use card_core::state::{CardInstance, CardRegistryImpl, GameState};
use card_core::types::{
    CardDefinition, CardId, CardType, Category, InstanceId, PlayerId, Property, Zone,
};
use card_protocol::codec::TcpConnection;
use card_protocol::message::{
    AttackTarget, AvailableAction, Command, CostPayment, GameEvent, NetworkMessage,
};
use card_script::loader::{ScriptIndex, ScriptLoader};
use card_server::AiClient;
use card_server::GameServer;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::{Frame, Terminal};
use tracing::{error, info};

use crate::input::{self, ActionInput, RecoveryInput};
use crate::matchmaker_client::{find_match, MatchResult};
use crate::ui;

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
    game_events: Vec<String>,
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
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: AppMode::MainMenu,
            selected_index: 0,
            game_events: Vec::new(),
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
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        loop {
            self.drain_game_events();
            self.drain_game_result();

            terminal.draw(|frame| self.render(frame))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key)?;
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame<'_>) {
        match self.mode {
            AppMode::MainMenu => self.render_main_menu(frame),
            AppMode::DeckBrowser => self.render_deck_browser(frame),
            AppMode::DeckSelection => self.render_deck_selection(frame),
            AppMode::DeckEditor => self.render_deck_browser(frame),
            AppMode::LocalGame | AppMode::OnlineGame | AppMode::InGame | AppMode::GameOver(_) => {
                self.render_game_screen(frame)
            }
        }
    }

    fn render_main_menu(&self, frame: &mut Frame<'_>) {
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Length(8),
                Constraint::Percentage(62),
            ])
            .split(frame.area());

        let panel = Block::default().borders(Borders::ALL).title("主菜单");
        frame.render_widget(panel, root[1]);

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .margin(1)
            .split(root[1]);

        let title = Paragraph::new("卡牌对战游戏")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(title, inner[0]);

        let menu = ["本地对战 (vs AI)", "联网对战", "卡组管理", "退出"];

        for (idx, item) in menu.iter().enumerate() {
            let prefix = if idx == self.selected_index {
                "> "
            } else {
                "  "
            };
            let style = if idx == self.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let line = Paragraph::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(*item, style),
            ]))
            .alignment(Alignment::Center);
            frame.render_widget(line, inner[idx + 1]);
        }
    }

    fn render_game_screen(&self, frame: &mut Frame<'_>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(2),
            ])
            .split(frame.area());

        let title = match self.mode {
            AppMode::LocalGame => "正在创建本地对战...",
            AppMode::OnlineGame => "联网对战：正在连接与匹配...",
            AppMode::InGame => &self.game_title,
            AppMode::GameOver(Some(winner)) => {
                if winner == PlayerId::Player1 {
                    "对局结束：你获胜"
                } else {
                    "对局结束：AI 获胜"
                }
            }
            AppMode::GameOver(None) => "对局结束：平局",
            _ => "",
        };

        frame.render_widget(
            Paragraph::new(title).alignment(Alignment::Center).style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            chunks[0],
        );

        if let Some(state) = &self.visible_state {
            ui::render_game_board(frame, chunks[1], state, &self.game_events);
        } else {
            ui::render_waiting_screen(frame, chunks[1]);
        }

        let hint = if matches!(self.mode, AppMode::GameOver(_)) {
            "按 Enter / Esc / q 返回主菜单"
        } else {
            "按 q 退出"
        };
        frame.render_widget(
            Paragraph::new(hint)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            chunks[2],
        );

        if let Some(actions) = self.pending_actions.as_deref() {
            self.render_action_overlay(frame, actions);
        }

        if let Some((count, options)) = self.pending_recovery.as_ref() {
            self.render_recovery_overlay(frame, *count, options);
        }
    }

    fn render_action_overlay(&self, frame: &mut Frame<'_>, actions: &[PhaseAction]) {
        let popup = centered_rect(70, 40, frame.area());
        frame.render_widget(Clear, popup);

        let mut lines = Vec::new();
        for (idx, action) in actions.iter().enumerate() {
            let prefix = if idx == self.selected_action_index {
                "> "
            } else {
                "  "
            };
            lines.push(Line::from(format!(
                "{prefix}{}",
                self.describe_action(action)
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from("↑/↓ 选择  Enter 确认  Esc 跳过"));

        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("选择操作")),
            popup,
        );
    }

    fn render_recovery_overlay(&self, frame: &mut Frame<'_>, count: usize, options: &[InstanceId]) {
        let popup = centered_rect(70, 45, frame.area());
        frame.render_widget(Clear, popup);

        let mut lines = Vec::new();
        lines.push(Line::from(format!("请最多选择 {count} 张回收卡")));
        lines.push(Line::from(""));

        for (idx, instance_id) in options.iter().enumerate() {
            let cursor = if idx == self.selected_recovery_index {
                ">"
            } else {
                " "
            };
            let selected = if self.selected_recovery_cards.contains(instance_id) {
                "x"
            } else {
                " "
            };
            lines.push(Line::from(format!(
                "{cursor} [{selected}] {}",
                self.describe_instance(*instance_id)
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from("↑/↓ 移动  Space 勾选  Enter 确认  Esc 取消"));

        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("回收选择")),
            popup,
        );
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

        if self.pending_actions.is_some() {
            self.handle_action_key(key);
            return Ok(());
        }

        if self.pending_recovery.is_some() {
            self.handle_recovery_key(key);
            return Ok(());
        }

        match self.mode {
            AppMode::MainMenu => {
                match key.code {
                    KeyCode::Up => {
                        self.selected_index = self.selected_index.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        self.selected_index = (self.selected_index + 1).min(3);
                    }
                    KeyCode::Enter => {
                        match self.selected_index {
                            0 => self.enter_deck_selection()?,
                            1 => self.start_online_game()?,
                            2 => self.open_deck_browser()?,
                            _ => self.should_quit = true,
                        }
                    }
                    _ => {}
                }
            }
            AppMode::DeckBrowser | AppMode::DeckEditor => {
                match key.code {
                    KeyCode::Up => {
                        self.deck_selected = self.deck_selected.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        let max = self.deck_list.len().saturating_sub(1);
                        self.deck_selected = (self.deck_selected + 1).min(max);
                    }
                    KeyCode::Esc => {
                        self.reset_to_main_menu();
                    }
                    _ => {}
                }
            }
            AppMode::DeckSelection => {
                match key.code {
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
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn reset_to_main_menu(&mut self) {
        self.mode = AppMode::MainMenu;
        self.selected_index = 0;
        self.game_events.clear();
        self.visible_state = None;
        self.pending_actions = None;
        self.pending_recovery = None;
        self.ui_event_rx = None;
        self.action_tx = None;
        self.recovery_tx = None;
        self.game_result_rx = None;
    }

    fn open_deck_browser(&mut self) -> Result<()> {
        self.deck_list = DeckManager::list_decks(std::path::Path::new("desks"))
            .unwrap_or_default();
        self.deck_selected = 0;
        self.mode = AppMode::DeckBrowser;
        Ok(())
    }

    fn enter_deck_selection(&mut self) -> Result<()> {
        self.deck_list = DeckManager::list_decks(std::path::Path::new("desks"))
            .unwrap_or_default();
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
        let ai_deck = DeckManager::load(&ai_deck_path)
            .map_err(|e| anyhow::anyhow!("加载AI卡组失败: {e}"))?;

        self.mode = AppMode::LocalGame;
        self.game_title = format!(
            "本地对战 ({} vs {})",
            player_deck.name, ai_deck.name
        );
        self.game_events.clear();
        self.game_events.push(format!(
            "正在加载卡组: {} / {}",
            player_deck.name, ai_deck.name
        ));
        self.visible_state = None;
        self.pending_actions = None;
        self.pending_recovery = None;
        self.selected_action_index = 0;
        self.selected_recovery_index = 0;
        self.selected_recovery_cards.clear();

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

            let scripts_root = find_scripts_root();
            match build_game_from_decks(
                player_deck.cards,
                ai_deck.cards,
                scripts_root,
            ) {
                Ok((state, registry)) => {
                    let result = run_local_game(
                        state,
                        registry,
                        Box::new(tui_client),
                        Box::new(ai_client),
                    );
                    match result {
                        Ok(game_result) => {
                            let visible = game_state_to_visible(
                                &game_result.final_state,
                                PlayerId::Player1,
                            );
                            let _ = ui_event_tx.send(UiEvent::StateUpdate(visible));
                            let _ = ui_event_tx
                                .send(UiEvent::Log("游戏线程已结束".to_string()));
                            let _ = result_tx.send(game_result);
                        }
                        Err(err) => {
                            error!(?err, "local game failed");
                            let _ = ui_event_tx
                                .send(UiEvent::Log(format!("对战启动失败: {err}")));
                        }
                    }
                }
                Err(err) => {
                    let _ = ui_event_tx
                        .send(UiEvent::Log(format!("加载脚本失败: {err}")));
                }
            }
        });

        self.mode = AppMode::InGame;
        Ok(())
    }

    fn render_deck_browser(&self, frame: &mut Frame<'_>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(2),
            ])
            .split(frame.area());

        frame.render_widget(
            Paragraph::new("卡组管理")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            chunks[0],
        );

        let mut lines: Vec<Line> = Vec::new();
        if self.deck_list.is_empty() {
            lines.push(Line::from("  (desks/ 目录下暂无卡组)"));
        } else {
            for (idx, summary) in self.deck_list.iter().enumerate() {
                let prefix = if idx == self.deck_selected { "> " } else { "  " };
                let style = if idx == self.deck_selected {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::from(Span::styled(
                    format!("{prefix}{} ({} 张)", summary.name, summary.card_count),
                    style,
                )));
            }
        }

        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("卡组列表")),
            chunks[1],
        );

        frame.render_widget(
            Paragraph::new("↑/↓ 选择 | Esc 返回主菜单")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            chunks[2],
        );
    }

    fn render_deck_selection(&self, frame: &mut Frame<'_>) {
        let title = match self.selection_phase {
            SelectionPhase::PickPlayerDeck => "选择己方卡组",
            SelectionPhase::PickAiDeck => "选择 AI 卡组",
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(2),
            ])
            .split(frame.area());

        frame.render_widget(
            Paragraph::new(title)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            chunks[0],
        );

        let mut lines: Vec<Line> = Vec::new();
        if let Some(p) = &self.player_deck_path {
            let name = self
                .deck_list
                .iter()
                .find(|s| &s.file_path == p)
                .map(|s| s.name.as_str())
                .unwrap_or("?");
            lines.push(Line::from(Span::styled(
                format!("  己方卡组: {name}  ✓"),
                Style::default().fg(Color::Green),
            )));
            lines.push(Line::from(""));
        }

        if self.deck_list.is_empty() {
            lines.push(Line::from("  (desks/ 目录下暂无卡组，请先创建)"));
        } else {
            for (idx, summary) in self.deck_list.iter().enumerate() {
                let prefix = if idx == self.deck_selected { "> " } else { "  " };
                let style = if idx == self.deck_selected {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::from(Span::styled(
                    format!("{prefix}{} ({} 张)", summary.name, summary.card_count),
                    style,
                )));
            }
        }

        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(title)),
            chunks[1],
        );

        frame.render_widget(
            Paragraph::new("↑/↓ 选择 | Enter 确认 | Esc 返回主菜单")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            chunks[2],
        );
    }

    fn handle_action_key(&mut self, key: KeyEvent) {
        let Some(actions) = self.pending_actions.as_ref() else {
            return;
        };

        match input::map_action_key(key) {
            ActionInput::MoveUp => {
                input::move_cursor_up(&mut self.selected_action_index, actions.len());
            }
            ActionInput::MoveDown => {
                input::move_cursor_down(&mut self.selected_action_index, actions.len());
            }
            ActionInput::Confirm => {
                if let Some(action) = actions.get(self.selected_action_index) {
                    let action = self.normalize_action(action.clone());
                    self.submit_action(action);
                }
            }
            ActionInput::Escape => {
                if let Some(pass) = actions
                    .iter()
                    .find(|action| matches!(action, PhaseAction::Pass))
                {
                    self.submit_action(pass.clone());
                }
            }
            ActionInput::Noop => {}
        }
    }

    fn handle_recovery_key(&mut self, key: KeyEvent) {
        let Some((count, options)) = self.pending_recovery.as_ref() else {
            return;
        };

        match input::map_recovery_key(key) {
            RecoveryInput::MoveUp => {
                input::move_cursor_up(&mut self.selected_recovery_index, options.len());
            }
            RecoveryInput::MoveDown => {
                input::move_cursor_down(&mut self.selected_recovery_index, options.len());
            }
            RecoveryInput::Toggle => {
                if let Some(instance_id) = options.get(self.selected_recovery_index) {
                    if self.selected_recovery_cards.contains(instance_id) {
                        self.selected_recovery_cards.remove(instance_id);
                    } else if self.selected_recovery_cards.len() < *count {
                        self.selected_recovery_cards.insert(*instance_id);
                    }
                }
            }
            RecoveryInput::Confirm => {
                let selected = options
                    .iter()
                    .filter(|id| self.selected_recovery_cards.contains(id))
                    .take(*count)
                    .copied()
                    .collect::<Vec<_>>();
                self.submit_recovery(selected);
            }
            RecoveryInput::Escape => {
                self.submit_recovery(Vec::new());
            }
            RecoveryInput::Noop => {}
        }
    }

    fn submit_action(&mut self, action: PhaseAction) {
        if let Some(action_tx) = &self.action_tx {
            if action_tx.send(action.clone()).is_ok() {
                self.game_events
                    .push(format!("已确认操作: {}", self.describe_action(&action)));
            }
        }
        self.pending_actions = None;
        self.selected_action_index = 0;
    }

    fn submit_recovery(&mut self, selected: Vec<InstanceId>) {
        if let Some(recovery_tx) = &self.recovery_tx {
            if recovery_tx.send(selected.clone()).is_ok() {
                self.game_events
                    .push(format!("已确认回收张数: {}", selected.len()));
            }
        }
        self.pending_recovery = None;
        self.selected_recovery_cards.clear();
        self.selected_recovery_index = 0;
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

    fn start_local_game(&mut self) -> Result<()> {
        self.mode = AppMode::LocalGame;
        self.game_title = "本地对战进行中 (Player1: TUI, Player2: AI)".to_string();
        self.game_events.clear();
        self.game_events.push("准备启动本地对战...".to_string());
        self.visible_state = None;

        let rules = GameRules::default();
        let (state, registry) = build_demo_state(rules);
        self.visible_state = Some(game_state_to_visible(&state, PlayerId::Player1));
        self.pending_actions = None;
        self.pending_recovery = None;
        self.selected_action_index = 0;
        self.selected_recovery_index = 0;
        self.selected_recovery_cards.clear();

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

            let result = run_local_game(state, registry, Box::new(tui_client), Box::new(ai_client));
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
        self.game_events.clear();
        self.game_events.push("连接匹配服务器...".to_string());
        self.visible_state = None;
        self.pending_actions = None;
        self.pending_recovery = None;
        self.selected_action_index = 0;
        self.selected_recovery_index = 0;
        self.selected_recovery_cards.clear();

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
                    if let Err(err) = rt.block_on(run_online_game_session(
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

    fn drain_game_events(&mut self) {
        if let Some(rx) = &self.ui_event_rx {
            while let Ok(event) = rx.try_recv() {
                let msg = match event {
                    UiEvent::ActionPrompt(actions) => {
                        self.pending_actions = Some(actions.clone());
                        self.pending_recovery = None;
                        self.selected_action_index = 0;
                        self.selected_recovery_cards.clear();
                        format!("等待行动选择，候选数: {}", actions.len())
                    }
                    UiEvent::RecoveryPrompt { count, options } => {
                        self.pending_recovery = Some((count, options.clone()));
                        self.pending_actions = None;
                        self.selected_recovery_index = 0;
                        self.selected_recovery_cards.clear();
                        format!(
                            "等待恢复选择，需选择 {} 张（可选 {}）",
                            count,
                            options.len()
                        )
                    }
                    UiEvent::StateUpdate(state) => {
                        self.visible_state = Some(state);
                        "板面状态已刷新".to_string()
                    }
                    UiEvent::Log(message) => message,
                    UiEvent::EnterOnlineBattle { opponent_name } => {
                        self.mode = AppMode::InGame;
                        self.game_title = format!("联网对战进行中 (你 vs {opponent_name})");
                        format!("匹配成功，对手: {opponent_name}")
                    }
                };
                self.game_events.push(msg);
            }
        }
    }

    fn drain_game_result(&mut self) {
        if let Some(rx) = &self.game_result_rx {
            if let Ok(result) = rx.try_recv() {
                self.visible_state = Some(game_state_to_visible(
                    &result.final_state,
                    PlayerId::Player1,
                ));
                self.game_events.push(format!("胜者: {:?}", result.winner));
                self.game_events
                    .push(format!("结束原因: {:?}", result.reason));
                self.game_events
                    .push(format!("回合数: {}", result.final_state.turn_number));
                self.game_events
                    .push(format!("事件总数: {}", result.event_log.len()));
                self.mode = AppMode::GameOver(result.winner);
            }
        }
    }
}

async fn run_online_game_session(
    action_rx: mpsc::Receiver<PhaseAction>,
    recovery_rx: mpsc::Receiver<Vec<InstanceId>>,
    ui_event_tx: mpsc::Sender<UiEvent>,
) -> Result<()> {
    let _ = ui_event_tx.send(UiEvent::Log("连接匹配服务器...".to_string()));
    let match_result = find_match("ws://127.0.0.1:9090", "Player", "0.1.0").await?;
    let _ = ui_event_tx.send(UiEvent::Log(format!(
        "匹配成功：对手={}，主机={}，我方是否主机={}",
        match_result.opponent_name, match_result.host_addr, match_result.is_host
    )));

    if match_result.is_host {
        run_host_flow(match_result, action_rx, recovery_rx, ui_event_tx).await
    } else {
        run_guest_flow(match_result, action_rx, recovery_rx, ui_event_tx).await
    }
}

async fn run_host_flow(
    match_result: MatchResult,
    action_rx: mpsc::Receiver<PhaseAction>,
    recovery_rx: mpsc::Receiver<Vec<InstanceId>>,
    ui_event_tx: mpsc::Sender<UiEvent>,
) -> Result<()> {
    let bind_addr = derive_host_bind_addr(&match_result.host_addr);
    let script_index = ScriptIndex::scan(Path::new("/tmp/card_tui_empty_scripts"))
        .map_err(|err| anyhow::anyhow!("扫描脚本目录失败: {err}"))?;

    let server = GameServer::new(
        bind_addr,
        script_index,
        GameRules::default(),
        "0.1.0".to_string(),
        20260322,
    );
    let server_tx = ui_event_tx.clone();
    tokio::spawn(async move {
        if let Err(err) = server.start().await {
            let _ = server_tx.send(UiEvent::Log(format!("游戏服务器错误: {err}")));
        }
    });

    let _ = ui_event_tx.send(UiEvent::Log(format!(
        "已启动本地游戏服务器，监听地址: {bind_addr}"
    )));

    tokio::time::sleep(Duration::from_millis(200)).await;
    run_network_client(
        &bind_addr.to_string(),
        &match_result,
        action_rx,
        recovery_rx,
        ui_event_tx,
    )
    .await
}

async fn run_guest_flow(
    match_result: MatchResult,
    action_rx: mpsc::Receiver<PhaseAction>,
    recovery_rx: mpsc::Receiver<Vec<InstanceId>>,
    ui_event_tx: mpsc::Sender<UiEvent>,
) -> Result<()> {
    run_network_client(
        &match_result.host_addr,
        &match_result,
        action_rx,
        recovery_rx,
        ui_event_tx,
    )
    .await
}

fn derive_host_bind_addr(host_addr: &str) -> SocketAddr {
    if let Ok(addr) = SocketAddr::from_str(host_addr) {
        return SocketAddr::new(addr.ip(), 10000);
    }
    SocketAddr::from(([127, 0, 0, 1], 10000))
}

async fn run_network_client(
    server_addr: &str,
    match_result: &MatchResult,
    action_rx: mpsc::Receiver<PhaseAction>,
    recovery_rx: mpsc::Receiver<Vec<InstanceId>>,
    ui_event_tx: mpsc::Sender<UiEvent>,
) -> Result<()> {
    let stream = tokio::net::TcpStream::connect(server_addr)
        .await
        .map_err(|err| anyhow::anyhow!("无法连接游戏服务器 {server_addr}: {err}"))?;
    let mut conn = TcpConnection::from_stream(stream);

    let server_version = match conn.recv().await? {
        NetworkMessage::Hello {
            version,
            player_name: _,
        } => version,
        other => {
            anyhow::bail!("握手失败，收到无效消息: {other:?}")
        }
    };

    conn.send(&NetworkMessage::Hello {
        version: server_version,
        player_name: "Player".to_string(),
    })
    .await?;

    match conn.recv().await? {
        NetworkMessage::HelloAck { player_id } => {
            let _ = ui_event_tx.send(UiEvent::Log(format!("握手成功，分配身份: {player_id:?}")));
        }
        other => anyhow::bail!("握手确认失败: {other:?}"),
    }

    conn.send(&NetworkMessage::DeckSubmit {
        card_ids: build_demo_deck_ids(),
    })
    .await?;

    match conn.recv().await? {
        NetworkMessage::DeckAccepted => {
            let _ = ui_event_tx.send(UiEvent::Log("卡组校验通过".to_string()));
        }
        NetworkMessage::DeckRejected { reason } => {
            anyhow::bail!("卡组被拒绝: {reason}")
        }
        other => anyhow::bail!("等待卡组校验结果时收到无效消息: {other:?}"),
    }

    match conn.recv().await? {
        NetworkMessage::GameStart { .. } => {
            let _ = ui_event_tx.send(UiEvent::EnterOnlineBattle {
                opponent_name: match_result.opponent_name.clone(),
            });
        }
        other => anyhow::bail!("等待开局消息时收到无效消息: {other:?}"),
    }

    loop {
        match conn.recv().await? {
            NetworkMessage::EventNotification { event } => {
                if let GameEvent::GameOver { winner, reason } = event {
                    let _ = ui_event_tx.send(UiEvent::Log(format!(
                        "对局结束，胜者: {winner:?}，原因: {reason:?}"
                    )));
                    break;
                } else {
                    let _ = ui_event_tx.send(UiEvent::Log(format!("游戏事件: {event:?}")));
                }
            }
            NetworkMessage::RequestAction {
                available_actions, ..
            } => {
                let actions = available_actions_to_phase_actions(&available_actions);
                let _ = ui_event_tx.send(UiEvent::ActionPrompt(actions));
                let chosen = action_rx
                    .recv()
                    .map_err(|_| anyhow::anyhow!("无法获取操作输入"))?;
                let command = phase_action_to_command(chosen);
                conn.send(&NetworkMessage::CommandResponse { command }).await?;
            }
            NetworkMessage::RequestCardSelection {
                candidates, count, ..
            } => {
                let _ = ui_event_tx.send(UiEvent::RecoveryPrompt {
                    count,
                    options: candidates.clone(),
                });
                let selected = recovery_rx
                    .recv()
                    .map_err(|_| anyhow::anyhow!("无法获取选牌输入"))?;
                conn.send(&NetworkMessage::CardResponse {
                    instance_ids: selected,
                })
                .await?;
            }
            NetworkMessage::RequestTargetSelection { .. } => {
                conn.send(&NetworkMessage::TargetResponse { targets: Vec::new() })
                    .await?;
            }
            NetworkMessage::Ping => {
                conn.send(&NetworkMessage::Pong).await?;
            }
            NetworkMessage::Disconnect { reason } => {
                anyhow::bail!("服务器断开连接: {reason}")
            }
            NetworkMessage::Hello { .. }
            | NetworkMessage::HelloAck { .. }
            | NetworkMessage::DeckSubmit { .. }
            | NetworkMessage::DeckAccepted
            | NetworkMessage::DeckRejected { .. }
            | NetworkMessage::GameStart { .. }
            | NetworkMessage::CommandResponse { .. }
            | NetworkMessage::TargetResponse { .. }
            | NetworkMessage::CardResponse { .. }
            | NetworkMessage::Pong => {}
        }
    }

    Ok(())
}

fn available_actions_to_phase_actions(available: &[AvailableAction]) -> Vec<PhaseAction> {
    let mut actions = Vec::new();
    for action in available {
        match action {
            AvailableAction::PlayCard { instance_id } => actions.push(PhaseAction::PlayCard {
                instance_id: *instance_id,
                target_zone: Zone::Front(0),
                cost_payment: Vec::new(),
            }),
            AvailableAction::DeclareAttack {
                attacker,
                possible_targets,
            } => {
                let target_slot = possible_targets.iter().find_map(|target| match target {
                    AttackTarget::FrontSlot(slot) => Some(*slot),
                    AttackTarget::DirectAttack => None,
                });
                actions.push(PhaseAction::DeclareAttack {
                    attacker: *attacker,
                    target_slot,
                });
            }
            AvailableAction::ChainPass => actions.push(PhaseAction::Pass),
            AvailableAction::Surrender => actions.push(PhaseAction::Surrender),
            AvailableAction::ActivateEffect { .. }
            | AvailableAction::ChainActivate { .. }
            | AvailableAction::SelectRecoveryCards { .. } => {}
        }
    }

    if actions.is_empty() {
        actions.push(PhaseAction::Pass);
    }
    actions
}

fn phase_action_to_command(action: PhaseAction) -> Command {
    match action {
        PhaseAction::PlayCard {
            instance_id,
            target_zone,
            ..
        } => Command::PlayCard {
            instance_id,
            target_zone,
            cost_payment: CostPayment {
                hand_cards: Vec::new(),
                real_point: 0,
            },
        },
        PhaseAction::DeclareAttack {
            attacker,
            target_slot,
        } => Command::DeclareAttack {
            attacker,
            target: target_slot
                .map(AttackTarget::FrontSlot)
                .unwrap_or(AttackTarget::DirectAttack),
        },
        PhaseAction::Pass => Command::ChainPass,
        PhaseAction::Surrender => Command::Surrender,
    }
}

fn build_demo_deck_ids() -> Vec<CardId> {
    (0..40).map(|_| CardId::new("S000-C-001")).collect()
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn run_local_game(
    state: GameState,
    registry: CardRegistryImpl,
    client1: Box<dyn PhaseClient>,
    client2: Box<dyn PhaseClient>,
) -> Result<GameResult> {
    let engine = GameEngine::new(state, registry, client1, client2);
    Ok(engine.run())
}

fn find_scripts_root() -> PathBuf {
    let candidates = [
        PathBuf::from("scripts"),
        PathBuf::from("../../scripts"),
    ];
    candidates
        .iter()
        .find(|p| p.join("S000").exists())
        .cloned()
        .unwrap_or(PathBuf::from("scripts"))
}

fn build_game_from_decks(
    player_cards: Vec<CardId>,
    ai_cards: Vec<CardId>,
    scripts_root: PathBuf,
) -> Result<(GameState, CardRegistryImpl)> {
    let index = ScriptIndex::scan(&scripts_root)
        .map_err(|e| anyhow::anyhow!("扫描脚本目录失败: {e}"))?;
    let loader = ScriptLoader::new(index);
    let registry = loader
        .load_for_game(&player_cards, &ai_cards)
        .map_err(|e| anyhow::anyhow!("加载卡片定义失败: {e}"))?;

    let rules = GameRules::default();
    let mut state = GameState::new(rules, 20260322);
    let mut next_id = 1u32;

    for card_id in &player_cards {
        let base_attack = registry
            .get(card_id)
            .and_then(|def| def.attack.map(|a| a as i32));
        state.players[0]
            .zones
            .deck
            .push(CardInstance::new(InstanceId(next_id), card_id.clone(), base_attack));
        next_id += 1;
    }

    for card_id in &ai_cards {
        let base_attack = registry
            .get(card_id)
            .and_then(|def| def.attack.map(|a| a as i32));
        state.players[1]
            .zones
            .deck
            .push(CardInstance::new(InstanceId(next_id), card_id.clone(), base_attack));
        next_id += 1;
    }

    Ok((state, registry))
}

fn build_demo_state(rules: GameRules) -> (GameState, CardRegistryImpl) {
    let mut registry = CardRegistryImpl::new();
    let base_card_id = CardId::new("S000-C-001");
    registry.insert(CardDefinition::new(
        base_card_id.clone(),
        "训练木桩".to_string(),
        CardType::Character,
        Property::Rational,
        Category::Math,
        1,
    ));

    let mut state = GameState::new(rules, 20260322);
    let mut next_id = 1u32;
    for _ in 0..20 {
        state.players[0].zones.deck.push(CardInstance::new(
            InstanceId(next_id),
            base_card_id.clone(),
            Some(300),
        ));
        next_id += 1;
        state.players[1].zones.deck.push(CardInstance::new(
            InstanceId(next_id),
            base_card_id.clone(),
            Some(300),
        ));
        next_id += 1;
    }

    (state, registry)
}
