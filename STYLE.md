# Shared conventions for vibe apps

Small terminal apps built on vibekit should feel the same.

## Global keys
These are the same in every vibe app.
- **`Q` (capital) quits, and it is the only quit key.** If quitting would lose something, it opens `Confirm::quit`, where `Q` again (or `y`) confirms.
- **Esc closes the topmost dialog** (on the main screen it backs out of the pane or search).
- **`?` opens help, and `?` closes it** (Esc closes it too).
- **Lowercase `q` is unbound globally.** It never quits or closes anything, so a stray keypress is harmless. Panes may use it only for a pane-local action, and preferably not at all.

## Keys
- **Shift+letter is global.** It works from any pane (e.g. `T` opens Theme Select).
- **Lowercase acts on the focused pane.** The same letter can mean different things in different panes.
- See Global keys above for `Q`, Esc and `?`.
- The key bar shows the keys for the focused pane. Change it whenever you change a key.

## Help screen
- One screen, grouped as Global, then one section per pane. It must list every key.
- It is generated from the same table as the key bar, so they can't drift apart.

## About / tip panel
- Shows the name, version, repo link, license, and an optional tip link.
- Shows "Update available" from `vibekit::update` when a newer release exists.

## Dialogs, help and toasts
- Dialogs open on a `vibekit::dialog::DialogStack`, which dims the screen behind them (a 150 ms fade). While one is open it gets every key.
- **Esc closes the topmost dialog.** `?` opens help from anywhere (help also closes with `?`; lowercase `q` does not).
- Yes/no questions use `Confirm`. `y`/`n` answer at once, and Enter presses the chosen button, which starts on No. Quitting asks only when something would be lost (a download, unsaved changes).
- Toasts (`vibekit::toast`) are for things that happened, not questions. They are short, timed and stacked bottom-right, and they never take keys.

## Settings menu
This is a style rule; each app builds its own screen.
- One vertical list. Group rows under plain headings (Look, Playback, Updates, About), ordered by how often people change them.
- Each row has the label on the left in Settings wording (that wording is also the config key) and the value right-aligned. The selected row is reversed.
- **Toggle rows** (`settings::toggle_row`) show On/Off. Space, Enter, ←/→ or h/l flip them, and the change applies and saves at once, with no Save button.
- **Picker rows** (`settings::picker_row`) show `Value ›`. Enter, → or l opens a picker popup, and Esc returns to the list. Theme Select is the standard one (`settings::ThemeRow` plus `picker`).
- j/k or ↑↓ move, g/G jump to the ends, and Esc or Tab leaves the list. The key bar shows what the selected row's keys do.
- A setting that is unavailable stays in the list, greyed out, with a short reason as its value (e.g. "follows Omarchy").
- Rows never wrap. The label gives way with "…" and the value always stays whole.

## Config
- TOML in `~/.config/<app>/config.toml`.
- **Config keys match the Settings wording**: a setting called "Show album art" is `show_album_art`.
- **Tolerate unknown keys.** Skip them with a one-line warning and never fail to start. Keep them on save if you can.
- Bad values fall back to the default with a warning.

## Themes
- Use `vibekit::theme`: built-ins, user themes in `~/.config/<app>/themes`, and Omarchy sync.
- Theme Select uses `vibekit::picker`: `key` returns an `Action` for the app, `draw` renders inside the app's block, and the app draws the delete question.

## Releases
- Tarball `<name>-<os>-<arch>.tar.gz` containing `<name>/<name>`.
- Set `<NAME>_FAKE_VERSION=0.0.1` to test the update flow.

## Changes
- Every user-facing change updates Help, the README, the key bar and the CHANGELOG.
- Tests use demo data or a fake server, never a live service.
