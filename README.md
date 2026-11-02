# vibekit

Shared pieces for small ratatui apps (boxvibe, ...):

- `theme`: the theme engine, user themes and Omarchy sync
- `theme_store`: the online theme catalog (fetch, save, remove)
- `picker`: Theme Select: state, fuzzy filter, a key handler returning `Action`s, and `draw`
- `dialog`: a modal stack over a dimmed screen, `Confirm` (yes/no, quit) and `Help` (`?`)
- `toast`: timed, stacked notes
- `settings`: toggle and picker rows (Theme Select row)
- `text`: fitting and wrapping text into terminal cells
- `update`: GitHub self-update check and install via `Release { name, repo, version }`

## Dialogs

```rust
use vibekit::dialog::{Confirm, DialogStack, Help, Response, Section};

let mut dialogs = DialogStack::new();
dialogs.push(Confirm::quit("boxvibe"), Instant::now()); // Q again / y quits, n / Esc stays
dialogs.push(Help::new(vec![Section::new("Global", &[("Q", "quit"), ("?", "help")])]), now);

// In the key handler: an open dialog gets every key.
if let Some(r) = dialogs.key(key) {
    if let Response::Answer { id, yes } = r { /* id == "quit", ... */ }
    return;
}
// In draw, after the screen: dims behind the top dialog (150 ms fade) and draws it.
dialogs.draw(frame, &palette, now);
```

- `Response`: `Keep`, `Close`, or `Answer { id, yes }`. Esc closes the top dialog; any other response pops it.
- `Confirm::new(id, question)` with `.title(..)` and `.labels(yes, no)`. `y`/`n` answer, Enter presses the chosen
  button (it starts on No), ←/→/Tab/h/l move between them, and other keys do nothing.
  `Confirm::quit(app)` also takes `Q` as yes.
- `Help::new(sections)` scrolls (j/k, PgUp/PgDn, g/G) and closes with `?` or Esc only.
- Your own dialog: `impl Dialog` (`key` and `draw`) and push it.
- `dim_at(now)` and `animating(now)` tell you whether to redraw during the fade.

## Toasts

```rust
let mut toasts = vibekit::toast::Toasts::new();
toasts.push("Saved", Kind::Info, now);          // shown for SHOW_FOR (4 s); push_for picks the time
if toasts.tick(now) { /* one expired: redraw */ }
toasts.draw(frame, area, &palette, now);         // at most MAX (3), bottom-right, never takes keys
```

## Settings rows

- `toggle_row(label, on, selected, width, &palette)` draws "label … On/Off". Space, Enter, ←/→ or h/l flip it (`toggles(key)`).
- `picker_row(label, value, selected, width, &palette)` draws "label … value ›". Enter, → or l opens it (`opens(key)`).
- `ThemeRow::line(current, ..)` and `ThemeRow::key(key)` are the Theme row; the key opens Theme Select (`ThemePicker`).

See STYLE.md for conventions (global keys, dialogs, settings) and CHANGELOG.md for changes.
Local only; not published. MIT licensed (see LICENSE-MIT).

### Finding a dialog again

`Dialog` is `Any`, so `stack.top::<T>()`, `find::<T>()` / `find_mut::<T>()`, `has::<T>()` and `remove::<T>()` reach an open dialog by type (e.g. to update a list it shows). `stack.draw_dialogs(frame, &palette)` draws without dimming the app behind.
