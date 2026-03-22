use card_client::api::{CardPublicInfo, VisibleGameState};
use card_core::state::{CardInstance, Phase};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

const EMPTY_SLOT: &str = "[  ---  ]";
const DEFAULT_MAX_HP: u8 = 5;

pub fn render_game_board(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &VisibleGameState,
    game_events: &[String],
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(render_public_row(&state.opponent_public.back))
            .block(Block::default().borders(Borders::ALL).title("对手后场")),
        chunks[0],
    );

    frame.render_widget(
        Paragraph::new(render_public_row(&state.opponent_public.front))
            .block(Block::default().borders(Borders::ALL).title("对手前场")),
        chunks[1],
    );

    let info_lines = vec![
        Line::from(format!(
            "回合: {} | 阶段: {} | 连锁: {} | {}",
            state.turn_number,
            phase_name(state.phase),
            state.chain_size,
            if state.is_my_turn {
                "当前行动方: 己方"
            } else {
                "当前行动方: 对手"
            }
        )),
        Line::from(format!(
            "对手HP: {}/{} RP: {} | 己方HP: {}/{} RP: {}",
            state.opponent_public.hp,
            DEFAULT_MAX_HP,
            state.opponent_public.real_point,
            state.my_state.hp,
            DEFAULT_MAX_HP,
            state.my_state.real_point
        )),
        Line::from(format!(
            "对手手牌: [?] x {} | 对手费用区 ({}张) | 己方卡组: {} | 己方费用区 ({}张) | 己方墓地: {}",
            state.opponent_public.hand_count,
            state.opponent_public.cost_zone.len(),
            state.my_state.zones.deck.len(),
            state.my_state.zones.cost_zone.len(),
            state.my_state.zones.grave.len(),
        )),
    ];
    frame.render_widget(
        Paragraph::new(info_lines).block(Block::default().borders(Borders::ALL).title("信息栏")),
        chunks[2],
    );

    frame.render_widget(
        Paragraph::new(render_private_row(&state.my_state.zones.front))
            .block(Block::default().borders(Borders::ALL).title("己方前场")),
        chunks[3],
    );

    frame.render_widget(
        Paragraph::new(render_private_row(&state.my_state.zones.back))
            .block(Block::default().borders(Borders::ALL).title("己方后场")),
        chunks[4],
    );

    frame.render_widget(
        Paragraph::new(render_hand(&state.my_state.zones.hand))
            .block(Block::default().borders(Borders::ALL).title("手牌"))
            .wrap(Wrap { trim: true }),
        chunks[5],
    );

    frame.render_widget(
        Paragraph::new(render_event_log(game_events))
            .block(Block::default().borders(Borders::ALL).title("事件日志"))
            .wrap(Wrap { trim: false }),
        chunks[6],
    );
}

pub fn render_waiting_screen(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new("等待游戏开始...")
            .block(Block::default().borders(Borders::ALL).title("游戏板面")),
        area,
    );
}

fn render_public_row(row: &[Option<CardPublicInfo>; 5]) -> String {
    row.iter()
        .map(render_card_slot)
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_private_row(row: &[Option<CardInstance>; 5]) -> String {
    row.iter()
        .map(render_instance_slot)
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_hand(hand: &[CardInstance]) -> String {
    if hand.is_empty() {
        return "(空)".to_string();
    }

    hand.iter()
        .map(|card| format!("[{}]", compact_card_id(&card.definition_id.0)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_event_log(game_events: &[String]) -> Vec<Line<'_>> {
    let mut lines = Vec::new();

    for event in game_events.iter().rev().take(4).rev() {
        lines.push(Line::from(format!("> {}", event)));
    }

    if lines.is_empty() {
        lines.push(Line::from("> 暂无事件，等待对局推进..."));
    }

    lines
}

fn render_instance_slot(card: &Option<CardInstance>) -> String {
    match card {
        Some(info) => format!(
            "[{} {}]",
            compact_card_id(&info.definition_id.0),
            info.current_attack.unwrap_or(0)
        ),
        None => EMPTY_SLOT.to_string(),
    }
}

fn render_card_slot(card: &Option<CardPublicInfo>) -> String {
    match card {
        Some(info) => format!(
            "[{} {}]",
            compact_card_id(&info.definition_id.0),
            info.current_attack.unwrap_or(0)
        ),
        None => EMPTY_SLOT.to_string(),
    }
}

fn compact_card_id(raw: &str) -> String {
    let mut parts = raw.split('-');
    let _pack = parts.next();
    let card_type = parts.next().unwrap_or("?");
    let card_num = parts.next().unwrap_or("---");
    format!("{}{}", card_type, card_num)
}

pub fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::TurnStart => "回合开始",
        Phase::Draw => "抽牌",
        Phase::Recovery => "回收",
        Phase::Main1 => "主阶段1",
        Phase::Battle => "战斗",
        Phase::Main2 => "主阶段2",
        Phase::TurnEnd => "回合结束",
    }
}
