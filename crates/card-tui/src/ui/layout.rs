use std::collections::HashSet;

use card_client::api::VisibleGameState;
use card_core::engine::phase::PhaseAction;
use card_core::types::CardDefinition;
use card_core::types::{InstanceId, PlayerId};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::AppMode;
use crate::cursor::FieldCursor;

use super::field::{render_game_board, render_waiting_screen};
use super::info_panel::render_info_panel;
use super::log::{render_log, LogState};
use super::overlay::{
    render_action_overlay, render_minimized_overlay_hint, render_recovery_overlay, OverlayState,
};

pub struct ActionOverlayData<'a> {
    pub actions: &'a [PhaseAction],
    pub selected_action_index: usize,
    pub action_descriptions: &'a [String],
}

pub struct RecoveryOverlayData<'a> {
    pub count: usize,
    pub options: &'a [InstanceId],
    pub selected_recovery_index: usize,
    pub selected_recovery_cards: &'a HashSet<InstanceId>,
    pub option_descriptions: &'a [String],
}

#[allow(clippy::too_many_arguments)]
pub fn render_game_screen(
    frame: &mut Frame<'_>,
    mode: &AppMode,
    game_title: &str,
    visible_state: Option<&VisibleGameState>,
    log_state: &LogState,
    field_cursor: &FieldCursor,
    card_defs: &[CardDefinition],
    action_overlay: Option<ActionOverlayData<'_>>,
    recovery_overlay: Option<RecoveryOverlayData<'_>>,
    overlay_state: &OverlayState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let title = match mode {
        AppMode::LocalGame => "正在创建本地对战...",
        AppMode::OnlineGame => "联网对战：正在连接与匹配...",
        AppMode::InGame => game_title,
        AppMode::GameOver(Some(winner)) => {
            if *winner == PlayerId::Player1 {
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

    if let Some(state) = visible_state {
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(chunks[1]);

        render_game_board(frame, body_chunks[0], state, &[], field_cursor);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(body_chunks[1]);

        render_info_panel(frame, right_chunks[0], field_cursor, state, card_defs);
        render_log(frame, right_chunks[1], log_state);
    } else {
        render_waiting_screen(frame, chunks[1]);
    }

    let hint = if matches!(mode, AppMode::GameOver(_)) {
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

    if overlay_state.visible && overlay_state.minimized {
        render_minimized_overlay_hint(frame, overlay_state.overlay_type);
    } else {
        if let Some(data) = action_overlay {
            render_action_overlay(
                frame,
                data.actions,
                data.selected_action_index,
                data.action_descriptions,
            );
        }

        if let Some(data) = recovery_overlay {
            render_recovery_overlay(
                frame,
                data.count,
                data.options,
                data.selected_recovery_index,
                data.selected_recovery_cards,
                data.option_descriptions,
            );
        }
    }
}
