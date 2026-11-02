# Changelog

## 0.2.0 (unreleased)

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
