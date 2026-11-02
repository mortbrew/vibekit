//! Text that fits fixed-width cells on any terminal: cleaning, worst-case
//! widths and cutting with "…".

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Private Use Area: Nerd Font icons, which foot (and others) may draw two
/// cells wide although they measure one.
fn is_pua(c: char) -> bool {
    matches!(c as u32, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD)
}

/// Pictographs and symbols that have emoji forms: terminals disagree on how
/// wide these are drawn (one cell or two), whatever unicode-width says.
fn is_pictographic(c: char) -> bool {
    matches!(
        c as u32,
        0x2300..=0x23FF
            | 0x24C2
            | 0x25A0..=0x25FF
            | 0x2600..=0x27BF
            | 0x2B00..=0x2BFF
            | 0x3030
            | 0x303D
            | 0x3297
            | 0x3299
            | 0x1F000..=0x1FAFF
    )
}

/// Characters that take no cell, or join or restyle their neighbours:
/// variation selectors, joiners, soft hyphen, BOM, bidi controls.
fn is_zero_width(c: char) -> bool {
    matches!(
        c as u32,
        0x00AD | 0x034F | 0x061C | 0x180E | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F
            | 0xFE00..=0xFE0F | 0xFEFF | 0xE0000..=0xE007F
    ) || (c != ' ' && UnicodeWidthChar::width(c) == Some(0))
}

/// Song and station text made safe for a fixed-width cell: tabs, newlines
/// and other controls become spaces; zero-width characters go; symbols whose
/// drawn width terminals disagree on (one-cell pictographs that foot may draw
/// as two-cell emoji, Nerd Font glyphs) go too, since a glyph drawn wider
/// than measured shifts the rest of the row, the rating included.
/// Two-cell emoji (🎵) stay: every terminal agrees on those.
pub fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_control() {
            out.push(' ');
        } else if !(is_zero_width(c) || is_pua(c) || is_pictographic(c) && UnicodeWidthChar::width(c) != Some(2)) {
            out.push(c);
        }
    }
    out
}

/// The cells `c` may take on any terminal: ambiguous-width, pictographic and
/// Private Use characters count as two so a cut never overflows.
pub fn safe_char_width(c: char) -> usize {
    if is_zero_width(c) || c.is_control() {
        return 0;
    }
    let w = UnicodeWidthChar::width(c).unwrap_or(0);
    let ambiguous = UnicodeWidthChar::width_cjk(c).unwrap_or(0) > w;
    if is_pua(c) || is_pictographic(c) || ambiguous || !c.is_ascii() && w == 0 { 2 } else { w }
}

/// The worst-case width of `text` on any terminal.
pub fn safe_width(text: &str) -> usize {
    text.chars().map(safe_char_width).sum()
}

/// Clean `text` and cut it to fit `cells` with one cell to spare, measuring
/// worst-case widths, ending in "…" when cut.
pub fn fit(text: &str, cells: usize) -> String {
    truncate(&clean(text), cells.saturating_sub(1))
}

/// `text` cut to `cells` cells (worst-case widths), ending in "…" when cut.
pub fn truncate(text: &str, cells: usize) -> String {
    if safe_width(text) <= cells {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let w = safe_char_width(c);
        if used + w + safe_char_width('…') > cells {
            break;
        }
        used += w;
        out.push(c);
    }
    if cells > 0 {
        out.push('…');
    }
    out
}

/// `text` padded with spaces to `cells` terminal cells (emoji take two).
pub fn pad(text: &str, cells: usize) -> String {
    let w = text.width();
    format!("{text}{}", " ".repeat(cells.saturating_sub(w)))
}

/// `text` word-wrapped to `width` cells, at most `max` lines; the last one
/// ends in "…" when there was more.
pub fn wrap(text: &str, width: usize, max: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines: Vec<String> = Vec::new();
    for word in clean(text).split_whitespace() {
        match lines.last_mut() {
            Some(line) if safe_width(line) + 1 + safe_width(word) <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(truncate(word, width)),
        }
    }
    if lines.len() > max {
        lines.truncate(max.max(1));
        let last = lines.pop().unwrap_or_default();
        lines.push(truncate(&format!("{last} …"), width));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_cuts_with_an_ellipsis_and_a_spare_cell() {
        assert_eq!(fit("hello", 10), "hello");
        assert_eq!(fit("hello world", 6), "hel…"); // "…" counts as two cells (ambiguous width)
        assert!(safe_width(&fit("ünïcödé ★ name", 8)) <= 7);
    }

    #[test]
    fn clean_drops_controls_and_zero_width() {
        assert_eq!(clean("a\tb\u{200B}c"), "a bc");
        assert_eq!(pad("ab", 4), "ab  ");
    }

    #[test]
    fn wrap_breaks_on_words_and_marks_the_cut() {
        assert_eq!(wrap("one two three", 7, 5), vec!["one two", "three"]);
        let cut = wrap("one two three four", 7, 1);
        assert_eq!(cut.len(), 1);
        assert!(cut[0].ends_with('…'), "{cut:?}");
    }
}
