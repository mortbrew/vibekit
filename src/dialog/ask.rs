//! A [`Confirm`] that carries typed context for the app.

use super::{Confirm, Dialog, Response};
use crate::theme::Palette;
use ratatui::Frame;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;

/// A yes/no question that remembers what it is about (which pin, which theme, …).
pub struct Ask<T> {
    pub confirm: Confirm,
    pub about: T,
}

impl<T: 'static> Dialog for Ask<T> {
    fn key(&mut self, key: KeyEvent) -> Response {
        self.confirm.key(key)
    }

    fn draw(&self, frame: &mut Frame, screen: Rect, palette: &Palette) {
        self.confirm.draw(frame, screen, palette);
    }
}
