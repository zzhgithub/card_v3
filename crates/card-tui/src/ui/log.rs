use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum LogSource {
    Me,
    Opponent,
    System,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub message: String,
    pub source: LogSource,
}

pub struct LogState {
    pub entries: Vec<LogEntry>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
}

impl LogState {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        self.entries.push(entry);
        if self.auto_scroll {
            self.scroll_offset = self.entries.len();
        }
    }

    #[allow(dead_code)]
    pub fn scroll_up(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    #[allow(dead_code)]
    pub fn scroll_down(&mut self, visible_height: usize) {
        let max = self.entries.len().saturating_sub(visible_height);
        self.scroll_offset = (self.scroll_offset + 1).min(max);
        if self.scroll_offset >= max {
            self.auto_scroll = true;
        }
    }

    pub fn visible_entries(&self, visible_height: usize) -> &[LogEntry] {
        if self.entries.is_empty() || visible_height == 0 {
            return &[];
        }
        let max_offset = self.entries.len().saturating_sub(visible_height);
        let offset = self.scroll_offset.min(max_offset);
        let end = (offset + visible_height).min(self.entries.len());
        &self.entries[offset..end]
    }
}

impl Default for LogState {
    fn default() -> Self {
        Self::new()
    }
}

fn style_for_source(source: &LogSource) -> (Style, &'static str) {
    match source {
        LogSource::Me => (Style::default().fg(Color::Green), "[己] "),
        LogSource::Opponent => (Style::default().fg(Color::Red), "[敌] "),
        LogSource::System => (Style::default().fg(Color::DarkGray), "[系] "),
    }
}

pub fn render_log(frame: &mut Frame<'_>, area: Rect, log_state: &LogState) {
    let inner_height = area.height.saturating_sub(2) as usize;

    let visible = log_state.visible_entries(inner_height);

    let lines: Vec<Line<'_>> = visible
        .iter()
        .map(|entry| {
            let (style, prefix) = style_for_source(&entry.source);
            Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(entry.message.clone(), style),
            ])
        })
        .collect();

    let block = Block::default().title("事件日志").borders(Borders::ALL);

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);

    if log_state.entries.len() > inner_height {
        let max_offset = log_state.entries.len().saturating_sub(inner_height);
        let current = log_state.scroll_offset.min(max_offset);

        let mut scrollbar_state = ScrollbarState::new(max_offset).position(current);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

#[allow(dead_code)]
pub fn render_event_log(game_events: &[String]) -> Vec<Line<'_>> {
    let mut lines = Vec::new();

    for event in game_events.iter().rev().take(4).rev() {
        lines.push(Line::from(format!("> {}", event)));
    }

    if lines.is_empty() {
        lines.push(Line::from("> 暂无事件，等待对局推进..."));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(source: LogSource, msg: &str) -> LogEntry {
        LogEntry {
            message: msg.to_string(),
            source,
        }
    }

    #[test]
    fn push_appends_entry() {
        let mut state = LogState::new();
        state.push(make_entry(LogSource::System, "hello"));
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].message, "hello");
    }

    #[test]
    fn auto_scroll_advances_offset_on_push() {
        let mut state = LogState::new();
        for i in 0..10 {
            state.push(make_entry(LogSource::Me, &format!("msg {i}")));
        }
        assert!(state.auto_scroll);
        assert_eq!(state.scroll_offset, 10);
    }

    #[test]
    fn push_does_not_advance_offset_when_auto_scroll_off() {
        let mut state = LogState::new();
        state.auto_scroll = false;
        state.scroll_offset = 0;
        state.push(make_entry(LogSource::Me, "msg"));
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn scroll_up_decrements_and_disables_auto_scroll() {
        let mut state = LogState::new();
        state.scroll_offset = 5;
        state.auto_scroll = true;
        state.scroll_up();
        assert_eq!(state.scroll_offset, 4);
        assert!(!state.auto_scroll);
    }

    #[test]
    fn scroll_up_saturates_at_zero() {
        let mut state = LogState::new();
        state.scroll_offset = 0;
        state.scroll_up();
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn scroll_down_increments_and_clamps() {
        let mut state = LogState::new();
        for i in 0..10 {
            state
                .entries
                .push(make_entry(LogSource::System, &format!("{i}")));
        }
        state.scroll_offset = 3;
        state.auto_scroll = false;

        state.scroll_down(5);
        assert_eq!(state.scroll_offset, 4);
        assert!(!state.auto_scroll);
    }

    #[test]
    fn scroll_down_re_enables_auto_scroll_at_bottom() {
        let mut state = LogState::new();
        for i in 0..10 {
            state
                .entries
                .push(make_entry(LogSource::System, &format!("{i}")));
        }
        state.scroll_offset = 4;
        state.auto_scroll = false;

        state.scroll_down(5);
        assert_eq!(state.scroll_offset, 5);
        assert!(state.auto_scroll);
    }

    #[test]
    fn scroll_down_clamps_at_max() {
        let mut state = LogState::new();
        for i in 0..3 {
            state
                .entries
                .push(make_entry(LogSource::System, &format!("{i}")));
        }
        state.scroll_offset = 0;
        state.scroll_down(5);
        assert_eq!(state.scroll_offset, 0);
        assert!(state.auto_scroll);
    }

    #[test]
    fn visible_entries_returns_correct_slice() {
        let mut state = LogState::new();
        for i in 0..10 {
            state
                .entries
                .push(make_entry(LogSource::Me, &format!("msg {i}")));
        }
        state.scroll_offset = 3;

        let vis = state.visible_entries(4);
        assert_eq!(vis.len(), 4);
        assert_eq!(vis[0].message, "msg 3");
        assert_eq!(vis[3].message, "msg 6");
    }

    #[test]
    fn visible_entries_clamps_offset() {
        let mut state = LogState::new();
        for i in 0..5 {
            state
                .entries
                .push(make_entry(LogSource::Opponent, &format!("m{i}")));
        }
        state.scroll_offset = 100;

        let vis = state.visible_entries(3);
        assert_eq!(vis.len(), 3);
        assert_eq!(vis[0].message, "m2");
        assert_eq!(vis[2].message, "m4");
    }

    #[test]
    fn visible_entries_empty_state() {
        let state = LogState::new();
        assert!(state.visible_entries(10).is_empty());
    }

    #[test]
    fn visible_entries_zero_height() {
        let mut state = LogState::new();
        state.push(make_entry(LogSource::System, "hi"));
        assert!(state.visible_entries(0).is_empty());
    }

    #[test]
    fn visible_entries_height_exceeds_entries() {
        let mut state = LogState::new();
        state.entries.push(make_entry(LogSource::Me, "only"));
        state.scroll_offset = 0;

        let vis = state.visible_entries(100);
        assert_eq!(vis.len(), 1);
        assert_eq!(vis[0].message, "only");
    }

    #[test]
    fn legacy_render_event_log_shows_last_four() {
        let events: Vec<String> = (0..6).map(|i| format!("evt{i}")).collect();
        let lines = render_event_log(&events);
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn legacy_render_event_log_empty_shows_placeholder() {
        let lines = render_event_log(&[]);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn log_source_equality() {
        assert_eq!(LogSource::Me, LogSource::Me);
        assert_ne!(LogSource::Me, LogSource::Opponent);
        assert_ne!(LogSource::Opponent, LogSource::System);
    }

    #[test]
    fn log_entry_clone() {
        let e = make_entry(LogSource::Me, "test");
        let e2 = e.clone();
        assert_eq!(e.message, e2.message);
        assert_eq!(e.source, e2.source);
    }
}
