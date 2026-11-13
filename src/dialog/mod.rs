//! Modal dialogs: a stack drawn over a dimmed (fading) screen.
//!
//! The app keeps one [`DialogStack`]. While it is open, keys go to
//! [`DialogStack::key`] first; Esc always closes the topmost dialog. Draw the
//! app as usual, then [`DialogStack::draw`] on top.

mod ask;
mod confirm;
mod help;

pub use ask::Ask;
pub use confirm::Confirm;
pub use help::{Help, Section};

use crate::theme::Palette;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use std::any::Any;
use std::time::{Duration, Instant};

/// What a dialog says after a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// Stay open (the key was used, or ignored).
    Keep,
    /// Close this dialog with no answer (Esc, or `?` on help).
    Close,
    /// A yes/no dialog was answered; `id` says which one.
    Answer { id: String, yes: bool },
}

/// One modal dialog.
/// `Any` lets the app find its own dialog types on the stack again
/// ([`DialogStack::top`], [`DialogStack::find_mut`]), e.g. to update one as data arrives.
pub trait Dialog: Any {
    /// Handles a key (Esc never arrives: the stack closes the dialog first).
    fn key(&mut self, key: KeyEvent) -> Response;
    /// Draws itself over `screen` (already dimmed).
    fn draw(&self, frame: &mut Frame, screen: Rect, palette: &Palette);
}

/// How long the background takes to dim, and how far it goes (0–1).
pub const FADE: Duration = Duration::from_millis(150);
pub const DIM: f32 = 0.55;
/// How much chroma remains when fully dimmed (0 = grey, 1 = full colour).
pub const DIM_SATURATION: f32 = 0.5;

/// Open dialogs, bottom first.
#[derive(Default)]
pub struct DialogStack {
    stack: Vec<Box<dyn Dialog>>,
    /// When the first dialog opened (the fade starts then).
    opened: Option<Instant>,
}

impl DialogStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens `dialog` on top. `now` starts the fade if nothing was open.
    pub fn push(&mut self, dialog: impl Dialog + 'static, now: Instant) {
        if self.stack.is_empty() {
            self.opened = Some(now);
        }
        self.stack.push(Box::new(dialog));
    }

    /// Closes the topmost dialog.
    pub fn pop(&mut self) -> bool {
        let closed = self.stack.pop().is_some();
        if self.stack.is_empty() {
            self.opened = None;
        }
        closed
    }

    pub fn clear(&mut self) {
        self.stack.clear();
        self.opened = None;
    }

    pub fn is_open(&self) -> bool {
        !self.stack.is_empty()
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// The topmost dialog, if it is a `T`.
    pub fn top<T: Dialog>(&self) -> Option<&T> {
        let top: &dyn Any = self.stack.last()?.as_ref();
        top.downcast_ref()
    }

    /// The topmost dialog, if it is a `T`, to change.
    pub fn top_mut<T: Dialog>(&mut self) -> Option<&mut T> {
        let top: &mut dyn Any = self.stack.last_mut()?.as_mut();
        top.downcast_mut()
    }

    /// The highest open `T`, anywhere on the stack.
    pub fn find<T: Dialog>(&self) -> Option<&T> {
        self.stack.iter().rev().find_map(|d| (d.as_ref() as &dyn Any).downcast_ref())
    }

    /// The highest open `T`, anywhere on the stack, to change.
    pub fn find_mut<T: Dialog>(&mut self) -> Option<&mut T> {
        self.stack.iter_mut().rev().find_map(|d| (d.as_mut() as &mut dyn Any).downcast_mut())
    }

    /// True if a `T` is open anywhere on the stack.
    pub fn has<T: Dialog>(&self) -> bool {
        self.find::<T>().is_some()
    }

    /// Closes every open `T`; true if there was one.
    pub fn remove<T: Dialog>(&mut self) -> bool {
        let before = self.stack.len();
        self.stack.retain(|d| !(d.as_ref() as &dyn Any).is::<T>());
        if self.stack.is_empty() {
            self.opened = None;
        }
        self.stack.len() != before
    }

    /// Sends a key to the topmost dialog. `None` when nothing is open (the key
    /// is the app's). Esc closes the top; `Close` and `Answer` close it too.
    pub fn key(&mut self, key: KeyEvent) -> Option<Response> {
        let top = self.stack.last_mut()?;
        let response = if key.code == KeyCode::Esc { Response::Close } else { top.key(key) };
        if response != Response::Keep {
            self.pop();
        }
        Some(response)
    }

    /// How dim the background is at `now` (0 = not at all, [`DIM`] = fully).
    pub fn dim_at(&self, now: Instant) -> f32 {
        let Some(opened) = self.opened else { return 0.0 };
        let t = now.saturating_duration_since(opened).as_secs_f32() / FADE.as_secs_f32();
        DIM * t.clamp(0.0, 1.0)
    }

    /// True while the fade is still moving: keep redrawing until then.
    pub fn animating(&self, now: Instant) -> bool {
        self.opened.is_some_and(|o| now.saturating_duration_since(o) < FADE)
    }

    /// Draws every dialog bottom first, with no dimming: for an app that already
    /// dimmed the screen (e.g. overlays that share [`dim`]).
    pub fn draw_dialogs(&self, frame: &mut Frame, palette: &Palette) {
        let area = frame.area();
        for dialog in &self.stack {
            dialog.draw(frame, area, palette);
        }
    }

    /// Dims the screen, then draws every dialog bottom first (each lower one
    /// dimmed again under the next).
    pub fn draw(&self, frame: &mut Frame, palette: &Palette, now: Instant) {
        let area = frame.area();
        let amount = self.dim_at(now);
        for dialog in &self.stack {
            dim(frame.buffer_mut(), area, amount, palette.background);
            dialog.draw(frame, area, palette);
        }
    }
}

/// Fades every cell in `area` towards `toward` by `amount` (0–1). RGB colours
/// are desaturated then blended; others get the DIM modifier once `amount`
/// passes a third. Apps that dim their own overlays (before
/// [`DialogStack::draw_dialogs`]) should call this with the same `toward`.
pub fn dim(buf: &mut Buffer, area: Rect, amount: f32, toward: Color) {
    if amount <= 0.0 {
        return;
    }
    let area = area.intersection(buf.area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            let (fg, bg) = (blend(cell.fg, toward, amount), blend(cell.bg, toward, amount));
            match (fg, bg) {
                (Some(fg), Some(bg)) => {
                    cell.fg = fg;
                    cell.bg = bg;
                }
                _ if amount > 0.33 => {
                    cell.modifier.insert(Modifier::DIM);
                }
                _ => {}
            }
        }
    }
}

/// `from` desaturated then moved `amount` of the way to `to`; `None` unless
/// `to` is RGB (`Reset` stays put so a default background is unchanged).
fn blend(from: Color, to: Color, amount: f32) -> Option<Color> {
    let Color::Rgb(tr, tg, tb) = to else { return None };
    if from == to {
        return Some(from);
    }
    let (tr, tg, tb) = (f32::from(tr), f32::from(tg), f32::from(tb));
    let (fr, fg, fb) = match from {
        Color::Rgb(r, g, b) => (f32::from(r), f32::from(g), f32::from(b)),
        Color::Reset => return Some(Color::Reset),
        _ => return None,
    };
    let grey = 0.299 * fr + 0.587 * fg + 0.114 * fb;
    let sat = 1.0 - amount * (1.0 - DIM_SATURATION);
    let brightness = 1.0 - amount;
    let fade = |c: f32, base: f32| {
        let c = grey + (c - grey) * sat;
        (base + (c - base) * brightness).round().clamp(0.0, 255.0) as u8
    };
    Some(Color::Rgb(fade(fr, tr), fade(fg, tg), fade(fb, tb)))
}

/// A rectangle `width` × `height` centred in `area` (clamped to it).
pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::theme::TOKYO_NIGHT;
    use ratatui::{Terminal, backend::TestBackend};

    pub(crate) fn render(cols: u16, rows: u16, f: impl FnOnce(&mut Frame)) -> (String, Buffer) {
        let mut term = Terminal::new(TestBackend::new(cols, rows)).unwrap();
        term.draw(f).unwrap();
        let buf = term.backend().buffer().clone();
        let text = (0..buf.area.height)
            .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect::<String>().trim_end().to_owned())
            .collect::<Vec<_>>()
            .join("\n");
        (text, buf)
    }

    pub(crate) fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    #[test]
    fn esc_closes_only_the_topmost_dialog() {
        let now = Instant::now();
        let mut stack = DialogStack::new();
        assert_eq!(stack.key(key(KeyCode::Esc)), None, "nothing open: the key is the app's");
        stack.push(Help::new(vec![]), now);
        stack.push(Confirm::new("quit", "Quit?"), now);
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.key(key(KeyCode::Esc)), Some(Response::Close));
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.key(key(KeyCode::Char('j'))), Some(Response::Keep));
        assert!(stack.is_open());
        assert_eq!(stack.key(key(KeyCode::Esc)), Some(Response::Close));
        assert!(!stack.is_open());
    }

    #[test]
    fn an_answer_closes_the_dialog_and_names_it() {
        let mut stack = DialogStack::new();
        stack.push(Confirm::new("quit", "Quit?"), Instant::now());
        assert_eq!(stack.key(key(KeyCode::Char('y'))), Some(Response::Answer { id: "quit".into(), yes: true }));
        assert!(stack.is_empty());
    }

    #[test]
    fn the_background_fades_in_then_holds() {
        let t0 = Instant::now();
        let mut stack = DialogStack::new();
        assert_eq!(stack.dim_at(t0), 0.0);
        stack.push(Help::new(vec![]), t0);
        assert_eq!(stack.dim_at(t0), 0.0);
        assert!(stack.animating(t0 + FADE / 2));
        let half = stack.dim_at(t0 + FADE / 2);
        assert!(half > 0.0 && half < DIM, "{half}");
        assert_eq!(stack.dim_at(t0 + FADE * 3), DIM);
        assert!(!stack.animating(t0 + FADE * 3));
        stack.pop();
        assert_eq!(stack.dim_at(t0 + FADE * 3), 0.0);
    }

    #[test]
    fn dim_blends_rgb_and_marks_the_rest() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        buf[(0, 0)].set_fg(Color::Rgb(200, 200, 200)).set_bg(Color::Reset);
        buf[(1, 0)].set_fg(Color::Red);
        let area = buf.area;
        dim(&mut buf, area, 0.5, Color::Rgb(0, 0, 0));
        // Grey stays grey; amount 0.5 keeps half the brightness toward black.
        assert_eq!(buf[(0, 0)].fg, Color::Rgb(100, 100, 100));
        assert_eq!(buf[(0, 0)].bg, Color::Reset);
        assert!(buf[(1, 0)].modifier.contains(Modifier::DIM));
    }

    #[test]
    fn dialogs_are_found_again_by_type() {
        let mut stack = DialogStack::new();
        let now = Instant::now();
        stack.push(Help::new(vec![]), now);
        stack.push(Confirm::quit("x"), now);
        assert!(stack.top::<Confirm>().is_some() && stack.top::<Help>().is_none());
        assert!(stack.has::<Help>());
        stack.find_mut::<Help>().unwrap().scroll = 3;
        assert_eq!(stack.find::<Help>().unwrap().scroll, 3);
        assert!(stack.remove::<Confirm>() && !stack.remove::<Confirm>());
        assert_eq!(stack.len(), 1);
        assert!(stack.remove::<Help>() && !stack.is_open() && stack.dim_at(now) == 0.0);
    }

    #[test]
    fn drawing_dims_the_screen_behind_the_dialog() {
        let p = TOKYO_NIGHT;
        let mut stack = DialogStack::new();
        let t0 = Instant::now();
        stack.push(Confirm::new("quit", "Quit boxvibe?"), t0);
        let (text, buf) = render(60, 20, |f| {
            f.buffer_mut()[(0, 0)].set_symbol("X").set_fg(p.fg).set_bg(p.background);
            stack.draw(f, &p, t0 + FADE);
        });
        assert!(text.contains("Quit boxvibe?"), "{text}");
        assert_ne!(buf[(0, 0)].fg, p.fg, "the corner is dimmed");
        assert_eq!(buf[(0, 0)].symbol(), "X", "dimmed, not erased");
    }
}
