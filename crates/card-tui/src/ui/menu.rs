use card_core::deck::{Deck, DeckSummary};
use card_core::types::{CardDefinition, CardId};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::SelectionPhase;

pub fn render_main_menu(frame: &mut Frame<'_>, selected_index: usize) {
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
        let prefix = if idx == selected_index { "> " } else { "  " };
        let style = if idx == selected_index {
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

pub fn render_deck_browser(frame: &mut Frame<'_>, deck_list: &[DeckSummary], deck_selected: usize) {
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
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        chunks[0],
    );

    let mut lines: Vec<Line> = Vec::new();
    if deck_list.is_empty() {
        lines.push(Line::from("  (desks/ 目录下暂无卡组)"));
    } else {
        for (idx, summary) in deck_list.iter().enumerate() {
            let prefix = if idx == deck_selected { "> " } else { "  " };
            let style = if idx == deck_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
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
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("卡组列表")),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new("↑/↓ 选择 | Esc 返回主菜单")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
}

pub fn render_deck_selection(
    frame: &mut Frame<'_>,
    selection_phase: &SelectionPhase,
    player_deck_name: Option<&str>,
    deck_list: &[DeckSummary],
    deck_selected: usize,
) {
    let title = match selection_phase {
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
        Paragraph::new(title).alignment(Alignment::Center).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        chunks[0],
    );

    let mut lines: Vec<Line> = Vec::new();
    if let Some(name) = player_deck_name {
        lines.push(Line::from(Span::styled(
            format!("  己方卡组: {name}  ✓"),
            Style::default().fg(Color::Green),
        )));
        lines.push(Line::from(""));
    }

    if deck_list.is_empty() {
        lines.push(Line::from("  (desks/ 目录下暂无卡组，请先创建)"));
    } else {
        for (idx, summary) in deck_list.iter().enumerate() {
            let prefix = if idx == deck_selected { "> " } else { "  " };
            let style = if idx == deck_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
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
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title)),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new("↑/↓ 选择 | Enter 确认 | Esc 返回主菜单")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
}

pub fn render_deck_editor(
    frame: &mut Frame<'_>,
    editing_deck: Option<&Deck>,
    all_card_defs: &[CardDefinition],
    editor_focus_right: bool,
    editor_left_index: usize,
    editor_right_index: usize,
) {
    use std::collections::HashMap;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let deck_name = editing_deck.map(|d| d.name.as_str()).unwrap_or("(无卡组)");
    let total = editing_deck.map(|d| d.cards.len()).unwrap_or(0);
    frame.render_widget(
        Paragraph::new(format!("编辑卡组: {deck_name} ({total}/60)"))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        chunks[0],
    );

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let left_title = if editor_focus_right {
        "当前卡组"
    } else {
        "当前卡组 ◀"
    };
    let right_title = if editor_focus_right {
        "可用卡片 ◀"
    } else {
        "可用卡片"
    };

    let mut left_lines: Vec<Line> = Vec::new();
    if let Some(deck) = editing_deck {
        let mut grouped: HashMap<&CardId, usize> = HashMap::new();
        for id in &deck.cards {
            *grouped.entry(id).or_insert(0) += 1;
        }
        let mut entries: Vec<_> = grouped.into_iter().collect();
        entries.sort_by(|a, b| a.0 .0.cmp(&b.0 .0));
        for (idx, (id, count)) in entries.iter().enumerate() {
            let prefix = if !editor_focus_right && idx == editor_left_index {
                "> "
            } else {
                "  "
            };
            let style = if !editor_focus_right && idx == editor_left_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let name = all_card_defs
                .iter()
                .find(|d| &d.id == *id)
                .map(|d| d.name.as_str())
                .unwrap_or("?");
            left_lines.push(Line::from(Span::styled(
                format!("{prefix}{} {} ×{count}", id.0, name),
                style,
            )));
        }
    }
    frame.render_widget(
        Paragraph::new(left_lines).block(Block::default().borders(Borders::ALL).title(left_title)),
        cols[0],
    );

    let selected_def = all_card_defs.get(editor_right_index);
    let mut right_lines: Vec<Line> = Vec::new();

    if editor_focus_right {
        for (idx, def) in all_card_defs.iter().enumerate() {
            let prefix = if idx == editor_right_index {
                "> "
            } else {
                "  "
            };
            let count_in_deck = editing_deck
                .map(|d| d.cards.iter().filter(|id| *id == &def.id).count())
                .unwrap_or(0);
            let style = if idx == editor_right_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            right_lines.push(Line::from(Span::styled(
                format!(
                    "{prefix}{} {} [{}费] ({}/3)",
                    def.id.0, def.name, def.cost, count_in_deck
                ),
                style,
            )));
        }
    } else if let Some(def) = selected_def.or_else(|| {
        editing_deck.and_then(|d| {
            let unique: Vec<CardId> = {
                let mut seen = std::collections::HashSet::new();
                let mut result = Vec::new();
                for id in &d.cards {
                    if seen.insert(id.0.clone()) {
                        result.push(id.clone());
                    }
                }
                result.sort_by(|a, b| a.0.cmp(&b.0));
                result
            };
            unique
                .get(editor_left_index)
                .and_then(|id| all_card_defs.iter().find(|def| &def.id == id))
        })
    }) {
        right_lines.push(Line::from(format!("ID:   {}", def.id.0)));
        right_lines.push(Line::from(format!("名称: {}", def.name)));
        right_lines.push(Line::from(format!("类型: {:?}", def.card_type)));
        right_lines.push(Line::from(format!("属性: {:?}", def.property)));
        right_lines.push(Line::from(format!("范畴: {:?}", def.category)));
        right_lines.push(Line::from(format!("费用: {}", def.cost)));
        if let Some(atk) = def.attack {
            right_lines.push(Line::from(format!("攻击: {atk}")));
        }
        let effect_count = def.effects.len();
        right_lines.push(Line::from(format!(
            "效果: {}",
            if effect_count == 0 {
                "(无)".to_string()
            } else {
                format!("{effect_count} 个")
            }
        )));
    }

    frame.render_widget(
        Paragraph::new(right_lines)
            .block(Block::default().borders(Borders::ALL).title(right_title)),
        cols[1],
    );

    frame.render_widget(
        Paragraph::new("Tab 切换面板 | ↑/↓ 移动 | A 添加 | R 移除 | S 保存 | Esc 返回")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
}
