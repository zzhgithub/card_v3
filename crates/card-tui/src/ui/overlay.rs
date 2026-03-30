use std::collections::HashSet;

use card_core::engine::phase::PhaseAction;
use card_core::types::InstanceId;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayType {
    ActionSelect,
    RecoverySelect,
    #[default]
    None,
}

impl OverlayType {
    pub fn minimized_hint_label(self) -> &'static str {
        match self {
            Self::ActionSelect => "操作选择",
            Self::RecoverySelect => "回收选择",
            Self::None => "弹窗",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OverlayState {
    pub visible: bool,
    pub minimized: bool,
    pub overlay_type: OverlayType,
}

impl OverlayState {
    pub fn for_action() -> Self {
        Self {
            visible: true,
            minimized: false,
            overlay_type: OverlayType::ActionSelect,
        }
    }

    pub fn for_recovery() -> Self {
        Self {
            visible: true,
            minimized: false,
            overlay_type: OverlayType::RecoverySelect,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn toggle_minimized(&mut self) {
        if self.visible {
            self.minimized = !self.minimized;
        }
    }
}

pub fn render_action_overlay(
    frame: &mut Frame<'_>,
    actions: &[PhaseAction],
    selected_action_index: usize,
    action_descriptions: &[String],
) {
    let popup = centered_rect(60, 50, frame.area());

    let mut lines = Vec::new();
    for (idx, _action) in actions.iter().enumerate() {
        let prefix = if idx == selected_action_index {
            "> "
        } else {
            "  "
        };
        lines.push(Line::from(format!(
            "{prefix}{}",
            action_descriptions
                .get(idx)
                .map(String::as_str)
                .unwrap_or("-")
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from("↑/↓ 选择  Enter 确认  Esc 跳过"));

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("选择操作")
                .style(Style::default().bg(Color::Rgb(20, 20, 40))),
        ),
        popup,
    );
}

pub fn render_recovery_overlay(
    frame: &mut Frame<'_>,
    count: usize,
    options: &[InstanceId],
    selected_recovery_index: usize,
    selected_recovery_cards: &HashSet<InstanceId>,
    option_descriptions: &[String],
) {
    let popup = centered_rect(60, 50, frame.area());

    let mut lines = Vec::new();
    lines.push(Line::from(format!("请最多选择 {count} 张回收卡")));
    lines.push(Line::from(""));

    for (idx, instance_id) in options.iter().enumerate() {
        let cursor = if idx == selected_recovery_index {
            ">"
        } else {
            " "
        };
        let selected = if selected_recovery_cards.contains(instance_id) {
            "x"
        } else {
            " "
        };
        lines.push(Line::from(format!(
            "{cursor} [{selected}] {}",
            option_descriptions
                .get(idx)
                .map(String::as_str)
                .unwrap_or("-")
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from("↑/↓ 移动  Space 勾选  Enter 确认  Esc 取消"));

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("回收选择")
                .style(Style::default().bg(Color::Rgb(20, 20, 40))),
        ),
        popup,
    );
}

pub fn render_minimized_overlay_hint(frame: &mut Frame<'_>, overlay_type: OverlayType) {
    let hint = format!("[{}] 按 m 恢复", overlay_type.minimized_hint_label());
    let area = frame.area();
    let width = ((hint.chars().count() as u16) + 2).min(area.width.max(1));
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(2);
    let hint_rect = Rect::new(x, y, width, 1);

    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::Yellow)),
        hint_rect,
    );
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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
