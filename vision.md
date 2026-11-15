# Vision
## Purpose
Shared pieces for small ratatui apps (boxvibe and others): theming, Theme Select, dialogs, toasts, settings rows, text layout, and GitHub self-update.
## In scope
The existing modules — `theme`, `theme_store`, `picker`, `dialog` (Confirm/Help), `toast`, `settings`, `text`, and `update` — as described in the README, for local in-tree use (not published).
## Out of scope
Publishing the crate; app-specific product features that belong in consumers, not the kit.
## Design principles and non-negotiables
Vibe apps share vibekit theming and style. Capital `Q` is the only quit key; lowercase `q` never quits. Shared keys are `Q`, Esc and `?`. `Confirm::quit` treats `Q` as yes; Help closes with `?` or Esc. MIT licensed. The repo is public and auditable. Git is `dev`-only, one `dev`→`main` PR at a time, and no push or merge to `main` without Toby's OK.
## Bots may decide alone
Compatible bugfixes and docs inside existing modules that do not change shared key behavior or public API behavior; relevant tests.
## Captain decision hold (needs Toby)
Behavior changes, new dependencies, and anything security- or data-related, including self-update. Shared-key or quit/help behavior, publishing the crate, new modules apps must adopt, and any push or merge to `main`.
