//! The Theme Select picker's state: two tabs, Search (first) filters the
//! built-in themes and every scheme GitHub lists as you type, fuzzy-ranked
//! like fzf, hiding the ones already saved; Installed lists the saved ones.
//! Keys and drawing stay in the app; this is the part every app shares.

use crate::theme;
use crate::theme_store::{Entry, built_in_entries};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

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
}
