# Work plan

The decisions behind the app, and what is still open. Code and CLAUDE.md refer to the decisions by number, so a
number never changes or comes back. The phases, review rounds and change log up to version 0.10.0 are in the git
history (the last full version of this file is in commit `c880dca`). Versions and their notes are in CHANGELOG.md.

## Open

- Code signing with SignPath Foundation, after the application is approved (D7).

## Decisions

| # | Decision |
|---|---|
| D1 | API keys are encrypted with Windows DPAPI, so only your Windows account can read them |
| D3 | UI text uses Project Fluent: semantic keys, whole sentences, placeholders, Arabic plural rules |
| D4 | The history is encrypted at rest with a key tied to your Windows account (DPAPI), no password |
| D5 | Alt+V opens the quick popup at the cursor; there, Enter pastes into the app you were using, Shift+Enter pastes plain text, Ctrl+C only copies; the full window opens from the popup |
| D6 | New installs read the text in images with Windows |
| D7 | Code signing with SignPath Foundation: later |
| D8 | Sync between devices is out of scope |
| D9 | Git: a branch for each piece of work, one commit per change, merged into local `main` with `--no-ff`; you push |
| D10 | A per-user installer (Inno Setup) with each release: no admin rights, Start menu and desktop shortcuts, an Installed apps entry with Uninstall; the portable exe stays |
| D12 | The full window uses the popup's keys: Enter pastes, Shift+Enter pastes plain text, Ctrl+C only copies; Alt+Shift+V opens it |
| D13 | Delete by age is off by default; pinned and tagged items are never deleted by it |
| D14 | Rules per app: ignore an app, and ignore text that matches a pattern (for one app or all); a copy a rule skipped can be kept with Save anyway |
| D16 | Encryption covers items and images; search vectors stay unencrypted |
| D17 | If the history key can't be opened (another PC or account), the old history is set aside and the app starts empty, with a message |
| D18 | Portable copies are encrypted too; a portable history opens only on the PC and account that made it |
| D19 | Applying a search server that is not on this PC shows a warning and asks once |
| D20 | The release job checks its own update signatures before publishing |
| D21 | Rules per app take simple patterns (* and ?) or regular expressions, with a switch per rule |
| D22 | Uninstalling asks whether to delete the history and settings |
| D23 | Each release starts its notes with its version's section of CHANGELOG.md |
| D24 | The installer is tested on this PC, where Inno Setup is installed, before a release |
| D25 | The quick popup is its own small window with no taskbar button; the full window never moves or changes size. One click selects; double-click or Enter pastes |
| D26 | The popup has a bar at its top: Settings, the full window, a handle to move it, Close. A moved popup still opens at the mouse pointer the next time |
| D27 | Gray text meets the 4.5 to 1 contrast minimum everywhere, in both themes; indexing shows a spinner that moves 30 times a second, and a progress bar |
| D28 | Colored text (accent, status, kind and tag colors, links, selected text) meets the 4.5 to 1 minimum in both themes; buttons with white text get a deeper fill |
| D30 | The dependency check (cargo-deny) is its own job, also run weekly: advisories, licenses and crate sources fail it; unmaintained crates and duplicate versions only warn |
| D31 | No tests through the screen for now (they would need egui_kittest) |
| D33 | The app is stable and has users (since the push on 2026-10-29): a breaking change to settings or stored data ships with a migration in the next version, and the migration is removed in the version after that |
| D34 | The app icon: "copy stack + search" in Midnight + amber, on a transparent background with midnight edges, and a bolder drawing for 16 to 24 px |
| D35 | Downloads continue where they stopped, checked by the server's tag for the file, and after a dropped connection try again after 10 s, 30 s, 1, 2 and 5 min |
| D37 | The server text reader has a Model type menu: DeepSeek-OCR-2, Unlimited-OCR-2 and MiniCPM-OCR get the name, prompt and request settings from their guides, and their answers are saved as plain text (table cells with spaces, rows on lines, formulas kept); every answer stops at 4096 tokens; the models run on the user's server (llama.cpp, vLLM, SGLang), never inside the app |
| D38 | No built-in models: search by meaning uses your own server or is off, and the text in images is read by Windows (the default), a server, or not at all |
| D39 | Servers are saved with a type (llama.cpp, Ollama, LM Studio, vLLM, Unsloth, OpenRouter or another one), a name, an address on any port and an API key, and keep several models, each for searching or reading, with its own name and settings. Picking a model applies at once, each keeps its own index, and a server's settings wait for Save in its editor |
| D40 | Server types show their brands' logos: llama.cpp, vLLM and Unsloth in their colours, the others white and tinted like the icons |
| D41 | The server editor tests the connection and each model, takes model names the server doesn't list, and Fill in details looks a model up on models.dev (downloaded only when clicked; nothing from the history is sent) and fills in the prompts well-known embedding models are trained with; a model models.dev doesn't list takes the server's name for it. The vector size is shortened on this PC, as `dimensions` fails on most models |
| D42 | GLM-OCR is a reading model type, asked with "Text Recognition:" from its model card; llama.cpp serves it from ggml-org's GGUF |
| D43 | OpenRouter stays. Each request says whether it is a search or an item (`input_type`), and asks only for providers that keep no data while the server's switch, Only providers that keep no data, is on (the default); free models need it off |
| D46 | In a wide window, List rows are at most about 900 points wide, centered, with the time, pin and relevance next to the text |
| D47 | The app remembers the last version it ran; the first start of a newer one shows its notes from the change log built into the exe, without the internet |
| D49 | When a server refuses an image, that model gets the image's read text instead until the next start or a model change, and one message offers to turn off Send images; the app doesn't change the setting by itself |
| D50 | A short gray line under each setting, and a longer text behind an info button (a click, Enter or Space) only where that text adds facts. Reset all settings keeps the saved servers and the rules per app |
| D51 | One font family: Noto Sans with Noto Sans Arabic, each in Regular and SemiBold, in five text sizes (12, 13, 14, 16, 20) |
| D52 | Ctrl+1 to Ctrl+9 paste the first nine items in both windows; only the popup shows the numbers, as plain gray digits. A message after a mouse action names its key once per action in each run |
| D53 | The getting-started checklist is for new installs only, and once closed it stays closed |
| D54 | Search measures meaning from the typical item's score up to the best one, so it works the same with every model: a match is strong when it contains all the words or is at least half way to the best. Relevance bars show on strong matches only: two bars from strong, three near the best |
| D55 | Messages look like shadcn's Sonner: a stack at the bottom right of the window (bottom left in Arabic), above the status bar. The newest is in front and up to two older ones peek out above it, each fainter; pointing at the stack opens it and stops the timers, which also stop while the window is hidden. Every message has a close button. Each delete has its own message and its own Undo, and stays undoable for 20 seconds, also after its message has closed; Ctrl+Z undoes the newest delete still in its time, then the one before |
| D56 | When the window is not in front, a message about a copy appears as the app's own note on the screen: the same look with the app icon, at the bottom right of the screen with the pointer (bottom left in Arabic). It never takes the keyboard, lasts 8 seconds (longer while the pointer is on it), stays quiet during full-screen apps and presentations, and can be turned off in Settings, Privacy. Clicking it opens the window with the same message |
| D57 | "Not saved" for every copy the app skips: the kind (password, API key, sign-in token, private key, link with a password, your rule), the program and a hint: keys show their public start and last 4 characters, passwords only their length. In the window it also has Show (the whole text while held) and Save anyway; the note on the screen never shows the text. A copy marked private by its program names only the program. The text leaves memory when the message ends |
| D58 | Dark theme: Dark panels in Appearance, Darker (the panes and cards below the window) or Lighter (raised surfaces lighter, menus lightest, with steps twice as big as in the light theme). Lighter is the default |
| D59 | While recording is paused the tray icon is gray and its tooltip says so. After Alt+C a note at the bottom right of the screen says the next copy won't be saved, for 2 seconds, and the tray icon has a badge until that copy |
| D60 | Delete by age and the item limit say how many items they removed, with Undo; the items wait, hidden, for the 20 seconds of D55 or until the app quits |
| D61 | Esc or Back in a server's editor keeps the draft until Save, Discard or quitting the app; the server shows Not saved |
| D62 | Named corner sizes (4, 8, 12 and round), gaps in steps of 4 and every color in the theme file; a test fails on a raw number or color in screen code |
| D64 | When the items the history limit counts reach 90% of it, one message says so, with a button to change the limit; it comes again only after the history drops below 90% |
| D65 | Updates show in one message at the bottom right of the window (bottom left in Arabic): found, the download with a thin progress line, checking it, ready, or failed with Try again. It lasts until the update is done, its close button hides it until the next step, and it stays out of the quick popup and the note on the screen. Settings, About's Version row shows the same step with the same buttons. A newer release found while an update waits takes its place |
| D66 | Model servers on this PC are found by themselves: when Settings, AI & Search or Servers, or the getting-started checklist shows, the app asks the usual addresses on 127.0.0.1 only (llama.cpp 8080, Ollama 11434, LM Studio 1234, vLLM 8000, Unsloth 8888) which models they have. Use next to a search model saves the server with that model and its known prompts, and uses it; the editor stays for the details. Nothing goes outside this PC |
| D67 | The model menus list every model each saved server reports, below the ones added in its editor; picking one adds it to the server, with the server's name for it and its known prompts, and uses it. The model menus and the server editor's list of models can be searched by a model's name, its id or its server's name |
| D68 | Settings in eight pages: General, Appearance, Clipboard, Privacy, AI & Search, Servers (the saved servers and the ones found on this PC), Shortcuts and pasting, About |
| D69 | Grid cards are all the same height (the default) or as tall as their content, a choice in Settings, Appearance; cards as tall as their content stack in columns, and the up and down keys move within a column. A card's height is measured from the layout it draws, so no line is cut |
| D70 | In Arabic, a text, link, path or tag written in a left-to-right script aligns left, as in English |
| D72 | A program without an icon of its own shows App Generic from Fluent UI System Icons, in card footers and before the program's name on list rows |
| D73 | Alt+Enter opens the selected item's details, a key in Settings, Shortcuts, shown in the item menu and the F1 sheet |
| D74 | A search's weaker matches sit behind one quiet link under the strong ones: Show N weaker matches when closed, Hide N weaker matches when open. It always starts closed; a click, or the down arrow from the last strong match, opens it, and a click holds for that search text. The same in the grid, the list and the popup |
| D75 | Arabic text is drawn 10% larger than its size's number, so it looks as large as the Latin text beside it; the five text sizes stay |
| D76 | The made-up history for screenshots gives its items the programs installed on the PC (`SAMPLE_PROGRAMS`, four Windows paths, the App Paths registry key), so they show the programs' own icons |
| D77 | Settings pages are built from cards: a section is a title and one card, a setting is a row with a thin line between rows, a note is a callout with an icon in its card (amber for a warning), and a changed setting's reset sits beside its control. The same in a server's editor |
| D78 | The details are a side pane at the reading end, 42% of the window at first (400 to 560 points), with its actions in one row at its top that stays while the content scrolls, and the one line of facts and the tags right under the text; the list takes the rest. Delete sits with the other actions. A double-click on the pane's edge gives it its first width again |
| D79 | Recording is paused and resumed from the status bar's far end; while paused the bar is tinted amber and the button says Pause or Resume (its hover text and screen readers say Pause recording and Resume recording) |
| D80 | Search by meaning is turned on and off with a switch, Use search by meaning, that shows its state as a chip on its name's row, with a gray line under the name saying what the state means; Read by keeps its three choices |
| D81 | List rows name the program on their second line: its icon at 16 points and its name, then one fact for some kinds (a picture's size, a color's other notation, a file's folder or count) and the tags as chips; the kind's tile carries no badge. A link and plain text show no fact, and one-line rows show no program |
| D82 | In the dark theme pictures rest at about 80% of their brightness in cards and rows, and show in full under the pointer, when selected and in the details |
| D83 | A color's grid card shows a swatch of 40 points beside its value; a picture on a one-line row is a thumbnail: 30 points in compact density, 16 points in the popup, the size of its symbols. The popup's selected row shows a title that one line cuts on two lines |
| D84 | Ctrl+C is the default key that only copies the selected item; while text is selected in the search field or in a label it copies that text, as in every program. Saved settings keep their key: only the default changed |
| D85 | Never record from a program, in an item's menu, asks first: a message with Never record and Cancel that waits for its answer. The rule is made only after Never record, and the message with Undo follows. The entry says Never record from this program; the question names it |
| D86 | The top bar has a Grid and List switch and a separate button for the details pane. The saved view keeps Grid, List and Detail (the list with the pane); the pane beside the grid is not saved. A new view keeps the item that showed in its place |
| D87 | The status bar shows the main keys in the look of the popup's hints: paste when there is a program to paste into, copy, and F1 for all shortcuts; they give way when the bar has no room |
| D88 | Settings: the column, at most 800 points, starts beside the page list; a setting's short line sits 4 points under its name with the control centred on both, and a control that would leave the text less than 260 points goes under it; a wrapped short line has even rows; an info button shows only where its text adds facts |
| D89 | AI & Search starts with Search by meaning: Use search by meaning, How close the meaning must be (Anything related, Close, Very close) and a closed Advanced row with its Model (in view while no model is in use), then Search, then Read text in images. General's startup note shows only while both of its switches are on |
| D90 | Paste is always the first entry of an item's menu and of the menu of several selected items: Paste into the program when a shortcut opened the window over one; otherwise Paste, grayed out, with the hint Open this window with your shortcut from another program to paste into it |
| D91 | In the dark theme a resting card's edge, in the grid and in Settings alike, has the same contrast with its card in both Dark panels choices: gray 110 on the card's 30 (Lighter), gray 99 on 10 (Darker) |
| D92 | Settings, Clipboard, Advanced: Trim spaces around text and Remove Markdown marks, both off by default, change the text when the app copies or pastes an item; the history keeps the item as it was copied |
| D93 | A touchpad scrolls as in a browser: the list follows the fingers, evenly over the frames, and coasts on by itself after a quick swipe. Windows' own coasting, which comes late as one big step while the window draws, is left out |
