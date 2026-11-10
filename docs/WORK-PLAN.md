# Work plan

The plan for the round of changes asked for on 2026-10-23: bug fixes, improvements and new features.
Phases 9 to 15, planned on 2026-11-03, are the work after 0.5.0: the server fixes left from 0.5.0 and the 17 points of
the design review.
Phases 16 to 20, planned on 2026-11-10, come from a second review with 12 design videos: 7 faults and 11 improvements,
drawn before and after on a page in the scratchpad.
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
| D11 | First encryption of an existing history: encrypt, check it reads back, then delete the unencrypted files | You | Replaced by D32 |
| D12 | The full window uses the popup's keys: Enter pastes, Shift+Enter pastes plain text, Ctrl+Enter only copies; Alt+Shift+V opens it | You | Decided |
| D13 | Delete by age is off by default; pinned and tagged items are never deleted by it | You | Decided |
| D14 | Rules per app: ignore an app, and ignore text that matches a pattern (for one app or all); a copy a rule skipped can be kept with "Save anyway" | You | Decided |
| D15 | Image paths become relative to the data folder in the same one-time step as the encryption (5.1), so the history is migrated once | Claude | Replaced by D32 |
| D16 | Encryption covers items and images; search vectors stay unencrypted | You | Decided |
| D17 | If the history key can't be opened (another PC or account), the old history is set aside and the app starts empty, with a message | You | Decided |
| D18 | Portable copies are encrypted too; a portable history opens only on the PC and account that made it | You | Decided |
| D19 | Applying a search server that is not on this PC shows a warning and asks once | You | Decided |
| D20 | The release job checks its own update signatures before publishing, done with 5.2 (was 8.3) | You | Decided |
| D21 | Rules per app take simple patterns (* and ?) or regular expressions, with a switch per rule | You | Decided |
| D22 | Uninstalling asks whether to delete the history and settings | You | Decided |
| D23 | Each release starts its notes with its version's section of CHANGELOG.md | You | Decided |
| D24 | Inno Setup is installed on this PC, so the installer is tested before a release | You | Decided |
| D25 | The quick popup is its own small window with no taskbar button; the full window never moves or changes size. One click selects; double-click or Enter pastes | You | Decided |
| D26 | The popup has a bar at its top: Settings, the full window, a handle to move it, Close. A moved popup still opens at the mouse pointer the next time | You | Decided |
| D27 | Gray text meets the 4.5 to 1 contrast minimum everywhere, in both themes; indexing shows a spinner that moves five times a second, and a progress bar | You | Decided |
| D28 | Colored text (accent, status, kind and tag colors, links, selected text) meets the 4.5 to 1 minimum in both themes; buttons with white text get a deeper fill | You | Decided |
| D29 | candle comes from its code on GitHub at a fixed commit until a release after 0.11.0: one `tokenizers` and no C library build. Back to crates.io when that release is out | You | Done: candle 0.11.1 from crates.io (2026-11-01) |
| D30 | The dependency check (cargo-deny) is its own job, also run weekly: advisories, licenses and crate sources fail it; unmaintained crates and duplicate versions only warn | You | Decided |
| D31 | No tests through the screen for now (they would need egui_kittest): the new tests cover the update steps, old settings and the worker | You | Decided |
| D32 | Only you use the app: no migrations or code for older versions. A history an older version wrote is moved aside for a new one, old setting names are not read, and releases write only the signed update line | You | Decided |
| D33 | From the push on 2026-10-29 the app is stable and has users: a breaking change to settings or stored data ships with a migration in the next version, and the migration is removed in the version after that | You | Decided |
| D34 | The app icon: "copy stack + search" in Midnight + amber, on a transparent background with midnight edges, and a bolder drawing for 16 to 24 px | You | Decided |
| D35 | Downloads continue where they stopped, checked by the server's tag for the file, and after a dropped connection try again after 10 s, 30 s, 1, 2 and 5 min; the text reader model, which the background worker downloads, continues at the next start instead | You | Decided |
| D36 | The text reader's models download as ONNX files; `.rten` files that earlier versions downloaded still load | Claude | Decided |
| D37 | The server text reader has a Model type menu: DeepSeek-OCR-2, Unlimited-OCR-2 and MiniCPM-OCR get the name, prompt and request settings from their guides, and their answers are saved as plain text (table cells with spaces, rows on lines, formulas kept); every answer stops at 4096 tokens, which fits DeepSeek-OCR-2's 8192 with the image; the models run on the user's server (llama.cpp, vLLM, SGLang), never inside the app | You | Decided |
| D38 | The built-in search model (SigLIP) and text reader (ocrs) are gone: meaning search uses your own server or is off, and text in images is read by Windows (the default), a server, or not at all. Settings that named them turn search off and let Windows read, with a one-time note, and the files and vectors they left are deleted at the first start | You | Decided |
| D39 | Servers are saved with a type (llama.cpp, Ollama, LM Studio, vLLM, Unsloth, OpenRouter or another one), a name, an address on any port and an API key, and keep several models, each for searching or reading, with its own name and settings. Picking a model applies at once, each keeps its own index, and a server's settings wait for Save in its editor. Settings from 0.4 become saved servers as they were, so the model in use keeps its vectors | You | Decided |
| D40 | Server types show their brands' logos: llama.cpp, vLLM and Unsloth in their colours, the others white and tinted like the icons (Simple Icons, Lobe Icons and llama.cpp's own logo) | You | Decided |
| D41 | The server editor tests the connection and each model, takes model names the server doesn't list, and Fill in details looks a model up on models.dev (downloaded only when clicked; nothing from the history is sent) and fills in the prompts well-known embedding models are trained with. The vector size is shortened on this PC, which works with every server, as `dimensions` would fail on most models | You (Claude for the vector size) | Decided |
| D42 | GLM-OCR joins the reading model types, asked with "Text Recognition:" from its model card; llama.cpp serves it from ggml-org's GGUF | Claude | Decided |
| D43 | OpenRouter stays. Every request to it asks only for providers that don't collect user data, and says whether it is a search or an item (`input_type`), so OpenRouter search models index once more | You | Decided |
| D44 | windows-registry stays at 0.6 until auto-launch moves to 0.100, so the exe has one copy of it | You | Decided |
| D45 | Each phase ends with a check in a test copy: a dev build with its own data folder and a copy of the saved servers, using the main folder's build cache. Claude says before each run and asks before any model download | You | Decided |
| D46 | In a wide window, List rows are at most about 900 points wide, centered, with the time, pin and relevance next to the text | You | Decided |
| D47 | The app remembers the last version it ran; the first start of a newer one shows its notes from the change log built into the exe, so an update installed on quit is covered too, without the internet | You | Decided |
| D48 | Version 0.5.1 after Phase 9, then one 0.6.0 after Phases 10 to 15, with no push in between | You | Decided |
| D49 | When a server refuses an image, that model gets the image's read text instead until the next start or a model change, and one message offers to turn off Send images; the app doesn't change the setting by itself | Claude | Decided |
| D50 | Settings in seven pages by topic (General, Appearance, Clipboard, Privacy, Search & AI, Shortcuts and pasting, About), a short gray line under each setting, and the long text behind an info button that opens on a click, Enter or Space. Reset all settings keeps the saved servers and the rules per app | Claude | Decided |
| D51 | One font family: Noto Sans with Noto Sans Arabic, each in Regular and SemiBold, in five text sizes (12, 13, 14, 16, 20) | Claude | Decided |
| D52 | Ctrl+1 to Ctrl+9 paste the first nine items in both windows; only the popup shows the numbers. A message after a mouse action names its key once per action in each run | Claude | Decided |
| D53 | The getting-started checklist is for new installs only, and once closed it stays closed | Claude | Decided |
| D54 | Search measures meaning from the typical item's score up to the best one, so it works the same with every model: a match is strong when it contains all the words or is at least half way to the best, and the relevance bars show the same measure (one bar below strong, two from there, three near the best) | You | Decided |
| D55 | Messages look like shadcn's Sonner: a stack at the bottom right of the window (bottom left in Arabic), above the status bar. The newest is in front and up to two older ones peek out above it; pointing at the stack opens it and stops the timers, which also stop while the window is hidden. Undo keeps its 8 seconds, and Ctrl+Z undoes | You | Decided |
| D56 | When the window is not in front, a message about a copy appears as the app's own note on the screen: the same look with the app icon, at the bottom right of the screen with the pointer (bottom left in Arabic). It never takes the keyboard, lasts 8 seconds (longer while the pointer is on it), stays quiet during full-screen apps and presentations, and can be turned off in Settings, Privacy. Clicking it opens the window with the same message | You | Decided |
| D57 | "Not saved" for every copy the app skips: the kind (password, API key, sign-in token, private key, link with a password, your rule), the program and a hint: keys show their public start and last 4 characters, passwords only their length. In the window it also has Show (the whole text while held) and Save anyway; the note on the screen never shows the text. A copy marked private by its program names only the program. The text leaves memory when the message ends | You | Decided |
| D58 | Dark theme: "Dark panels" in Appearance, Darker (the panes and cards below the window, as before 0.7.0) or Lighter (raised surfaces lighter, menus lightest, with steps twice as big as in the light theme). Lighter is the default | You | Decided |
| D59 | While recording is paused the tray icon is gray and its tooltip says so. After Alt+C a note at the bottom right of the screen says the next copy won't be saved, for 2 seconds, and the tray icon has a badge until that copy | You | Decided |
| D60 | Delete by age and the item limit say how many items they removed, with Undo; the items wait, hidden, until the message is gone or the app quits | You | Decided |
| D61 | Esc or Back in a server's editor keeps the draft until Save, Discard or quitting the app; the server shows "Not saved" | You | Decided |
| D62 | Named corner sizes (4, 8, 12 and round), gaps in steps of 4 and every color in the theme file; a test fails on a raw number or color in screen code | You | Decided |
| D63 | Version 0.6.1 after Phase 16, then 0.7.0 after Phases 17 to 20 | You | Decided |

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
- [x] 2.30 A global shortcut changed in Settings did nothing until a restart: the library keeps only the first event handler (found while adding the quick popup)
- [x] 2.31 The quick popup closed the app when the full window was minimized: eframe 0.37 can't make a new window during such a frame (found in the Phase 6 check)
- [x] 2.32 With the full window open, the popup's search field didn't get the keyboard (found in the Phase 6 check)
- [x] 2.33 A few Arabic texts went to egui without the reading-order handling: the "none" hint, the vector sizes and two download progress texts showed their words in the wrong order (found in Phase 7)
- [x] 2.34 The pause button overlapped the end of the search field: the field left 160 px for buttons that take 171 (found after 0.4.0)
- [x] 2.35 The taskbar showed Windows' generic icon, because the exe had none; fixed with the app icon (D34, found after 0.4.0)
- [x] 2.36 Every update download failed at its end: its limit was the signed size, and ureq refuses even the last read once the limit is used up (found when 0.4.0 tried to update to 0.4.1; 0.4.0 and 0.4.1 copies need the installer once)
- [x] 2.37 The built-in text reader's model files are gone from their address: Amazon answers Access Denied for text-detection.rten and text-recognition.rten; the ocrs project now serves only ONNX versions there (found while testing resume); fixed: the app downloads the ONNX files (D36)
- [x] 2.38 Quitting while the window was minimized saved it at -32000 with no size (eframe saves every 30 seconds and at quit), so the next start opened it tiny in a corner, and a dev build stopped on a negative size in the empty details pane; now the entry from before the window was minimized goes back in, and a window that starts smaller than the smallest size gets its first size (found in the Phase 13 check)
- [x] 2.39 In the bar for selected items, "2 selected" sat 3 pixels above the labels of the buttons beside it (found in the Phase 14 check); not the font files, which have the same height measures in both weights: a row centers each item in the height it has so far, and a button's label sits a point higher in its box than a plain label's. Now the row starts as tall as the buttons, and the count is set like their labels

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
- [x] 6.1 Paste straight into the app you were using
- [x] 6.2 Paste as plain text
- [x] 6.3 Keep formatting (HTML and RTF)
- [x] 6.4 Quick popup at the cursor with search, and a button to open the full window
- [x] 6.5 Rules per app
- [x] 6.6 Delete by age
- [x] 6.7 OCR providers: built-in, Windows OCR, or an OCR server with a custom prompt (DeepSeek-OCR, PaddleOCR-VL, llama.cpp, other vision models)
- [x] 6.8 Register the app on the PC: Start menu and desktop shortcuts, an entry in Installed apps (D10)
- [x] 6.9 A bar at the top of the quick popup: Settings, the full window, a handle to move it, Close (D26)
- [x] 6.10 Downloads continue after a dropped connection: the update and the models (D35)
- [x] 6.11 The server text reader knows DeepSeek-OCR-2, Unlimited-OCR-2 and MiniCPM-OCR: a Model type menu with their prompts and settings, answers as plain text, and a README guide to start each one (D37)
- [x] 6.12 Your own servers only: the built-in engines are gone (D38); saved servers with several models each, any port, a logo for each type, Test and Fill in details (D39 to D41)
- [x] 6.13 GLM-OCR as a reading model type (D42)

### Phase 7. Speed and accessibility
- [x] 7.1 No repaint loop while indexing, cached file checks, thumbnail cache that keeps visible images
- [x] 7.2 Screen reader labels, focus rings, readable contrast
- [x] 7.3 Readable colored text (D28); gray text on a selected row in the dark theme and on search hits, which the first test did not measure

### Phase 8. Build, CI and tests
- [x] 8.1 One `tokenizers` version (no second copy, no C library build): candle from GitHub at a fixed commit (D29), then candle 0.11.1 from crates.io, which also brings zip 9 and ends the three zip advisories
- [x] 8.2 GitHub Actions pinned to commit hashes; the push hook reads the release from the comment and reports a candle release newer than the git commit
- [x] 8.3 The release job checks the update signature before publishing (moved into 5.2, D20)
- [x] 8.4 `cargo deny` for licenses and security advisories (D30)
- [x] 8.5 Clippy fails on warnings in CI
- [x] 8.6 More tests: installing, rolling back and cleaning up an update, a staged update, a 0.2.1 settings file, a search server the worker can't reach (D31)
- [!] 8.7 Code signing with SignPath, after your application is approved (D7: later)
- [x] 8.8 Remove the code for older versions: old setting names, the one-time history encryption, the update fields for 0.2.x copies (D32)

Phases 9 to 15 come in this order: the server fixes first, as a small release (D48); then type and layout, which
every later screen uses; then settings, content, keys and the new features; and the 0.6.0 clean-up last, with the
screenshots of the finished screens.

### Phase 9. Server fixes (version 0.5.1)
- [x] 9.1 Server errors in the interface language: can't reach, needs a key, not found, refused (with the server's own words), failed, doesn't take images, unexpected answer; the log stays in English
- [x] 9.2 Test also sends an image when Send images is on, and says whether the server takes images
- [x] 9.3 A server that refuses images: the images go as their read text, with one message and a button that turns off Send images (D49)
- [x] 9.4 OpenRouter: only providers that don't collect user data, and the search or item hint (D43)
- [x] 9.5 Crate updates within the versions `Cargo.toml` allows; windows-registry waits (D44)
- [x] 9.6 The README roadmap lists only what is still to do
- [x] 9.7 Esc while typing in a Settings field leaves the field, not Settings; Esc in a server's editor goes back to its page
- [x] 9.8 A check in a test copy with your servers (D45): Unsloth with its key, llama.cpp with EmbeddingGemma 2 and its image projector, vLLM with bge-small-en-v1.5, the model menu opened with a real click; LM Studio and Sentence Transformers were not running
- [x] 9.9 A search server that stops while the app runs: one message in the status bar, not a log line for each item
- [x] 9.10 Images also go in llama.cpp's and OpenRouter's request format, so Send images works with EmbeddingGemma 2 on llama.cpp

### Phase 10. Type and layout
- [x] 10.1 Noto Sans and Noto Sans Arabic in Regular and SemiBold, five text sizes, more space between the lines of long text (D51)
- [x] 10.2 One left and one right edge for every part of the window; List rows at most about 900 points wide (D46); the status bar names a model file by its file name
- [x] 10.3 A view switch in three parts, and Pause with a word
- [x] 10.4 The day header stays at the top while scrolling, in the style of section titles

### Phase 11. Settings
- [x] 11.1 Seven pages grouped by topic, each keeping its own scroll position (D50)
- [x] 11.2 A short gray line under each setting (each help's first sentence, four written anew); the info button opens the rest on a click, Enter or Space
- [x] 11.3 Reset all settings keeps the saved servers and the rules per app, and says so

### Phase 12. Content
- [x] 12.1 The program a copy came from: its icon and its own name, and the program in front for a WebView2 helper
- [x] 12.2 Content in its best form: decoded web addresses, a code icon (Fluent Code, your choice), color swatches, file paths
- [x] 12.3 Calmer grid cards: no kind badge, a footer with the program and the time, short text larger, pictures centered
- [x] 12.4 Weak search matches fold into one line; relevance bars on grid cards. Strong: contains all the words, or its meaning score is at least halfway from the typical score to the best

### Phase 13. Pasting and keys
- [x] 13.1 The details pane's main button names the action and the program it pastes into
- [x] 13.2 Quick paste with Ctrl+1 to Ctrl+9, numbers in the popup, a line of key hints, a short search hint (D52)
- [x] 13.3 A right-click menu on items, also with Shift+F10; Delete away from Pin and Copy
- [x] 13.4 F1 opens a sheet of every key; a message after a mouse action names its key (D52)

### Phase 14. New features
- [x] 14.1 Select several items at once
- [x] 14.2 A getting-started checklist for new installs (D53)
- [x] 14.3 What's new after an update (D47)
- [x] 14.4 Start llama-server for reading models, from a file or a Hugging Face name
- [x] 14.5 Smart search ranks meaning matches on the scale of D54, from the typical score up to the best, so a match that only shares a word no longer outranks close meaning matches with models such as EmbeddingGemma 2 (your choice)

### Phase 15. Version 0.6.0
- [x] 15.1 Remove the update from 0.4 settings, the old model folders' clean-up and the old vectors' deletion (D33)
- [x] 15.2 New screenshots from a test copy with made-up items
- [x] 15.3 README, README_AR, the change log and version 0.6.0 (D48)

Phases 16 to 20 come in this order: the faults first, as a small release (D63); then the messages, which later changes
use; then the look, motion and smaller changes; and the README, bug reports and 0.7.0 last, with new screenshots.

### Phase 16. Faults (version 0.6.1)
- [x] 16.1 Recording, thumbnails and program icons start again after a crash; if recording can't, the status bar says "Recording stopped" with Start again
- [x] 16.2 Dividers between the groups of the right-click menu and the model menu, in a soft line color; an edge around the quick popup like the menus have
- [x] 16.3 Rows with no day header above them say "Yesterday" instead of only the time
- [x] 16.4 The search server's real state: a red dot with Try again while requests fail, and the app tries again by itself; pictures whose text couldn't be read say so, with Try again
- [x] 16.5 The log: no user name or password in server addresses, a new file after 10 MB; the bug form and CONTRIBUTING point to Settings, About
- [x] 16.6 The change log and version 0.6.1

### Phase 17. Messages
- [x] 17.1 Messages in the Sonner style at the bottom right of the window (D55)
- [x] 17.2 The app's own note on the screen (D56)
- [x] 17.3 "Not saved" says what, with Show in the window (D57)
- [x] 17.4 Delete by age and the item limit say what they removed, with Undo (D60); a new copy pushing the oldest item out of a full history says nothing, as before
- [x] 17.5 Alt+C: a note and a tray badge (D59)

### Phase 18. Look
- [x] 18.1 Named sizes and colors, and a test against raw ones (D62)
- [x] 18.2 Dark panels: Darker or Lighter, Lighter by default (D58)
- [x] 18.3 Filter counts within the search results, filters with none dimmed, the result count in the popup
- [x] 18.4 Image rows: "Image" or the read text, then the size

### Phase 19. Motion and smaller changes
- [ ] 19.1 Deleted rows and cards fade out and the gap closes; messages slide in and out
- [x] 19.2 Filled buttons react to the pointer and to a press
- [x] 19.3 Nothing moves when Windows' "Animation effects" is off
- [x] 19.4 The tray icon turns gray while recording is paused, and its tooltip says so (D59)
- [x] 19.5 The hover buttons' tips name their keys
- [ ] 19.6 A server's editor keeps your draft (D61)

### Phase 20. README, bug reports and 0.7.0
- [ ] 20.1 README and README_AR open with one line, the download, a screenshot and three benefits (texts shown to you first)
- [ ] 20.2 The log's first line gives the version, the build and the Windows version, without your user name; item ids and times on reading and indexing lines
- [ ] 20.3 About: Copy details for a bug report
- [ ] 20.4 New screenshots, the change log and version 0.7.0

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
| 2026-10-29 | `be36fe2` to `8360d79` (13 commits), merge `fa2cfa5` | Phase 6 (features), version 0.4.0: delete by age, rules per app, kept formatting, changed global shortcuts working at once (2.30), pasting into the program you were using with plain-text and copy-only keys, the quick popup in its own window (D25), the choice of text reader, a per-user installer; the check's popup fixes (2.31, 2.32) |
| 2026-10-29 | `937026c`, merge `6e92fbe` | The quick popup's top bar: Settings, the full window, a handle to move it, Close (6.9, D26), checked in a dev build in English and Arabic |
| 2026-10-29 | `4b651cc` to `6cb7795` (9 commits), merge `02bd857` | Phase 7 (speed and accessibility): a slow spinner and a progress bar while indexing (about 60 frames a second down to about 8), thumbnails on screen kept, disk checks remembered, screen reader names with Arabic in reading order, a focus outline, readable gray text (D27); the Arabic texts fix (2.33) |
| 2026-10-29 | `d27d621`, `27c31f9`, `1cec729`, merge `88da6f2` | Readable colored text (7.3, D28): accent, status, kind and tag colors as text, links and selected text in a shade that reads on every background they sit on; deeper button fills behind white text; gray text on dark selected rows and search hits, which the first contrast test did not measure |
| 2026-10-29 | `6606f13` to `2ff5491` (7 commits), merge `14dc0d0` | Phase 8 (build, checks, tests): candle from GitHub at a fixed commit, so one `tokenizers` and no C library (D29); GitHub Actions pinned to commits, and the push hook reads them; lint warnings fail the GitHub check; a cargo-deny job for advisories, licenses and crate sources, also weekly (D30); tests for installing, rolling back and cleaning up updates, a 0.2.1 settings file and a search server the worker can't reach (D31) |
| 2026-10-29 | `3c9dcb5` to `fa90042` (4 commits), merge `34d37c9` | The code for older versions removed (8.8, D32): old setting names, the one-time history encryption and format upgrade (a plain history is moved aside instead), the update fields and exe signature for 0.2.x copies |
| 2026-11-01 | `976e259` to `209256f` (7 commits), merge `f9001a1` | Version 0.4.1: room for the top bar buttons (2.34); the app icon in the exe, window, tray and installer (2.35, D34), with the app_icon example that makes app.ico; a large-icon change tried and reverted, since eframe already sets it |
| 2026-11-01 | `e570f50`, merge `6e6d19c` | candle 0.11.1 from crates.io instead of a GitHub commit (D29 done): one `tokenizers`, no C library, and zip 9, so deny.toml lists no advisories |
| 2026-11-01 | `0a55ba6` to `93f9af2` (3 commits), merge `e10464c` | Version 0.4.2: update downloads no longer fail at their end (2.36); 0.4.0 and 0.4.1 copies need the installer once |
| 2026-11-01 | `3288801`, `3e402ba`, merge `f7a6041` | Downloads continue after a dropped connection (6.10, D35), checked against GitHub and Hugging Face with 1 KB ranges; found that the text reader's model files are gone from their address (2.37, open) |
| 2026-11-01 | `1fae543`, `6fc8a7e`, merge `3ad5229` | The text reader's models download as ONNX files (2.37, D36); checked by downloading them from the real address and reading a test image with both formats |
| 2026-11-01 | `a03e64d` to `a753dbb` (4 commits), merge `da7f969` | Version 0.4.3: the Model type menu for the server text reader (6.11, D37), with a README guide; checked with a stand-in server, not with the real models or in the app's window; Unlimited-OCR-2 has no llama.cpp or vLLM guide yet, only SGLang |
| 2026-11-02 | `544b8f9` to `db8143a` (11 commits), merge `fd6d77c` | Version 0.5.0: your own servers only (6.12, D38 to D41): the built-in search model and text reader are gone; saved servers with several models each, a type and logo, any port, Test and Fill in details from models.dev; GLM-OCR as a reading model type (6.13, D42). Checked: the update step on a copy of your settings file, and the screens in a dev build with a stand-in server, which found 4 small faults, fixed in the last 4 commits; not checked with real model servers |
| 2026-11-04 | `902ac42` to `53cbaeb` (16 commits), merge `525adbb` | Version 0.5.1 (Phase 9): server errors in both languages; Test tries an image; images a text-only server refuses are indexed by their read text, with one message and a button; one message when the search server stops; images also in llama.cpp's and OpenRouter's request format; OpenRouter asks for providers that keep no data and sends the search or item hint; Esc in Settings; crate updates. Checked in a test copy with Unsloth (its key), llama.cpp (EmbeddingGemma 2 with its image projector: images work) and vLLM (bge-small-en-v1.5; vLLM 0.30 can't load the EmbeddingGemma 2 GGUF), which found 4 small faults, fixed here |
| 2026-11-04 | `ed1cad9` to `25ae76b` (7 commits), merge `b4867db` | Phase 10 (type and layout): Noto Sans with Noto Sans Arabic in Regular and SemiBold, five text sizes, long text in lines of about 70 characters (D51); one left and one right edge; List rows at most 900 points wide (D46); the view switch in three parts and Pause with its word; the day header pinned while scrolling; a sample history for checks. Checked in a test copy in English and Arabic; the pinned header's push-up only by a unit test |
| 2026-11-04 | `6d2338f` to `5e5a00a` (4 commits), merge `9f6c42a` | Phase 11 (settings): seven pages by topic, each with its own scroll position, and a Privacy page that shows where each kind of data goes (D50); a short gray line under each setting (each help's first sentence, four written anew in both languages), and an info button that opens the rest on a click, Enter or Space; Reset all settings keeps saved servers and rules. Checked in a test copy in English and Arabic |
| 2026-11-04 | `9b87a8b` to `1843d2d` (9 commits), merge `d602ca5` | Phase 12 (content): the program a copy came from, with its own icon and name (WebView2 helpers name the program you saw); web addresses with their letters decoded, also for search; color swatches; paths with their name first and Show in folder; a code icon (Fluent Code); calmer grid cards with a footer for the program and the time; weaker search matches folded under one line, and relevance bars measured from the typical match up to the best (D54), on cards too; the list no longer opens scrolled down by one row. Checked in a test copy in English and Arabic, and with EmbeddingGemma 2 on a local llama.cpp server |
| 2026-11-04 | `80cd287` to `f02ed0e` (9 commits), merge `ca63c72` | Phase 13 (pasting and keys): the details pane's main button names the program it pastes into, with its key, and holds the other ways behind its arrow; Ctrl+1 to Ctrl+9 paste one of the first nine items, numbered in the popup, with a quiet line of keys and a short search hint (D52); a right-click menu on rows and cards, also with Shift+F10, with every action and its key and Delete last, in red; the rule link moved into it; F1 opens a sheet of every key; the first pin, copy or delete with the mouse in a run names its key (D52); the plan gained 14.5 (Smart ranking, your choice). Found in the check and fixed: Esc on an open menu also cleared the search or hid the window; the key sheet kept half its height; one-line row text ran under the pin; an Arabic key cap read backwards. Checked in a test copy in English and Arabic; Ctrl+1 to 9 only by a unit test, and the copy and delete messages not on screen (they share the pin message's code) |
| 2026-11-04 | `9ee7b02`, `5547cb6`, merge `daee0a6` | Bug 2.38, found in the Phase 13 check: a window quit while minimized keeps the place and size it had before, and a window saved that way by an older version starts at its first size instead of tiny in a corner. Checked in a test copy: minimized for 35 seconds, then quit; app.ron kept 32,32 and 1040 by 680 points, and the next start opened there; with the old broken entry the window opened at 1040 by 680 points instead of 98 by 127 pixels, and the dev build no longer stopped |
| 2026-11-05 | `88e4ffa` to `07f405f` (8 commits), merge `c97ef71` | Phase 14 (new features): several items selected with Ctrl or Shift clicks, Shift with the arrows, or Ctrl+A, then pinned, tagged, copied together or deleted at once from the status bar, the right-click menu or their keys, with one Undo for a delete; a getting-started checklist for new installs that ticks itself (D53); the first start of a newer version says so, with a What's new sheet from the change log (D47); llama-server started for reading models too, from a file or a Hugging Face name, one server per job, with a status line while it starts; Smart search ranks meaning matches from the typical match up to the best (14.5, D54). Found in the check and fixed: the reading server's status line was cleared by the search model's; the checklist above the list lost its gaps. Checked in a test copy: selecting and acting on several items in Grid and List, the checklist in an empty folder and above a history, What's new, and Gemma 4 E2B started for reading text. Only unit tests for Copy together, Ctrl+1 to 9 and the Smart order; starting by Hugging Face name not tried (your choice). Found, open: 2.39 |
| 2026-11-05 | `cbf73af` to `d94495a` (5 commits), merge `19e4e17` | Phase 15, version 0.6.0: bug 2.39 fixed (the group bar's row starts as tall as a button and the count uses the buttons' text format, so "2 selected" sits on their line; measured in a test and on screen); the update of 0.4 settings removed, with the old model folders' clean-up and the built-in model's vectors' deletion, their tests and three messages (D33), and a change-log note for people who skipped 0.5; the READMEs describe selecting several items, the new keys, the seven settings pages, the checklist, What's new and llama-server for reading models; version 0.6.0; new screenshots from made-up items with recording on, at 1600 by 1000 pixels, and a new social preview (you upload it on GitHub). Before this phase, at your request, Phases 10 to 14 went out as 0.5.1 build 24 |
| 2026-11-10 | `633ebaa` to `d321fc8` (10 commits), merge `030f36b` | Phase 16, version 0.6.1: Phases 16 to 20 planned with D55 to D63; a copy that crashes the clipboard watcher is skipped, three crashes in a minute stop it with "Recording stopped" and Start again in the status bar, and the thumbnail and program threads go on after a crash; dividers in the right-click menu and the model menu, and an edge around the quick popup; "Yesterday" on rows with no day header; the search server's real state in red with Try again, retries after 30 seconds and then twice as late up to every 10 minutes, a model whose server was down at start loads once it answers, and no requests while it is down; pictures whose text could not be read say so, with Try again; no user name or password in logged addresses or the connecting line; a new log file after 10 MB; the bug form and CONTRIBUTING point to Settings, About; the change log and version 0.6.1. Unit tests for the crash count, Yesterday, the login, the log limit and the server state; not checked on screen (your choice) |
| 2026-11-10 | `a2065a7` to `c8f8959` (5 commits), merge `0d4afe0` | Phase 17 (messages): messages in the style of shadcn's Sonner at the bottom right of the window (bottom left in Arabic), above the status bar, the newest in front and two older ones peeking out, opening when pointed at; their time runs only while a window shows them and the pointer is elsewhere; a delete becomes final when its message is gone, and Ctrl+Z undoes it, also from the search field (D55); "Not saved" names what the copy looked like (a password, an API key, a sign-in token, a private key, a link with a password, a key) and the program, with a hint that hides it and Show in the window (D57); while neither window is in front, such a message shows as the app's own note at the screen's corner, with the app icon, never taking the keyboard, quiet during full-screen programs and presentations, and off in Settings, Privacy (D56); delete by age and a lower history limit say what they removed, with Undo (D60); Alt+C shows a 2-second note and a badge on the tray icon until the skipped copy (D59). Unit tests for the message layout, the secret kinds and hints, the badge and the prune selection; not checked on screen |
| 2026-11-10 | `087d91c` to `1972659` (4 commits), merge `6873100` | Phase 18 (look): Dark panels in Settings, Appearance, Darker as before or Lighter, the new default, with the window darkest, panes and cards a step lighter and menus another (D58), gray text a little lighter with it and colored dark-theme text a little lighter in both, the contrast tests run for both choices; while searching, the filters and tags count the results each would show, from the same search over every kind and tag, and dim when they have none; the quick popup says how many results there are; image rows show the read text, the file name or "Image" over their size; corners take four named sizes, gaps named steps of 4 and colors the theme file, and a test fails on a raw one in the window's code (D62). Unit tests for the counts, the contrast of both dark choices and the named sizes; not checked on screen |
