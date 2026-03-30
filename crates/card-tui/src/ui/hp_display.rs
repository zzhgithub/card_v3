use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

pub fn render_hp(hp: u8, max_hp: u8) -> Span<'static> {
    let filled = hp as usize;
    let empty = (max_hp.saturating_sub(hp)) as usize;

    let stars: String = std::iter::repeat_n('★', filled)
        .chain(std::iter::repeat_n('☆', empty))
        .collect();

    let style = match hp {
        0 | 1 => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        2 => Style::default().fg(Color::Yellow),
        _ => Style::default().fg(Color::Green),
    };

    Span::styled(stars, style)
}

pub fn render_rp(rp: u8, max_rp: u8) -> Span<'static> {
    let filled = rp as usize;
    let empty = (max_rp.saturating_sub(rp)) as usize;

    let circles: String = std::iter::repeat_n('●', filled)
        .chain(std::iter::repeat_n('○', empty))
        .collect();

    Span::styled(circles, Style::default().fg(Color::Blue))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_hp_full() {
        let span = render_hp(5, 5);
        assert_eq!(span.content, "★★★★★");
    }

    #[test]
    fn test_render_hp_partial() {
        let span = render_hp(3, 5);
        assert_eq!(span.content, "★★★☆☆");
    }

    #[test]
    fn test_render_hp_empty() {
        let span = render_hp(0, 5);
        assert_eq!(span.content, "☆☆☆☆☆");
    }

    #[test]
    fn test_render_rp_full() {
        let span = render_rp(6, 6);
        assert_eq!(span.content, "●●●●●●");
    }

    #[test]
    fn test_render_rp_partial() {
        let span = render_rp(2, 6);
        assert_eq!(span.content, "●●○○○○");
    }
}
