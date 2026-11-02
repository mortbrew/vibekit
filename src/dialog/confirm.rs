//! The yes/no dialog (quit, delete, …).

use super::{Dialog, Response, centered};
use crate::text::wrap;
use crate::theme::Palette;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

/// A yes/no question. `y` says yes, `n` no; ←→/Tab move between the buttons
/// and Enter presses the chosen one, which starts on **No**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirm {
    /// Comes back in [`Response::Answer`] so the app knows which question.
    pub id: String,
    pub title: String,
    pub question: String,
    pub yes: String,
    pub no: String,
    /// Which button Enter presses.
    pub on_yes: bool,
    /// Another key that also says yes (`Q` on the quit question).
    pub also_yes: Option<char>,
    /// The line under the buttons, if any.
    pub hint: Option<String>,
}

impl Confirm {
    pub fn new(id: impl Into<String>, question: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: "Confirm".into(),
            question: question.into(),
            yes: "Yes".into(),
            no: "No".into(),
            on_yes: false,
            also_yes: None,
            hint: None,
        }
    }

    /// The standard quit question for `app`: `Q` (the only quit key) opens
    /// it and `Q` again confirms; `y`/`n` work too; lowercase `q` does nothing.
    pub fn quit(app: &str) -> Self {
        Self {
            title: "Quit".into(),
            yes: "Quit".into(),
            also_yes: Some('Q'),
            hint: Some("Q again to quit · Esc stay".into()),
            ..Self::new("quit", format!("Quit {app}?"))
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn labels(mut self, yes: impl Into<String>, no: impl Into<String>) -> Self {
        self.yes = yes.into();
        self.no = no.into();
        self
    }

    fn answer(&self, yes: bool) -> Response {
        Response::Answer { id: self.id.clone(), yes }
    }
}

impl Dialog for Confirm {
    fn key(&mut self, key: KeyEvent) -> Response {
        match key.code {
            KeyCode::Char(c) if Some(c) == self.also_yes => self.answer(true),
            KeyCode::Char('y' | 'Y') => self.answer(true),
            KeyCode::Char('n' | 'N') => self.answer(false),
            KeyCode::Enter => self.answer(self.on_yes),
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab | KeyCode::Char('h' | 'l') => {
                self.on_yes = !self.on_yes;
                Response::Keep
            }
            _ => Response::Keep,
        }
    }

    fn draw(&self, frame: &mut Frame, screen: Rect, p: &Palette) {
        let width = 44.min(screen.width.saturating_sub(4)).max(screen.width.min(12));
        let inner = usize::from(width.saturating_sub(4));
        let room = usize::from(screen.height.saturating_sub(7)).max(1);
        let mut lines = vec![Line::from("")];
        lines.extend(
            wrap(&self.question, inner, room).into_iter().map(|l| Line::from(Span::styled(l, Style::new().fg(p.fg)))),
        );
        lines.push(Line::from(""));
        let button = |label: &str, key: char, on: bool| {
            let style = if on {
                Style::new().fg(p.background).bg(p.red).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(p.muted)
            };
            Span::styled(format!(" {label} ({key}) "), style)
        };
        lines.push(Line::from(vec![
            button(&self.yes, 'y', self.on_yes),
            Span::raw("   "),
            button(&self.no, 'n', !self.on_yes),
        ]));
        if let Some(hint) = &self.hint {
            lines.push(Line::from(Span::styled(crate::text::fit(hint, inner + 1), Style::new().fg(p.muted))));
        }
        lines.push(Line::from(""));
        let area = centered(screen, width, lines.len() as u16 + 2);
        frame.render_widget(Clear, area);
        let block = Block::bordered()
            .border_style(Style::new().fg(p.red))
            .title(format!(" {} ", self.title))
            .style(Style::new().bg(p.background));
        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center).block(block), area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::tests::{key, render};
    use crate::theme::TOKYO_NIGHT;

    #[test]
    fn enter_presses_no_until_moved_to_yes() {
        let mut c = Confirm::quit("boxvibe");
        assert_eq!(c.key(key(KeyCode::Enter)), Response::Answer { id: "quit".into(), yes: false });
        assert_eq!(c.key(key(KeyCode::Right)), Response::Keep);
        assert_eq!(c.key(key(KeyCode::Enter)), Response::Answer { id: "quit".into(), yes: true });
        assert_eq!(c.key(key(KeyCode::Char('n'))), Response::Answer { id: "quit".into(), yes: false });
        assert_eq!(c.key(key(KeyCode::Char('x'))), Response::Keep);
    }

    #[test]
    fn the_quit_question_takes_capital_q_and_ignores_lowercase_q() {
        let yes = Response::Answer { id: "quit".into(), yes: true };
        let mut c = Confirm::quit("boxvibe");
        assert_eq!(c.key(key(KeyCode::Char('q'))), Response::Keep, "lowercase q does nothing");
        assert_eq!(c.key(key(KeyCode::Char('Q'))), yes);
        assert_eq!(c.key(key(KeyCode::Char('y'))), yes);
        assert_eq!(c.key(key(KeyCode::Char('n'))), Response::Answer { id: "quit".into(), yes: false });
        assert_eq!(Confirm::new("del", "Delete?").key(key(KeyCode::Char('Q'))), Response::Keep, "only quit takes Q");
        let (text, _) = render(60, 12, |f| c.draw(f, f.area(), &TOKYO_NIGHT));
        assert!(text.contains("Quit boxvibe?") && text.contains("Q again to quit"), "{text}");
    }

    #[test]
    fn it_draws_the_question_and_both_buttons_on_a_small_screen() {
        let c = Confirm::new("del", "Delete the theme “A very long theme name indeed”? Its file goes too.")
            .title("Delete theme")
            .labels("Delete", "Keep");
        for cols in [30u16, 80] {
            let (text, _) = render(cols, 12, |f| c.draw(f, f.area(), &TOKYO_NIGHT));
            assert!(text.contains("Delete (y)") && text.contains("Keep (n)"), "{cols}\n{text}");
            assert!(text.contains(" Delete theme "), "{cols}\n{text}");
        }
    }
}
