//! Toasts: short timed notes stacked in the bottom-right corner, newest at
//! the bottom. They never take keys.

use crate::text::{fit, safe_width};
use crate::theme::Palette;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub text: String,
    pub kind: Kind,
    pub until: Instant,
}

/// How long a toast stays by default, and how many show at once.
pub const SHOW_FOR: Duration = Duration::from_secs(4);
pub const MAX: usize = 3;

#[derive(Debug, Clone, Default)]
pub struct Toasts {
    items: VecDeque<Toast>,
}

impl Toasts {
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows `text` for [`SHOW_FOR`]; the oldest goes past [`MAX`].
    pub fn push(&mut self, text: impl Into<String>, kind: Kind, now: Instant) {
        self.push_for(text, kind, now, SHOW_FOR);
    }

    pub fn push_for(&mut self, text: impl Into<String>, kind: Kind, now: Instant, show_for: Duration) {
        self.items.push_back(Toast { text: text.into(), kind, until: now + show_for });
        while self.items.len() > MAX {
            self.items.pop_front();
        }
    }

    /// Drops the expired ones; true if any went (redraw).
    pub fn tick(&mut self, now: Instant) -> bool {
        let before = self.items.len();
        self.items.retain(|t| t.until > now);
        self.items.len() != before
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Toast> {
        self.items.iter()
    }

    /// Draws the live ones (at `now`) in `area`'s bottom-right corner.
    pub fn draw(&self, frame: &mut Frame, area: Rect, p: &Palette, now: Instant) {
        let max_w = usize::from(area.width.saturating_sub(2)).min(48);
        let mut bottom = area.bottom();
        for t in self.items.iter().rev().filter(|t| t.until > now) {
            if bottom < area.top() + 3 || max_w < 6 {
                break;
            }
            let text = fit(&t.text, max_w.saturating_sub(3));
            let w = (safe_width(&text) + 4).min(max_w + 2) as u16;
            let rect = Rect::new(area.right().saturating_sub(w + 1).max(area.x), bottom - 3, w, 3);
            let color = match t.kind {
                Kind::Info => p.accent,
                Kind::Success => p.green,
                Kind::Error => p.red,
            };
            frame.render_widget(Clear, rect);
            let block = Block::bordered().border_style(Style::new().fg(color)).style(Style::new().bg(p.background));
            frame.render_widget(
                Paragraph::new(Line::styled(format!(" {text}"), Style::new().fg(p.fg))).block(block),
                rect,
            );
            bottom -= 3;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::tests::render;
    use crate::theme::TOKYO_NIGHT;

    #[test]
    fn toasts_expire_and_keep_only_the_newest() {
        let t0 = Instant::now();
        let mut toasts = Toasts::new();
        for i in 0..5 {
            toasts.push(format!("note {i}"), Kind::Info, t0);
        }
        let texts: Vec<_> = toasts.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["note 2", "note 3", "note 4"]);
        toasts.push_for("quick", Kind::Error, t0, Duration::from_secs(1));
        assert!(toasts.tick(t0 + Duration::from_secs(2)));
        assert_eq!(toasts.iter().count(), 2, "the quick one went");
        assert!(!toasts.tick(t0 + Duration::from_secs(2)));
        toasts.tick(t0 + SHOW_FOR);
        assert!(toasts.is_empty());
    }

    #[test]
    fn they_stack_newest_at_the_bottom_right() {
        let t0 = Instant::now();
        let mut toasts = Toasts::new();
        toasts.push("Saved theme", Kind::Success, t0);
        toasts.push("Update available", Kind::Info, t0);
        let (text, _) = render(60, 12, |f| toasts.draw(f, f.area(), &TOKYO_NIGHT, t0));
        let rows: Vec<&str> = text.lines().collect();
        assert!(rows[10].contains("Update available"), "{text}");
        assert!(rows[7].contains("Saved theme"), "{text}");
        assert!(rows[10].trim_end().len() >= 55, "right-aligned\n{text}");
        let (later, _) = render(60, 12, |f| toasts.draw(f, f.area(), &TOKYO_NIGHT, t0 + SHOW_FOR));
        assert!(later.trim().is_empty(), "{later}");
    }
}
