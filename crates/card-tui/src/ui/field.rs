use card_client::api::{CardPublicInfo, VisibleGameState};
use card_core::state::{CardInstance, Phase};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::cursor::{FieldCursor, FieldZone};

use super::hand::render_hand;
use super::hp_display::{render_hp, render_rp};

pub const EMPTY_SLOT: &str = "[ --- ]";
pub const DEFAULT_MAX_HP: u8 = 5;
pub const DEFAULT_MAX_RP: u8 = 6;

pub fn render_game_board(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &VisibleGameState,
    _game_events: &[String],
    field_cursor: &FieldCursor,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Fill(1),
        ])
        .split(area);

    render_public_row(
        frame,
        chunks[0],
        &state.opponent_public.back,
        "对手后场",
        field_cursor,
        FieldZone::OpponentBack,
    );
    render_public_row(
        frame,
        chunks[1],
        &state.opponent_public.front,
        "对手前场",
        field_cursor,
        FieldZone::OpponentFront,
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
        Line::from(vec![
            Span::raw("对手 HP★ "),
            render_hp(state.opponent_public.hp, DEFAULT_MAX_HP),
            Span::raw("  RP● "),
            render_rp(state.opponent_public.real_point, DEFAULT_MAX_RP),
            Span::raw(format!(
                "  手牌: [?] x {}",
                state.opponent_public.hand_count
            )),
        ]),
        Line::from(vec![
            Span::raw("己方 HP★ "),
            render_hp(state.my_state.hp, DEFAULT_MAX_HP),
            Span::raw("  RP● "),
            render_rp(state.my_state.real_point, DEFAULT_MAX_RP),
            Span::raw(format!(
                "  卡组: {}  墓地: {}",
                state.my_state.zones.deck.len(),
                state.my_state.zones.grave.len()
            )),
        ]),
        Line::from(format!(
            "对手费用区: {} 张 | 己方费用区: {} 张",
            state.opponent_public.cost_zone.len(),
            state.my_state.zones.cost_zone.len(),
        )),
    ];
    frame.render_widget(
        Paragraph::new(info_lines)
            .block(Block::default().borders(Borders::ALL).title("信息栏"))
            .wrap(Wrap { trim: true }),
        chunks[2],
    );

    render_private_row(
        frame,
        chunks[3],
        &state.my_state.zones.front,
        "己方前场",
        field_cursor,
        FieldZone::MyFront,
    );
    render_private_row(
        frame,
        chunks[4],
        &state.my_state.zones.back,
        "己方后场",
        field_cursor,
        FieldZone::MyBack,
    );

    render_cost_zones(frame, chunks[5], state, field_cursor);

    let hand_highlighted = is_selected_zone(field_cursor, &FieldZone::MyHand(0));

    frame.render_widget(
        Paragraph::new(render_hand(&state.my_state.zones.hand))
            .block(Block::default().borders(Borders::ALL).title("手牌"))
            .style(slot_style(hand_highlighted))
            .wrap(Wrap { trim: true }),
        chunks[6],
    );
}

fn render_public_row(
    frame: &mut Frame<'_>,
    area: Rect,
    row: &[Option<CardPublicInfo>; 5],
    title: &str,
    field_cursor: &FieldCursor,
    zone_of: fn(usize) -> FieldZone,
) {
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let slots = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20); 5])
        .split(inner);

    for (idx, card) in row.iter().enumerate() {
        let highlighted = is_selected_zone(field_cursor, &zone_of(idx));
        frame.render_widget(
            Paragraph::new(render_card_slot(card))
                .alignment(Alignment::Center)
                .style(slot_style(highlighted)),
            slots[idx],
        );
    }
}

fn render_private_row(
    frame: &mut Frame<'_>,
    area: Rect,
    row: &[Option<CardInstance>; 5],
    title: &str,
    field_cursor: &FieldCursor,
    zone_of: fn(usize) -> FieldZone,
) {
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let slots = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20); 5])
        .split(inner);

    for (idx, card) in row.iter().enumerate() {
        let highlighted = is_selected_zone(field_cursor, &zone_of(idx));
        frame.render_widget(
            Paragraph::new(render_instance_slot(card))
                .alignment(Alignment::Center)
                .style(slot_style(highlighted)),
            slots[idx],
        );
    }
}

fn render_cost_zones(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &VisibleGameState,
    field_cursor: &FieldCursor,
) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let my_cost_lines = (0..6)
        .map(|idx| {
            let text = state
                .my_state
                .zones
                .cost_zone
                .get(idx)
                .map(render_instance_card)
                .unwrap_or_else(|| EMPTY_SLOT.to_string());
            let line = format!("{:>2}. {}", idx + 1, text);
            if is_selected_zone(field_cursor, &FieldZone::MyCostZone(idx)) {
                Line::styled(line, slot_style(true))
            } else {
                Line::from(line)
            }
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(my_cost_lines)
            .block(Block::default().borders(Borders::ALL).title("己方费用区"))
            .wrap(Wrap { trim: true }),
        columns[0],
    );

    let mut opponent_lines = vec![Line::from(format!(
        "总数: {} 张",
        state.opponent_public.cost_zone.len()
    ))];
    for idx in 0..6 {
        let text = state
            .opponent_public
            .cost_zone
            .get(idx)
            .map(render_public_card)
            .unwrap_or_else(|| EMPTY_SLOT.to_string());
        let line = format!("{:>2}. {}", idx + 1, text);
        if is_selected_zone(field_cursor, &FieldZone::OpponentCostZone(idx)) {
            opponent_lines.push(Line::styled(line, slot_style(true)));
        } else {
            opponent_lines.push(Line::from(line));
        }
    }

    frame.render_widget(
        Paragraph::new(opponent_lines)
            .block(Block::default().borders(Borders::ALL).title("对手费用区"))
            .wrap(Wrap { trim: true }),
        columns[1],
    );
}

fn is_selected_zone(cursor: &FieldCursor, zone: &FieldZone) -> bool {
    cursor.active && cursor.current_zone() == zone
}

fn slot_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

pub fn render_waiting_screen(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new("等待游戏开始...")
            .block(Block::default().borders(Borders::ALL).title("游戏板面")),
        area,
    );
}

fn render_instance_slot(card: &Option<CardInstance>) -> String {
    match card {
        Some(info) => render_instance_card(info),
        None => EMPTY_SLOT.to_string(),
    }
}

fn render_card_slot(card: &Option<CardPublicInfo>) -> String {
    match card {
        Some(info) => render_public_card(info),
        None => EMPTY_SLOT.to_string(),
    }
}

fn render_instance_card(info: &CardInstance) -> String {
    format!(
        "[{} {}]",
        compact_card_id(&info.definition_id.0),
        info.current_attack.unwrap_or(0)
    )
}

fn render_public_card(info: &CardPublicInfo) -> String {
    format!(
        "[{} {}]",
        compact_card_id(&info.definition_id.0),
        info.current_attack.unwrap_or(0)
    )
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
