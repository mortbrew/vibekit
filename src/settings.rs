//! Settings rows: the label on the left, the value on the right, the whole
//! row reversed when selected. See STYLE.md → Settings.

use crate::picker::ThemePicker;
use crate::text::{fit, pad, safe_width};
use crate::theme::{self, Palette};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

fn row(label: &str, value: &str, value_style: Style, selected: bool, width: usize, p: &Palette) -> Line<'static> {
    let value = fit(value, width / 2);
    let label = fit(label, width.saturating_sub(safe_width(&value) + 3));
    let gap = width.saturating_sub(safe_width(&label) + safe_width(&value) + 2);
    let text_style = Style::new().fg(p.fg);
    let mut line = Line::from(vec![
        Span::styled(format!(" {label}"), text_style),
        Span::raw(" ".repeat(gap)),
        Span::styled(format!("{} ", pad(&value, safe_width(&value))), value_style),
    ]);
    if selected {
        line = line.patch_style(Style::new().fg(p.background).bg(p.fg).add_modifier(Modifier::BOLD));
    }
    line
}

/// `Label ............ On` / `Off`. Space, Enter, ←/→ (and h/l) flip it.
pub fn toggle_row(label: &str, on: bool, selected: bool, width: usize, p: &Palette) -> Line<'static> {
    let (text, color) = if on { ("On", p.green) } else { ("Off", p.muted) };
    row(label, text, Style::new().fg(color), selected, width, p)
}

/// True if `key` flips a toggle row.
pub fn toggles(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter | KeyCode::Left | KeyCode::Right | KeyCode::Char('h' | 'l'))
}

/// `Label ........ Value ›`: Enter (or →/l) opens a picker for it.
pub fn picker_row(label: &str, value: &str, selected: bool, width: usize, p: &Palette) -> Line<'static> {
    row(label, &format!("{value} ›"), Style::new().fg(p.accent), selected, width, p)
}

/// True if `key` opens a picker row.
pub fn opens(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Enter | KeyCode::Right | KeyCode::Char('l'))
}

/// The Theme Select picker row: shows the theme in use (by label) and, on
/// [`opens`], hands back a fresh [`ThemePicker`] to start fetching for.
pub struct ThemeRow;

impl ThemeRow {
    pub fn line(current: &str, selected: bool, width: usize, p: &Palette) -> Line<'static> {
        picker_row("Theme Select", &theme::label(current), selected, width, p)
    }

    pub fn key(key: KeyEvent) -> Option<ThemePicker> {
        opens(key).then(ThemePicker::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::tests::render;
    use crate::theme::TOKYO_NIGHT;
    use ratatui::widgets::Paragraph;

    fn text(line: Line<'static>, width: u16) -> String {
        render(width, 1, |f| f.render_widget(Paragraph::new(line), f.area())).0
    }

    #[test]
    fn rows_put_the_value_at_the_right_edge() {
        let p = TOKYO_NIGHT;
        let t = text(toggle_row("Show album art", true, false, 40, &p), 40);
        assert!(t.starts_with(" Show album art") && t.ends_with(" On"), "{t:?}");
        let t = text(toggle_row("Show album art", false, false, 40, &p), 40);
        assert!(t.ends_with(" Off"), "{t:?}");
        let t = text(ThemeRow::line("tokyo-night", false, 40, &p), 40);
        assert!(t.starts_with(" Theme Select") && t.ends_with("Tokyo Night ›"), "{t:?}");
    }

    #[test]
    fn a_long_label_gives_way_to_the_value() {
        let t = text(toggle_row("A really very long setting label that goes on", true, false, 24, &TOKYO_NIGHT), 24);
        assert!(t.ends_with(" On") && t.contains('…'), "{t:?}");
    }

    #[test]
    fn selected_rows_are_reversed() {
        let p = TOKYO_NIGHT;
        let (_, buf) =
            render(30, 1, |f| f.render_widget(Paragraph::new(picker_row("Icons", "Nerd", true, 30, &p)), f.area()));
        assert_eq!(buf[(1, 0)].bg, p.fg);
    }

    #[test]
    fn keys_flip_toggles_and_open_pickers() {
        let k = |c| KeyEvent::from(c);
        assert!(toggles(k(KeyCode::Char(' '))) && toggles(k(KeyCode::Left)));
        assert!(!toggles(k(KeyCode::Char('x'))));
        assert!(ThemeRow::key(k(KeyCode::Enter)).is_some_and(|p| p.loading));
        assert!(ThemeRow::key(k(KeyCode::Char(' '))).is_none());
    }
}
