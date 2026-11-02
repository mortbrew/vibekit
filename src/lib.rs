//! vibekit: the parts the *vibe terminal apps share, kept small and generic.
//!
//! - [`theme`]: palettes, the built-in themes, Omarchy detection and live sync.
//! - [`theme_store`]: the iTerm2-Color-Schemes / Omarchy catalog from GitHub,
//!   saving and removing downloaded themes.
//! - [`picker`]: Theme Select: state, fuzzy ranking, keys and drawing.
//! - [`text`]: fitting text into terminal cells.
//! - [`dialog`]: modal dialogs on a dimmed screen (stack, confirm, `?` help).
//! - [`toast`]: timed notes stacked in a corner.
//! - [`settings`]: toggle and picker rows for a Settings list.
//! - [`update`]: the GitHub release check and in-place self-update.
//!
//! See `STYLE.md` for the conventions the apps follow.

pub mod dialog;
pub mod picker;
pub mod settings;
pub mod text;
pub mod theme;
pub mod theme_store;
pub mod toast;
pub mod update;

/// A fresh, empty folder for one test.
#[cfg(test)]
pub(crate) fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("vibekit-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
