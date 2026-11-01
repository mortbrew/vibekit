//! The Theme Select picker's state: two tabs, Search (first) filters the
//! built-in themes and every scheme GitHub lists as you type, fuzzy-ranked
//! like fzf, hiding the ones already saved; Installed lists the saved ones.
//! [`ThemePicker::key`] handles the keys and returns an [`Action`] for the
//! app to carry out; [`draw`] draws it inside a block the app styles.

use crate::text::{clean, fit, pad, safe_width};
use crate::theme::{self, Palette};
use crate::theme_store::{Entry, built_in_entries};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

/// The picker's tabs (Tab / Shift+Tab switch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeTab {
    Search,
    Installed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemePicker {
    pub tab: ThemeTab,
    /// What is typed in Search.
    pub query: String,
    /// The built-in themes, then every scheme GitHub listed.
    pub catalog: Vec<Entry>,
    /// `catalog` indexes that match `query`, best first.
    pub hits: Vec<usize>,
    pub search_cursor: usize,
    pub installed_cursor: usize,
    /// Waiting for the list from GitHub.
    pub loading: bool,
    /// Why there is no list.
    pub error: Option<String>,
    /// The scheme being downloaded.
    pub downloading: Option<String>,
    /// The saved theme `x` asked to delete.
    pub confirm_delete: Option<String>,
}

impl ThemePicker {
    /// A picker on Search with only the built-in themes until GitHub answers.
    pub fn new() -> Self {
        Self {
            tab: ThemeTab::Search,
            query: String::new(),
            catalog: built_in_entries(),
            hits: Vec::new(),
            search_cursor: 0,
            installed_cursor: 0,
            loading: true,
            error: None,
            downloading: None,
            confirm_delete: None,
        }
    }

    /// Ranks the catalog against the query (fzf-style fuzzy match on the
    /// name and source), leaving out the saved themes; an empty query lists
    /// everything by name.
    pub fn refilter(&mut self, saved: &[(String, theme::Palette)]) {
        self.hits = rank(&self.catalog, &self.query)
            .into_iter()
            .filter(|i| !saved.iter().any(|(n, _)| *n == self.catalog[*i].slug()))
            .collect();
        self.search_cursor = 0;
    }

    /// Like [`Self::refilter`], keeping the cursor where it was (clamped).
    pub fn refilter_keep(&mut self, saved: &[(String, theme::Palette)]) {
        let at = self.search_cursor;
        self.refilter(saved);
        self.search_cursor = at.min(self.hits.len().saturating_sub(1));
    }
}

/// `catalog` indexes matching `query`, best match first (ties by name).
pub fn rank(catalog: &[Entry], query: &str) -> Vec<usize> {
    if query.trim().is_empty() {
        let mut all: Vec<usize> = (0..catalog.len()).collect();
        all.sort_by_cached_key(|i| catalog[*i].name.to_lowercase());
        return all;
    }
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
    let mut buf = Vec::new();
    let mut scored: Vec<(u32, usize)> = catalog
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let hay = format!("{} {}", e.name, e.source.label());
            pattern.score(Utf32Str::new(&hay, &mut buf), &mut matcher).map(|s| (s, i))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| catalog[a.1].name.cmp(&catalog[b.1].name)));
    scored.into_iter().map(|(_, i)| i).collect()
}

impl Default for ThemePicker {
    fn default() -> Self {
        Self::new()
    }
}

/// What a key in the picker asks the app to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Nothing for the app (the picker changed itself, or the key was ignored).
    None,
    /// Esc: close the picker.
    Close,
    /// Use the theme with this slug (built-in or already saved).
    Use(String),
    /// Download, save and use this one; `downloading` is already set.
    Download(Entry),
    /// Enter on a theme while another is still downloading.
    Busy,
    /// Delete the saved theme with this slug (the user said yes).
    Delete(String),
}

/// One step through `len` rows: ±1 wraps around, bigger steps stop at the ends.
pub fn step(at: usize, len: usize, by: i32) -> usize {
    if len == 0 {
        return 0;
    }
    let at = at.min(len - 1) as i32;
    let len = len as i32;
    if by.abs() == 1 { (at + by).rem_euclid(len) as usize } else { (at + by).clamp(0, len - 1) as usize }
}

/// Typing hints under the picker, one per tab.
pub const SEARCH_HINT: &str = "type search · ↑↓ move · Enter use · Tab Installed · Esc close";
pub const INSTALLED_HINT: &str = "↑↓ move · Enter use · x delete · Tab Search · Esc close";

impl ThemePicker {
    /// Handles one key. `saved` are the user's themes (slug, palette);
    /// `installed` is the Installed tab's rows, in order.
    pub fn key(&mut self, key: KeyEvent, saved: &[(String, Palette)], installed: &[String]) -> Action {
        if let Some(name) = self.confirm_delete.clone() {
            return match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.confirm_delete = None;
                    Action::Delete(name)
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.confirm_delete = None;
                    Action::None
                }
                _ => Action::None,
            };
        }
        match key.code {
            KeyCode::Esc => return Action::Close,
            KeyCode::Tab | KeyCode::BackTab => {
                self.tab = match self.tab {
                    ThemeTab::Search => ThemeTab::Installed,
                    ThemeTab::Installed => ThemeTab::Search,
                };
                return Action::None;
            }
            _ => {}
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match self.tab {
            ThemeTab::Search => {
                let len = self.hits.len();
                match key.code {
                    KeyCode::Char('u') if ctrl => {
                        self.query.clear();
                        self.refilter(saved);
                    }
                    KeyCode::Char(c) if !ctrl => {
                        self.query.push(c);
                        self.refilter(saved);
                    }
                    KeyCode::Backspace => {
                        self.query.pop();
                        self.refilter(saved);
                    }
                    KeyCode::Down => self.search_cursor = step(self.search_cursor, len, 1),
                    KeyCode::Up => self.search_cursor = step(self.search_cursor, len, -1),
                    KeyCode::PageDown => self.search_cursor = (self.search_cursor + 10).min(len.saturating_sub(1)),
                    KeyCode::PageUp => self.search_cursor = self.search_cursor.saturating_sub(10),
                    KeyCode::Enter => {
                        let Some(entry) = self.hits.get(self.search_cursor).map(|i| self.catalog[*i].clone()) else {
                            return Action::None;
                        };
                        let slug = entry.slug();
                        if entry.source == crate::theme_store::Source::BuiltIn || saved.iter().any(|(n, _)| *n == slug)
                        {
                            return Action::Use(slug);
                        }
                        if self.downloading.is_some() {
                            return Action::Busy;
                        }
                        self.downloading = Some(entry.name.clone());
                        return Action::Download(entry);
                    }
                    _ => {}
                }
            }
            ThemeTab::Installed => {
                let len = installed.len();
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => self.installed_cursor = step(self.installed_cursor, len, 1),
                    KeyCode::Char('k') | KeyCode::Up => self.installed_cursor = step(self.installed_cursor, len, -1),
                    KeyCode::Home | KeyCode::Char('g') => self.installed_cursor = 0,
                    KeyCode::End | KeyCode::Char('G') => self.installed_cursor = len.saturating_sub(1),
                    KeyCode::Enter => {
                        if let Some(name) = installed.get(self.installed_cursor) {
                            return Action::Use(name.clone());
                        }
                    }
                    KeyCode::Char('x') | KeyCode::Delete => {
                        if let Some(name) = installed.get(self.installed_cursor) {
                            self.confirm_delete = Some(name.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        Action::None
    }

    /// The key bar's caps for the picker (key, what it does).
    pub fn key_caps(&self) -> Vec<(&'static str, &'static str)> {
        if self.confirm_delete.is_some() {
            return vec![("y", "Yes, Delete"), ("n Esc", "No")];
        }
        match self.tab {
            ThemeTab::Search => vec![
                ("Type", "Fuzzy Search"),
                ("↑↓", "Move"),
                ("Enter", "Download & Use"),
                ("Tab", "Installed"),
                ("Esc", "Close"),
            ],
            ThemeTab::Installed => {
                vec![("j/k ↑↓", "Move"), ("Enter", "Use"), ("x Del", "Delete"), ("Tab", "Search"), ("Esc", "Close")]
            }
        }
    }
}

/// What the picker needs from the app to draw.
pub struct View<'a> {
    pub palette: &'a Palette,
    /// The Installed tab's rows (saved theme slugs), in order.
    pub installed: &'a [String],
    /// The slug of the theme in use (tagged "in use").
    pub current: &'a str,
    /// The popup's frame, styled by the app (borders, title, background).
    pub block: Block<'a>,
}

/// Draws the picker centred on `screen`; returns where it went. A pending
/// delete question is left to the app (`picker.confirm_delete`), drawn on top.
pub fn draw(frame: &mut Frame, screen: Rect, picker: &ThemePicker, view: View) -> Rect {
    let p = view.palette;
    let installed = view.installed;
    let width = 76.min(screen.width.saturating_sub(2)).max(screen.width.min(24));
    let inner_w = usize::from(width.saturating_sub(2));
    let rows_room = usize::from(screen.height.saturating_sub(14)).clamp(3, 14);
    let selected = Style::new().fg(p.background).bg(p.fg).add_modifier(Modifier::BOLD);
    let tab = |label: String, on: bool| {
        let style = if on { selected } else { Style::new().fg(p.muted) };
        Span::styled(format!(" {label} "), style)
    };
    let search_label = if picker.loading { "Search".to_owned() } else { format!("Search ({})", picker.hits.len()) };
    let tabs = vec![
        Span::raw(" "),
        tab(search_label, picker.tab == ThemeTab::Search),
        Span::raw("  "),
        tab(format!("Installed ({})", installed.len()), picker.tab == ThemeTab::Installed),
    ];
    let mut lines = vec![Line::from(""), Line::from(tabs).left_aligned(), Line::from("")];
    let row = |text: String, on: bool| {
        let style = if on { selected } else { Style::new().fg(p.fg) };
        Line::from(Span::styled(pad(&fit(&text, inner_w), inner_w), style)).left_aligned()
    };
    let note = |text: String| {
        Line::from(Span::styled(format!(" {}", fit(&text, inner_w.saturating_sub(1))), Style::new().fg(p.muted)))
            .left_aligned()
    };
    let window = |len: usize, cursor: usize, room: usize| {
        let at = cursor.min(len.saturating_sub(1));
        let start = at.saturating_sub(room - 1).min(len.saturating_sub(room));
        (start, at)
    };
    // Right-aligned tag after a name ("in use", "downloading…"), never cut:
    // the name gives way, and a margin keeps the tag off the border.
    let tagged = |name: &str, tag: &str| {
        if tag.is_empty() {
            return format!(" {}", fit(name, inner_w.saturating_sub(2)));
        }
        let room = inner_w.saturating_sub(safe_width(tag) + 4);
        let name = fit(name, room);
        format!(" {name}{} {tag} ", " ".repeat(room.saturating_sub(safe_width(&name))))
    };
    let mut footer = String::new();
    let hint;
    if picker.tab == ThemeTab::Search {
        hint = SEARCH_HINT;
        lines.push(
            Line::from(vec![
                Span::styled(" › ", Style::new().fg(p.accent).bold()),
                Span::styled(fit(&picker.query, inner_w.saturating_sub(5)), Style::new().fg(p.fg)),
                Span::styled("▏", Style::new().fg(p.accent)),
            ])
            .left_aligned(),
        );
        let mut room = rows_room - 1;
        // The built-in themes list at once; GitHub's join them when they come.
        if picker.loading {
            lines.push(note("Asking GitHub for iTerm2-Color-Schemes…".into()));
            room = room.saturating_sub(1).max(1);
        } else if let Some(e) = &picker.error {
            lines.push(note(format!("No theme list: {}", clean(e))));
            room = room.saturating_sub(1).max(1);
        }
        if picker.hits.is_empty() {
            if !picker.loading && picker.error.is_none() {
                lines.push(note("No theme matches".into()));
            }
        } else {
            let (start, at) = window(picker.hits.len(), picker.search_cursor, room);
            for (i, idx) in picker.hits.iter().enumerate().skip(start).take(room) {
                let entry = &picker.catalog[*idx];
                let tag = if entry.slug() == view.current {
                    "in use"
                } else if picker.downloading.as_deref() == Some(entry.name.as_str()) {
                    "downloading…"
                } else {
                    ""
                };
                let name = format!("{} · {}", clean(&entry.name), entry.source.label());
                lines.push(row(tagged(&name, tag), i == at));
            }
            footer = format!("{} of {}", at + 1, picker.hits.len());
        }
    } else {
        hint = INSTALLED_HINT;
        lines.push(note("The themes you downloaded (built-in ones are in Search)".into()));
        let room = rows_room - 1;
        if installed.is_empty() {
            lines.push(note("None yet: Tab to Search and Enter on one".into()));
        } else {
            let (start, at) = window(installed.len(), picker.installed_cursor, room);
            for (i, name) in installed.iter().enumerate().skip(start).take(room) {
                let tag = if name == view.current { "in use" } else { "" };
                lines.push(row(tagged(&theme::label(name), tag), i == at));
            }
            footer = format!("{} of {}", at + 1, installed.len());
        }
    }
    let used = lines.len() - 3;
    for _ in used..rows_room.max(1) {
        lines.push(Line::from(""));
    }
    lines.push(note(footer));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(hint, Style::new().fg(p.muted))));
    lines.push(Line::from(""));
    let h = (lines.len() as u16 + 2).min(screen.height);
    let w = width.min(screen.width);
    let area = Rect::new(screen.x + (screen.width - w) / 2, screen.y + (screen.height - h) / 2, w, h);
    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center).block(view.block), area);
    area
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_puts_the_closest_name_first_and_lists_all_when_empty() {
        let catalog = built_in_entries();
        assert_eq!(rank(&catalog, "").len(), catalog.len());
        let hits = rank(&catalog, "gruv");
        assert!(catalog[hits[0]].name.to_lowercase().contains("gruvbox"), "{:?}", catalog[hits[0]]);
    }

    #[test]
    fn saved_themes_are_left_out_of_search() {
        let mut picker = ThemePicker::new();
        picker.refilter(&[]);
        let all = picker.hits.len();
        let slug = picker.catalog[picker.hits[0]].slug();
        picker.refilter(&[(slug, theme::TOKYO_NIGHT)]);
        assert_eq!(picker.hits.len(), all - 1);
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    fn loaded() -> ThemePicker {
        let mut picker = ThemePicker::new();
        picker.loading = false;
        picker.refilter(&[]);
        picker
    }

    #[test]
    fn typing_filters_and_enter_uses_a_built_in() {
        let mut picker = loaded();
        for c in "gruv".chars() {
            assert_eq!(picker.key(key(KeyCode::Char(c)), &[], &[]), Action::None);
        }
        assert_eq!(picker.query, "gruv");
        match picker.key(key(KeyCode::Enter), &[], &[]) {
            Action::Use(slug) => assert!(slug.contains("gruvbox"), "{slug}"),
            other => panic!("{other:?}"),
        }
        picker.key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL), &[], &[]);
        assert_eq!(picker.query, "");
    }

    #[test]
    fn enter_downloads_one_at_a_time() {
        let mut picker = ThemePicker::new();
        picker.loading = false;
        picker.catalog = crate::theme_store::demo_catalog();
        picker.refilter(&[]);
        let at = picker.hits.iter().position(|i| picker.catalog[*i].source != crate::theme_store::Source::BuiltIn);
        picker.search_cursor = at.expect("demo has a downloadable theme");
        let Action::Download(entry) = picker.key(key(KeyCode::Enter), &[], &[]) else { panic!("no download") };
        assert_eq!(picker.downloading.as_deref(), Some(entry.name.as_str()));
        assert_eq!(picker.key(key(KeyCode::Enter), &[], &[]), Action::Busy);
    }

    #[test]
    fn installed_tab_moves_uses_and_deletes_after_asking() {
        let mut picker = loaded();
        let installed = vec!["a".to_owned(), "b".to_owned()];
        picker.key(key(KeyCode::Tab), &[], &installed);
        assert_eq!(picker.tab, ThemeTab::Installed);
        picker.key(key(KeyCode::Char('j')), &[], &installed);
        assert_eq!(picker.key(key(KeyCode::Enter), &[], &installed), Action::Use("b".into()));
        picker.key(key(KeyCode::Char('x')), &[], &installed);
        assert_eq!(picker.confirm_delete.as_deref(), Some("b"));
        assert_eq!(picker.key_caps()[0], ("y", "Yes, Delete"));
        assert_eq!(picker.key(key(KeyCode::Char('n')), &[], &installed), Action::None);
        picker.key(key(KeyCode::Delete), &[], &installed);
        assert_eq!(picker.key(key(KeyCode::Char('y')), &[], &installed), Action::Delete("b".into()));
        assert_eq!(picker.key(key(KeyCode::Esc), &[], &installed), Action::Close);
    }

    #[test]
    fn step_wraps_by_one_and_stops_at_the_ends() {
        assert_eq!(step(0, 3, -1), 2);
        assert_eq!(step(2, 3, 1), 0);
        assert_eq!(step(1, 3, 10), 2);
        assert_eq!(step(0, 0, 1), 0);
    }

    #[test]
    fn draw_never_cuts_the_hint_or_the_in_use_tag() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut picker = loaded();
        let installed = vec!["gruvbox".to_owned()];
        for cols in [40u16, 80, 160] {
            for tab in [ThemeTab::Search, ThemeTab::Installed] {
                picker.tab = tab;
                let mut term = Terminal::new(TestBackend::new(cols, 32)).unwrap();
                term.draw(|f| {
                    let view = View {
                        palette: &theme::TOKYO_NIGHT,
                        installed: &installed,
                        current: "tokyo-night",
                        block: Block::bordered(),
                    };
                    draw(f, f.area(), &picker, view);
                })
                .unwrap();
                let buf = term.backend().buffer();
                let all: String = (0..buf.area.height)
                    .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect::<String>() + "\n")
                    .collect();
                let hint = if tab == ThemeTab::Search { SEARCH_HINT } else { INSTALLED_HINT };
                if cols >= 80 {
                    assert!(all.contains(hint), "{cols}: hint cut\n{all}");
                }
                if tab == ThemeTab::Search {
                    assert!(all.contains("in use "), "{cols}: tag cut\n{all}");
                }
            }
        }
    }
}
