use card_core::state::CardInstance;
use card_core::types::CardDefinition;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

#[allow(dead_code)]
pub fn render_hand(hand: &[CardInstance]) -> String {
    if hand.is_empty() {
        return "(空)".to_string();
    }

    hand.iter()
        .map(|card| format!("[{}]", compact_card_id(&card.definition_id.0)))
        .collect::<Vec<_>>()
        .join(" ")
}

#[allow(dead_code)]
pub fn render_my_hand(hand: &[CardInstance], card_defs: &[CardDefinition]) -> Vec<Line<'static>> {
    if hand.is_empty() {
        return vec![Line::from("(空)")];
    }

    let mut lines = Vec::new();
    let total = hand.len();

    for chunk in hand.chunks(10) {
        let mut parts = Vec::with_capacity(chunk.len());
        for card in chunk {
            let label = card_defs
                .iter()
                .find(|def| def.id.0 == card.definition_id.0)
                .map(|def| format!("{} {}", def.id.0, def.name))
                .unwrap_or_else(|| compact_card_id(&card.definition_id.0));
            parts.push(format!("[{}]", label));
        }

        lines.push(Line::from(parts.join(" ")));
    }

    if let Some(last) = lines.last_mut() {
        last.spans.push(Span::raw(format!(" ({}/20)", total)));
    }

    lines
}

#[allow(dead_code)]
pub fn render_opponent_hand(hand_count: usize) -> Line<'static> {
    if hand_count == 0 {
        return Line::from("(0张)");
    }

    let face_down_count = hand_count.min(10);
    let hidden_cards = (0..face_down_count).flat_map(|idx| {
        let mut spans = vec![Span::styled("[?]", Style::default().fg(Color::DarkGray))];
        if idx + 1 < face_down_count {
            spans.push(Span::raw(" "));
        }
        spans
    });

    let mut spans: Vec<Span<'static>> = hidden_cards.collect();
    if hand_count > 10 {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::raw(format!("... +{}张", hand_count - 10)));
    }
    spans.push(Span::raw(format!(" ({}张)", hand_count)));

    Line::from(spans)
}

fn compact_card_id(raw: &str) -> String {
    let mut parts = raw.split('-');
    let _pack = parts.next();
    let card_type = parts.next().unwrap_or("?");
    let card_num = parts.next().unwrap_or("---");
    format!("{}{}", card_type, card_num)
}
