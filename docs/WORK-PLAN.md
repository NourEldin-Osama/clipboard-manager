# Work plan

The plan for the round of changes asked for on 2026-10-23: bug fixes, improvements and new features.
This file is updated with every change.

Status: `[ ]` to do, `[~]` in progress, `[x]` done, `[!]` waiting for a decision or for you.

## Decisions

| # | Decision | Who | Status |
|---|---|---|---|
| D1 | API keys are encrypted with Windows DPAPI, so only your Windows account can read them | Claude | Decided |
| D2 | Embedding server settings are remembered for each server address | You | Decided |
| D3 | UI text uses Project Fluent: semantic keys, whole sentences, placeholders, Arabic plural rules | Claude | Decided |
| D4 | History encryption at rest: key tied to your Windows account, or a password at start | You | Waiting |
| D5 | Shortcuts for the quick popup and for paste, copy and plain-text paste | You | Waiting |
| D6 | Default OCR provider for new installs | You | Waiting |
| D7 | Code signing: SignPath Foundation needs an application from you | You | Waiting |
| D8 | Sync between devices is out of scope | You | Decided |
| D9 | Git: one branch per phase, one commit per task, merged into local `main` with `--no-ff`; you push | You | Decided |

## Order of work, and why

1. **Tooling:** quick, and the hooks then check every later change.
2. **Bugs:** small fixes you get early.
3. **Structure:** split the big files and give items a single writer, so new features don't make them bigger.
4. **Text system:** new screens use the new text system from their first line.
5. **Security:** encryption is in place before features that store more data (formatting, OCR from servers).
6. **Features.**
7. **Speed and accessibility:** after the new screens exist.
8. **Build, CI and tests.**

## Tasks

### Phase 1. Tooling
- [x] 1.1 Hooks use prek's built-in versions; the config moves to `prek.toml` (prek 0.5.7 or newer)
- [x] 1.2 The pipelines skip changes to unrelated files (docs, README, templates, hook config)

### Phase 2. Bugs
- [x] 2.1 Global shortcuts stay off after a cancelled or unchanged shortcut recording
- [x] 2.2 Embedding server settings are remembered per server address (the Unsloth report)
- [x] 2.3 API keys are encrypted with Windows DPAPI
- [x] 2.4 OCR no longer overwrites pins and tags
- [x] 2.5 Secret filter: no false alarms on file names, catches more real secrets, "Save anyway"
- [x] 2.6 Tag filter works with meaning search
- [x] 2.7 The tag field no longer freezes the keyboard
- [x] 2.8 Tray "Pause recording" works while Settings is open
- [x] 2.9 Grid arrow keys go the right way in Arabic
- [x] 2.10 Day headers without O(n squared) work every frame
- [x] 2.11 Sign-in entry: quoted path, not re-enabled at every start
- [x] 2.12 A settings file that fails to load is kept as `.bad` and logged
- [x] 2.13 A second start shows the first window instead of quitting silently
- [x] 2.14 Check, then fix: "Back to default" buttons that do nothing (confirmed: 6 rows)
- [x] 2.15 Check, then fix: a server request for every key press in the server settings (confirmed)
- [ ] 2.16 Check, then fix: no network timeouts
- [ ] 2.17 Check, then fix: an update that never installs when Windows shuts down
- [ ] 2.18 Check, then fix: Esc in Settings applies a half-typed model
- [ ] 2.19 Check, then fix: delete has no undo

### Phase 3. Structure (no behavior change)
- [ ] 3.1 Split `view.rs` (theme, widgets, time format, top bar, list, grid, details, status bar, keyboard)
- [ ] 3.2 Split `settings_page.rs` (one file per page, plus the server settings)
- [ ] 3.3 Move secret detection into `privacy.rs`
- [ ] 3.4 A `Library` that owns items, store, tags, pins and pruning; one `Command` enum for user actions
- [ ] 3.5 Store: `update` in one transaction, batch writes, `delete_many`, smaller cache, relative image paths, schema version
- [ ] 3.6 A background store thread for delete, prune, clear, import and PNG work
- [ ] 3.7 Worker queue: priorities, survives panics, restarts
- [ ] 3.8 Search runs on a background thread
- [ ] 3.9 Code quality: `setting_for` helper, enums instead of numbers and strings, no ignored errors

### Phase 4. Text system
- [ ] 4.1 Project Fluent with semantic keys, whole sentences, placeholders and Arabic plurals
- [ ] 4.2 Every UI text moves to `locales/en` and `locales/ar`; nothing hard-coded
- [ ] 4.3 Tests: every key in every language, matching placeholders, no unused keys
- [ ] 4.4 Guide: how to add a language

### Phase 5. Security and privacy
- [ ] 5.1 Encryption at rest for the history (D4)
- [ ] 5.2 Updates: the signature also covers version and build; size limit; HTTPS only
- [ ] 5.3 A warning when an embedding or OCR server is not on this PC
- [ ] 5.4 `tools/st_server.py`: Host and Origin checks, request size limit

### Phase 6. Features
- [ ] 6.1 Paste straight into the app you were using
- [ ] 6.2 Paste as plain text
- [ ] 6.3 Keep formatting (HTML and RTF)
- [ ] 6.4 Quick popup at the cursor with search, and a button to open the full window
- [ ] 6.5 Rules per app
- [ ] 6.6 Delete by age
- [ ] 6.7 OCR providers: built-in, Windows OCR, or an OCR server with a custom prompt (DeepSeek-OCR, PaddleOCR-VL, llama.cpp, other vision models)

### Phase 7. Speed and accessibility
- [ ] 7.1 No repaint loop while indexing, cached file checks, thumbnail cache that keeps visible images
- [ ] 7.2 Screen reader labels, focus rings, readable contrast

### Phase 8. Build, CI and tests
- [ ] 8.1 One `tokenizers` version (no second copy, no C library build) if candle allows it
- [ ] 8.2 GitHub Actions pinned to commit hashes
- [ ] 8.3 The release job checks the update signature before publishing
- [ ] 8.4 `cargo deny` for licenses and security advisories
- [ ] 8.5 Clippy fails on warnings in CI
- [ ] 8.6 More tests: worker, update install and rollback, settings migration, app logic
- [ ] 8.7 Code signing with SignPath, after your application is approved (D7)

## Change log

| Date | Commits | Change |
|---|---|---|
| 2026-10-23 | `e2813c5` to `8b0c5d8` | Before this plan: rustfmt, clippy fixes, dependency updates, format check in CI, prek hooks |
| 2026-10-23 | `ff0e6aa`, `c7ff1e4`, `8138797`, merge `67c3b1e` | Phase 1: this plan, prek built-in hooks, pipelines skip non-code changes |
