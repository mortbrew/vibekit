//! The `?` help overlay: every key, in sections (Global first, then one per pane).

use super::{Dialog, Response, centered};
use crate::text::{fit, pad, safe_width};
use crate::theme::Palette;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

/// One group of keys: (keys, what they do).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub title: String,
    pub keys: Vec<(String, String)>,
}

impl Section {
    pub fn new(title: impl Into<String>, keys: &[(&str, &str)]) -> Self {
        Self { title: title.into(), keys: keys.iter().map(|(k, d)| ((*k).into(), (*d).into())).collect() }
    }
}

/// The help screen. j/k ↑↓ PgUp/PgDn Home/End scroll; `?` or Esc closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Help {
    pub title: String,
    pub sections: Vec<Section>,
    pub scroll: usize,
}

impl Help {
    pub fn new(sections: Vec<Section>) -> Self {
        Self { title: "Help".into(), sections, scroll: 0 }
    }

    /// Every line, before scrolling: section headings, then aligned keys.
    fn lines(&self, width: usize, p: &Palette) -> Vec<Line<'static>> {
        let key_w =
            self.sections.iter().flat_map(|s| &s.keys).map(|(k, _)| safe_width(k)).max().unwrap_or(0).min(width / 3);
        let mut out = Vec::new();
        for (i, s) in self.sections.iter().enumerate() {
            if i > 0 {
                out.push(Line::from(""));
            }
            out.push(Line::from(Span::styled(
                fit(&s.title, width),
                Style::new().fg(p.accent).add_modifier(Modifier::BOLD),
            )));
            for (k, d) in &s.keys {
                out.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", pad(&fit(k, key_w + 1), key_w)),
                        Style::new().fg(p.fg).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(fit(d, width.saturating_sub(key_w + 3)), Style::new().fg(p.muted)),
                ]));
            }
        }
        out
    }
}

impl Dialog for Help {
    fn key(&mut self, key: KeyEvent) -> Response {
        match key.code {
            KeyCode::Char('?') | KeyCode::Char('q') => return Response::Close,
            KeyCode::Char('j') | KeyCode::Down => self.scroll += 1,
            KeyCode::Char('k') | KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::PageDown | KeyCode::Char(' ') => self.scroll += 10,
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::Home | KeyCode::Char('g') => self.scroll = 0,
            KeyCode::End | KeyCode::Char('G') => self.scroll = usize::MAX / 2,
            _ => {}
        }
        Response::Keep
    }

    fn draw(&self, frame: &mut Frame, screen: Rect, p: &Palette) {
        let width = 72.min(screen.width.saturating_sub(4)).max(screen.width.min(16));
        let inner = usize::from(width.saturating_sub(4));
        let all = self.lines(inner, p);
        let room = usize::from(screen.height.saturating_sub(6)).max(1);
        let top = self.scroll.min(all.len().saturating_sub(room));
        let mut lines: Vec<Line> = all.iter().skip(top).take(room).cloned().collect();
        let more = if all.len() > room {
            format!("{}–{} of {} · ", top + 1, top + lines.len(), all.len())
        } else {
            String::new()
        };
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(fit(&format!("{more}? or Esc close"), inner), Style::new().fg(p.muted))));
        let area = centered(screen, width, lines.len() as u16 + 2);
        frame.render_widget(Clear, area);
        let block = Block::bordered()
            .border_style(Style::new().fg(p.fg))
            .title(format!(" {} ", self.title))
            .style(Style::new().bg(p.background));
        let inner_area = block.inner(area);
        frame.render_widget(block, area);
        let pad_area = Rect { x: inner_area.x + 1, width: inner_area.width.saturating_sub(2), ..inner_area };
        frame.render_widget(Paragraph::new(lines), pad_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::tests::{key, render};
    use crate::theme::TOKYO_NIGHT;

    fn help() -> Help {
        Help::new(vec![
            Section::new("Global", &[("?", "Help"), ("Shift+T", "Theme Select"), ("q", "Quit")]),
            Section::new("Stations", &[("j/k ↑↓", "Move"), ("Enter", "Play")]),
        ])
    }

    #[test]
    fn it_lists_every_section_and_key() {
        let (text, _) = render(80, 24, |f| help().draw(f, f.area(), &TOKYO_NIGHT));
        for want in ["Global", "Stations", "Shift+T", "Theme Select", "j/k ↑↓", "? or Esc close"] {
            assert!(text.contains(want), "{want}\n{text}");
        }
    }

    #[test]
    fn it_scrolls_on_a_short_screen_and_question_mark_closes() {
        let mut h = help();
        let (text, _) = render(60, 9, |f| h.draw(f, f.area(), &TOKYO_NIGHT));
        assert!(text.contains("1–3 of 8"), "{text}");
        h.key(key(KeyCode::End));
        let (text, _) = render(60, 9, |f| h.draw(f, f.area(), &TOKYO_NIGHT));
        assert!(text.contains("Play") && text.contains("6–8 of 8"), "{text}");
        assert_eq!(h.key(key(KeyCode::Char('?'))), Response::Close);
    }
}
