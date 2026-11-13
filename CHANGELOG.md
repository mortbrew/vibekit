# Changelog

## 0.3.0

### Added
- `input::TextInput`: shared line editor (cursor, mask/window, editing keys).
- `dialog::Ask<T>`: `Confirm` plus typed payload for the app.
- `fs::write_private`: atomic 0600/0700 tmp+rename writes for config and similar.
- `desktop::Entry`: parameterized Linux `.desktop` install / remove / Exec quoting.
- `proc::detached`: zombie-safe background spawn.
- `time::{iso_utc, today}`: UTC ISO-8601 stamps without chrono.
- `theme_store::CatalogSources`: choose iTerm and/or Omarchy catalogs (`ITERM`, `BOTH`).
- `dialog::DIM_SATURATION`: desaturating dim shared by dialog fade and app overlays.
- `picker::Action::AskDelete`: Theme Select asks the app to confirm deletes on DialogStack.

### Changed
- `theme_store::fetch_catalog(demo, sources)` takes [`CatalogSources`] (breaking).
- Theme Select no longer owns an inline delete confirm (`confirm_delete` / `Action::Delete` removed).
- Theme Select loading copy is neutral ("Fetching themes…").
- `dialog::dim` desaturates RGB before blending toward the background.
- Doc wording no longer assumes album/song/station or a single consumer app.

## Unreleased (folded into 0.3.0)

- STYLE: the shortcuts common core is now exactly the approved shared list (Q, Esc, ?, Tab/Shift+Tab, j/k ↑↓, PgUp/PgDn, Enter, /, Shift+O/Shift+A, ., Shift = global / lowercase = pane). `g`/`G` and `Ctrl+c` quit-at-once leave the core and are app-specific; all other shortcuts are app-specific.

## 0.2.0

- `Dialog: Any`: `DialogStack::top`, `top_mut`, `find`, `find_mut`, `has` and `remove` reach a dialog by its type; `draw_dialogs` draws the stack without the dim, for apps that dim themselves.
- Help is as wide as its longest line (at least 72, at most the screen) and keeps icons in its keys.
### Added
- `dialog`: `DialogStack` (a modal stack that dims the screen behind it with a 150 ms fade), the `Dialog` trait and `Response`.
- `dialog::Confirm`: a yes/no question with `y`/`n`, Enter on the chosen button (it starts on No) and Esc.
  `Confirm::quit(app)` also confirms with a second `Q`.
- `dialog::Help`: a scrolling help overlay built from `Section`s. It closes with `?` or Esc only.
- `toast`: timed, stacked notes (`Toasts`, `Kind`).
- `settings`: `toggle_row`, `picker_row` and `ThemeRow` for Settings menus.
- `text::wrap`.
- STYLE.md: Global keys (the shared shortcuts core and open questions), dialogs/help/toasts, Settings menu.

### Changed
- Lowercase `q` is unbound: it never quits or closes anything. `Q` is the only quit key.

## 0.1.0
- First release: `theme`, `theme_store`, `picker` (Theme Select), `text` and `update`, extracted from boxvibe.
