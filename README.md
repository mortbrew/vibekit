# vibekit

Shared pieces for small ratatui apps (boxvibe, ...):

- `theme`: the theme engine, user themes and Omarchy sync
- `theme_store`: the online theme catalog (fetch, save, remove)
- `picker`: Theme Select: state, fuzzy filter, a key handler returning `Action`s, and `draw`
- `text`: fitting text into terminal cells
- `update`: GitHub self-update check and install via `Release { name, repo, version }`

See STYLE.md for conventions. Local only; not published. MIT licensed (see LICENSE-MIT).
