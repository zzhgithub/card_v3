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
    // 主布局：垂直分割为多个区域
    // 0: 对手手卡
    // 1: 对手区域第1行（费用区+卡组+后场）
    // 2: 对手区域第2行（费用区+墓地+前场）
    // 3: 信息栏
    // 4: 己方区域第1行（费用区+卡组+前场）
    // 5: 己方区域第2行（费用区+墓地+后场）
    // 6: 己方手卡
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 对手手卡
            Constraint::Length(3), // 对手第1行：费用区|卡组|后场
            Constraint::Length(3), // 对手第2行：费用区|墓地|前场
            Constraint::Length(4), // 信息栏
            Constraint::Length(3), // 己方第1行：费用区|卡组|前场
            Constraint::Length(3), // 己方第2行：费用区|墓地|后场
            Constraint::Fill(1),   // 己方手卡
        ])
        .split(area);

    // 渲染对手手卡
    render_hand_zone(
        frame,
        chunks[0],
        state.opponent_public.hand_count,
        "对手手卡",
        false,
    );

    // 渲染对手第1行：费用区 | 卡组 | 后场
    render_opponent_row1(frame, chunks[1], state, field_cursor);

    // 渲染对手第2行：费用区 | 墓地 | 前场
    render_opponent_row2(frame, chunks[2], state, field_cursor);

    // 渲染信息栏
    render_info_bar(frame, chunks[3], state);

    // 渲染己方第1行：费用区 | 卡组 | 前场
    render_my_row1(frame, chunks[4], state, field_cursor);

    // 渲染己方第2行：费用区 | 墓地 | 后场
    render_my_row2(frame, chunks[5], state, field_cursor);

    // 渲染己方手卡
    let hand_highlighted = is_selected_zone(field_cursor, &FieldZone::MyHand(0));
    frame.render_widget(
        Paragraph::new(render_hand(&state.my_state.zones.hand))
            .block(Block::default().borders(Borders::ALL).title("手卡"))
            .style(slot_style(hand_highlighted))
            .wrap(Wrap { trim: true }),
        chunks[6],
    );
}

/// 渲染手卡区域（仅显示数量，不显示具体卡牌）
fn render_hand_zone(
    frame: &mut Frame<'_>,
    area: Rect,
    hand_count: usize,
    title: &str,
    highlighted: bool,
) {
    let content = format!("手卡: {} 张", hand_count);
    frame.render_widget(
        Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(slot_style(highlighted))
            .alignment(Alignment::Center),
        area,
    );
}

/// 渲染对手第1行：费用区 | 卡组 | 后场
fn render_opponent_row1(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &VisibleGameState,
    field_cursor: &FieldCursor,
) {
    // 水平分割：费用区 | 卡组 | 后场×5
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12), // 费用区（竖条）
            Constraint::Length(10), // 卡组
            Constraint::Fill(1),    // 后场5槽
        ])
        .split(area);

    // 费用区 - 竖条显示6个槽位
    render_opponent_cost_zone_vertical(
        frame,
        chunks[0],
        &state.opponent_public.cost_zone,
        "费用",
        field_cursor,
    );

    // 卡组
    render_deck_zone(frame, chunks[1], state.opponent_public.deck_count, "卡组");

    // 后场5槽
    render_field_row(
        frame,
        chunks[2],
        &state.opponent_public.back,
        "后场",
        field_cursor,
        FieldZone::OpponentBack,
    );
}

/// 渲染对手第2行：费用区 | 墓地 | 前场
fn render_opponent_row2(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &VisibleGameState,
    field_cursor: &FieldCursor,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12), // 费用区（竖条，与第1行对齐）
            Constraint::Length(10), // 墓地
            Constraint::Fill(1),    // 前场5槽
        ])
        .split(area);

    // 费用区占位（与第1行对齐）
    render_cost_zone_placeholder(frame, chunks[0]);

    // 墓地
    render_grave_zone(frame, chunks[1], state.opponent_public.grave.len(), "墓地");

    // 前场5槽
    render_field_row(
        frame,
        chunks[2],
        &state.opponent_public.front,
        "前场",
        field_cursor,
        FieldZone::OpponentFront,
    );
}

/// 渲染己方第1行：费用区 | 卡组 | 前场
fn render_my_row1(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &VisibleGameState,
    field_cursor: &FieldCursor,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12), // 费用区（竖条）
            Constraint::Length(10), // 卡组
            Constraint::Fill(1),    // 前场5槽
        ])
        .split(area);

    // 费用区 - 竖条显示6个槽位
    render_my_cost_zone_vertical(
        frame,
        chunks[0],
        &state.my_state.zones.cost_zone,
        "费用",
        field_cursor,
    );

    // 卡组
    render_deck_zone(frame, chunks[1], state.my_state.zones.deck.len(), "卡组");

    // 前场5槽
    render_private_field_row(
        frame,
        chunks[2],
        &state.my_state.zones.front,
        "前场",
        field_cursor,
        FieldZone::MyFront,
    );
}

/// 渲染己方第2行：费用区 | 墓地 | 后场
fn render_my_row2(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &VisibleGameState,
    field_cursor: &FieldCursor,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12), // 费用区（竖条，与第1行对齐）
            Constraint::Length(10), // 墓地
            Constraint::Fill(1),    // 后场5槽
        ])
        .split(area);

    // 费用区占位（与第1行对齐）
    render_cost_zone_placeholder(frame, chunks[0]);

    // 墓地
    render_grave_zone(frame, chunks[1], state.my_state.zones.grave.len(), "墓地");

    // 后场5槽
    render_private_field_row(
        frame,
        chunks[2],
        &state.my_state.zones.back,
        "后场",
        field_cursor,
        FieldZone::MyBack,
    );
}

/// 渲染竖条费用区（己方版本）
fn render_my_cost_zone_vertical(
    frame: &mut Frame<'_>,
    area: Rect,
    cards: &[CardInstance],
    title: &str,
    field_cursor: &FieldCursor,
) {
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 在竖条区域内显示6个槽位
    let lines: Vec<Line> = (0..6)
        .map(|idx| {
            let text = cards
                .get(idx)
                .map(render_instance_card)
                .unwrap_or_else(|| EMPTY_SLOT.to_string());
            let line = format!("{:>1}", text);
            if is_selected_zone(field_cursor, &FieldZone::MyCostZone(idx)) {
                Line::styled(line, slot_style(true))
            } else {
                Line::from(line)
            }
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

/// 渲染竖条费用区（对手版本）
fn render_opponent_cost_zone_vertical(
    frame: &mut Frame<'_>,
    area: Rect,
    cards: &[CardPublicInfo],
    title: &str,
    field_cursor: &FieldCursor,
) {
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 在竖条区域内显示6个槽位
    let lines: Vec<Line> = (0..6)
        .map(|idx| {
            // 对手只显示是否有卡
            let text = if idx < cards.len() {
                "[●]".to_string()
            } else {
                EMPTY_SLOT.to_string()
            };
            let line = format!("{:>1}", text);
            if is_selected_zone(field_cursor, &FieldZone::OpponentCostZone(idx)) {
                Line::styled(line, slot_style(true))
            } else {
                Line::from(line)
            }
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

/// 渲染费用区占位（用于第2行对齐）
fn render_cost_zone_placeholder(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(Block::default().borders(Borders::ALL), area);
}

/// 渲染卡组区域
fn render_deck_zone(frame: &mut Frame<'_>, area: Rect, count: usize, title: &str) {
    let content = format!("{}\n[{}]", count, if count > 0 { "▓▓" } else { "  " });
    frame.render_widget(
        Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL).title(title))
            .alignment(Alignment::Center),
        area,
    );
}

/// 渲染墓地区域
fn render_grave_zone(frame: &mut Frame<'_>, area: Rect, count: usize, title: &str) {
    let content = format!("{}\n[{}]", count, if count > 0 { "▒▒" } else { "  " });
    frame.render_widget(
        Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL).title(title))
            .alignment(Alignment::Center),
        area,
    );
}

/// 渲染公开场地区域行（对手）
fn render_field_row(
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

/// 渲染私有场地区域行（己方）
fn render_private_field_row(
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

/// 渲染信息栏
fn render_info_bar(frame: &mut Frame<'_>, area: Rect, state: &VisibleGameState) {
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
            Span::raw(format!("  手牌: {}", state.opponent_public.hand_count)),
        ]),
        Line::from(vec![
            Span::raw("己方 HP★ "),
            render_hp(state.my_state.hp, DEFAULT_MAX_HP),
            Span::raw("  RP● "),
            render_rp(state.my_state.real_point, DEFAULT_MAX_RP),
            Span::raw(format!(
                "  卡组: {}  墓地: {}  费用区: {}",
                state.my_state.zones.deck.len(),
                state.my_state.zones.grave.len(),
                state.my_state.zones.cost_zone.len()
            )),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(info_lines)
            .block(Block::default().borders(Borders::ALL).title("信息栏"))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn is_selected_zone(cursor: &FieldCursor, zone: &FieldZone) -> bool {
    cursor.active && *cursor.current_zone() == *zone
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
