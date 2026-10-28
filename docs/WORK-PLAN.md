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
| D4 | History encryption at rest: a key tied to your Windows account (DPAPI), no password | You | Decided |
| D5 | Alt+V opens the quick popup at the cursor; there, Enter pastes into the app you were using, Shift+Enter pastes plain text, Ctrl+Enter only copies; the full window opens from the popup | You | Decided |
| D6 | New installs use Windows OCR by default | You | Decided |
| D7 | Code signing with SignPath Foundation: later | You | Decided (later) |
| D8 | Sync between devices is out of scope | You | Decided |
| D9 | Git: one branch per phase, one commit per task, merged into local `main` with `--no-ff`; you push | You | Decided |
| D10 | A per-user installer (Inno Setup) built with each release: no admin rights, Start menu and desktop shortcuts, an Installed apps entry with Uninstall; the portable exe stays | You | Decided |
| D11 | First encryption of an existing history: encrypt, check it reads back, then delete the unencrypted files | You | Decided |
| D12 | The full window uses the popup's keys: Enter pastes, Shift+Enter pastes plain text, Ctrl+Enter only copies; Alt+Shift+V opens it | You | Decided |
| D13 | Delete by age is off by default; pinned and tagged items are never deleted by it | You | Decided |
| D14 | Rules per app: ignore an app, and ignore text that matches a pattern (for one app or all); a copy a rule skipped can be kept with "Save anyway" | You | Decided |
| D15 | Image paths become relative to the data folder in the same one-time step as the encryption (5.1), so the history is migrated once | Claude | Decided |
| D16 | Encryption covers items and images; search vectors stay unencrypted | You | Decided |
| D17 | If the history key can't be opened (another PC or account), the old history is set aside and the app starts empty, with a message | You | Decided |
| D18 | Portable copies are encrypted too; a portable history opens only on the PC and account that made it | You | Decided |
| D19 | Applying a search server that is not on this PC shows a warning and asks once | You | Decided |
| D20 | The release job checks its own update signatures before publishing, done with 5.2 (was 8.3) | You | Decided |

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
- [x] 1.3 A prek hook that reports newer versions of the crates and GitHub Actions in use

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
- [x] 2.16 Check, then fix: no network timeouts (confirmed; also cut-off downloads)
- [x] 2.17 Check, then fix: an update that never installs when Windows shuts down (confirmed in code; shutdown not run)
- [x] 2.18 Check, then fix: Esc in Settings applies a half-typed model (confirmed)
- [x] 2.19 Check, then fix: delete has no undo (confirmed)
- [x] 2.20 Touchpad scrolling stops where the fingers stop (fast swipes kept moving)
- [x] 2.21 A global shortcut that another app already uses is detected and shown
- [x] 2.22 "Clear clipboard history" counted the tagged items it keeps (found in 3.4)
- [x] 2.23 Shift+Delete, the default Delete key, did nothing on Windows: egui-winit turns it into "cut" (found in the dev-build test)
- [x] 2.24 After the Add files, Add a folder and Save image dialogs, typing did not reach the search box (found in the dev-build test)
- [x] 2.25 Opening the window kept the last selection, so Enter could copy an old item; now the top item is selected (found in the dev-build test)
- [x] 2.26 The tray menu did nothing after the tray was built again (a new language, or the tray turned off and on) until a restart (found in the Phase 4 check)
- [x] 2.27 In the status bar a long error ran over the item count, and an Arabic error would show its words in the wrong order (found in the Phase 4 check)
- [x] 2.28 A portable copy or a dev build (CLIPBOARD_MANAGER_HOME) kept its window state in the installed app's folder (found in the Phase 4 check)
- [x] 2.29 The update banner and the update status in Settings cut a long Arabic message at its start, and a long message could run over the buttons or the version next to it (found in the Phase 4 check)

### Phase 3. Structure (no behavior change)
- [x] 3.1 Split `view.rs` (theme, widgets, time format, top bar, list, grid, details, status bar, keyboard)
- [x] 3.2 Split `settings_page.rs` (one file per page, plus the server settings)
- [x] 3.3 Move secret detection into `privacy.rs`
- [x] 3.4 A `Library` that owns items, store, tags, pins and pruning; one `Command` enum for user actions
- [x] 3.5 Store: `update` in one transaction, batch writes, `delete_many`, smaller cache, schema version (relative image paths: in 5.1, D15)
- [x] 3.6 Slow disk work off the UI thread: import (own thread, one transaction per batch), PNG save and copy-back (one background thread). Delete, prune and clear stay: measured 2-3 ms
- [x] 3.7 Worker queue: priorities, survives panics, restarts
- [x] 3.8 Search runs on a background thread (measured: 85 ms at 100,000 items, 650 ms for 1,000 multi-vector images; the snapshot it needs takes 4 ms at 100,000)
- [x] 3.9 Code quality: `setting_for` helper, enums instead of numbers and strings, no ignored errors

### Phase 4. Text system
- [x] 4.1 Project Fluent with semantic keys, whole sentences, placeholders and Arabic plurals
- [x] 4.2 Every UI text moves to `locales/en` and `locales/ar`; nothing hard-coded
- [x] 4.3 Tests: every key in every language, matching placeholders, no unused keys
- [x] 4.4 Guide: how to add a language

### Phase 5. Security and privacy
- [x] 5.1 Encryption at rest for the history (D4, D11, D15 to D18)
- [x] 5.2 Updates: the signature also covers version and build; size limit; HTTPS only; the release job checks its own signatures (D20)
- [x] 5.3 A warning when an embedding or OCR server is not on this PC, and a question on Apply (D19)
- [x] 5.4 `tools/st_server.py`: Host and Origin checks, request size limit

### Phase 6. Features
- [ ] 6.1 Paste straight into the app you were using
- [ ] 6.2 Paste as plain text
- [ ] 6.3 Keep formatting (HTML and RTF)
- [ ] 6.4 Quick popup at the cursor with search, and a button to open the full window
- [ ] 6.5 Rules per app
- [ ] 6.6 Delete by age
- [ ] 6.7 OCR providers: built-in, Windows OCR, or an OCR server with a custom prompt (DeepSeek-OCR, PaddleOCR-VL, llama.cpp, other vision models)
- [ ] 6.8 Register the app on the PC: Start menu and desktop shortcuts, an entry in Installed apps (D10)

### Phase 7. Speed and accessibility
- [ ] 7.1 No repaint loop while indexing, cached file checks, thumbnail cache that keeps visible images
- [ ] 7.2 Screen reader labels, focus rings, readable contrast

### Phase 8. Build, CI and tests
- [ ] 8.1 One `tokenizers` version (no second copy, no C library build) if candle allows it
- [ ] 8.2 GitHub Actions pinned to commit hashes
- [x] 8.3 The release job checks the update signature before publishing (moved into 5.2, D20)
- [ ] 8.4 `cargo deny` for licenses and security advisories
- [ ] 8.5 Clippy fails on warnings in CI
- [ ] 8.6 More tests: worker, update install and rollback, settings migration, app logic
- [!] 8.7 Code signing with SignPath, after your application is approved (D7: later)

## Change log

| Date | Commits | Change |
|---|---|---|
| 2026-10-23 | `e2813c5` to `8b0c5d8` | Before this plan: rustfmt, clippy fixes, dependency updates, format check in CI, prek hooks |
| 2026-10-23 | `ff0e6aa`, `c7ff1e4`, `8138797`, merge `67c3b1e` | Phase 1: this plan, prek built-in hooks, pipelines skip non-code changes |
| 2026-10-23 | `a12e456` to `c564da8` (19 commits), merge `8bcf88d` | Phase 2: all 13 bugs and the 6 checked agent findings, one commit each |
| 2026-10-23 | `dd39a9a` to `0c33161` (4 commits), merge `976bcca` | Decisions D4 to D7, touchpad scrolling (2.20), shortcuts used by another app (2.21), the hook that reports newer crates and actions |
| 2026-10-23 | `78ff429` to `720aac5` (16 commits), merge `bcef26c` | Phase 3: decisions D10 to D15; privacy.rs; view and settings split into folders; store schema version and batch writes; worker priorities and crash safety; Library and Command; Clear history count (2.22); image saving, copy-back and import off the UI thread; search on its own thread; setting_for; enums for string states; dropped errors reported |
| 2026-10-23 | `48c3e08`, `47d826f`, `201990e`, merge `e7f518c` | Found by running a dev build: Shift+Delete (2.23), search box focus after file dialogs (2.24), top item selected when the window opens (2.25); all three checked in the dev build |
| 2026-10-23 | `c682e87` to `7eec6b4` (7 commits), merge `7d7bdc7` | Phase 4: interface texts in Project Fluent files (locales/en, locales/ar) with the `t!` and `log_error!` macros; every text moved, with plural forms per language; tests for keys, values and use; the old table removed; guide in docs/TRANSLATING.md; checked in a dev build in both languages, which also made five Arabic texts start with an Arabic word |
| 2026-10-23 | `42d0fde`, `5ee0cf5`, merge `9d83e2d` | Found in the Phase 4 check: the tray menu after the tray is built again (2.26); the status bar error fits before the item count and keeps the Arabic word order (2.27); both checked in the dev build |
| 2026-10-23 | `5044dd7`, `89d1327`, merge `c4f4839` | Found in the Phase 4 check: the window state stays in the data folder, so a portable copy or a dev build no longer writes the installed app's file (2.28); the update banner and the update status in Settings are cut in reading order and fit next to their buttons (2.29); both checked in the dev build, the banner by reading the code only (a dev build has no updates) |
| 2026-10-28 | `ebe97b3` to `f0e2ebe` (7 commits), merge `017d52c` | Security: the Sentence Transformers helper refuses web pages, other host names and requests over 64 MB; a warning, and a question on Apply, before the history goes to a server outside this PC; every update value is signed, and the release job checks the signatures before publishing; the history is encrypted on Windows, an existing one once and checked before its plain files are deleted; checked in a dev build with a history made by the previous version |
| 2026-10-28 | `8782e7a` | Version 0.3.0: the first release with the new text system, the encrypted history and the new update signing |
