# Changelog

What changed in each version, for the people who use the app. Each release
starts its notes with the section of the version it was built from, so add
the notes here before a push that makes a release.

## 0.6.0

- New type: Noto Sans for Latin, Greek and Cyrillic next to Noto Sans
  Arabic, both in Regular and SemiBold, so the two scripts look alike.
  Titles stand out in SemiBold, and the window uses five text sizes
  instead of thirteen.
- Long text in the details pane keeps its lines to about 70 characters,
  with more room between them.
- The search field, filters, cards, rows and status bar line up on one
  left and one right edge. In a wide window, List view keeps its rows at
  most 900 points wide, in the middle, so the time stays near the text.
- The view button became three: Grid, List and Detail side by side, the
  current one highlighted. Ctrl+L still steps through them.
- Pause shows its word, and reads Resume, in amber, while recording is
  paused.
- While you scroll List or Detail view, the header of the day at the top
  stays in view.
- The status bar names a llama-server model by its file name, not its
  whole path.
- Settings has seven pages, each about one thing: General, Appearance,
  Clipboard (what is kept, and for how long), Privacy (what is never
  kept, and where your data goes), Search & AI, Shortcuts and pasting,
  and About (updates, the data folder, resetting). Each page keeps its
  own scroll position.
- A short gray line under each setting says what it does. The info
  button beside it opens the rest on a click, or with Enter or Space, and
  screen readers read it.
- Privacy shows where each kind of data goes: your history, search by
  meaning, text in images and the update check.
- **Reset all settings** keeps your saved servers and your rules per app.

## 0.5.1

- A search server that takes only text no longer fails on every image at
  each start. After its first refusal, images go to it as the text read in
  them, so search by meaning finds them, and one message offers to turn off
  **Send images** for that model.
- **Test** also sends a small test image when **Send images** is on, and
  says whether the server takes images.
- **Send images** works with llama.cpp and OpenRouter: images now go to
  them in their own request format, and a server that refuses one format
  gets the other. For example, EmbeddingGemma 2 on llama.cpp with its
  `mmproj` file finds images by what they show.
- When the search server stops while the app runs, one message says that
  items can't be indexed. Before, nothing showed, and the log got a line
  for each item.
- Server errors are worded in your language: the server can't be reached,
  needs an API key, doesn't know the address or model, refused the request
  (with its own words), failed, or answered something unexpected.
- OpenRouter: every request asks only for providers that don't collect your
  data, and says whether it is a search or an item, as its embedding models
  expect. OpenRouter search models build their index once more.
- In Settings, Esc while typing in a field leaves the field; the next Esc
  goes back, from a server's editor to its page, then out of Settings.
  Before, one Esc left Settings and dropped a server's unsaved changes.

## 0.5.0

- Meaning search and reading text now run on your own servers. Save each
  server once in **Settings, Search & AI**: its type (llama.cpp, Ollama,
  LM Studio, vLLM, Unsloth, OpenRouter or another one, each with its logo),
  a name, its address on any port and its API key, and the models you use
  on it, each with its own name and settings. Picking another model applies
  at once, and each model keeps its own index, so going back to one needs
  no new indexing.
- **Test** checks the connection and each model: a search model shows how
  big its vectors are, a reading model what it read from a test image.
  **Fill in details** looks the model up on models.dev and fills in its
  name, and the prompts that well-known models need. You can also type a
  model the server doesn't list.
- The built-in search model and text reader are gone. If you used the
  built-in search, it is off now: add your server to turn it on again. If
  you used the built-in text reader, Windows reads the text in images now.
  The files they downloaded (up to 1.5 GB) are deleted.
- Reading text in images moved from the Clipboard page to Search & AI, and
  GLM-OCR joins its model types.
- Your servers and models come over by themselves, and your items are not
  indexed again.

## 0.4.3

- The text reader on a server has a **Model type** menu with DeepSeek-OCR-2,
  Unlimited-OCR-2 and MiniCPM-OCR. Picking one fills in the model's name and
  prompt from its guide and sends the settings the guide asks for. Its text
  is saved as plain text: table cells separated by spaces, rows by lines,
  formulas as written. The README tells how to start each model with
  llama.cpp, vLLM or SGLang. A server set up before keeps working as
  "Another model".
- Each answer from a text reader server stops at 4096 tokens (about 3000
  words), so a model that keeps repeating itself can't run for minutes.

## 0.4.2

- Fix: updating the app failed at the end of the download, with "the
  response body is larger than request limit". Copies of 0.4.0 and 0.4.1
  still have this fault, so they can't update themselves to this version:
  download the installer below and run it once. Your history and settings
  stay.
- Downloads continue where they stopped. The update and the built-in search
  models keep what arrived, and after a dropped connection they try again by
  themselves: after 10 seconds, then 30 seconds, 1, 2 and 5 minutes. The
  text reader model continues the next time the app starts.
- Fix: the built-in text reader's model couldn't be downloaded any more,
  because its old address stopped serving it. It now downloads in its newer
  format (12 MB, one time); a model downloaded before keeps working.

## 0.4.1

- The app has its own icon: a copy stack with a magnifier. It shows in the
  taskbar, the Start menu, the desktop shortcut, the tray and the installer.
- Fix: the pause button overlapped the end of the search field.

## 0.4.0

- Fix: a global shortcut changed in Settings worked only after a restart.
- Fix: a few Arabic texts showed their words in the wrong order: a hint
  in the server settings, the vector sizes and two download progress
  texts.
- Lighter while indexing: the window drew about 60 frames a second the
  whole time; now about 8. The status bar shows the run's progress with a
  bar, as "done of total".
- Thumbnails on screen no longer flicker and load again when there are
  many images.
- Screen readers: list rows and cards, the Settings pages, switches,
  text fields and menus have names, and Arabic is read in reading order.
  The control with the keyboard focus gets an outline after Tab; list rows
  and cards leave the Tab order, since the arrow keys move through them.
- Gray text (help texts, times, hints) is easier to read: it now meets the
  usual minimum contrast in both themes, and normal text is a little
  brighter in the dark theme.
- Colored text meets the same minimum: links, the selected page and
  options, status texts, kinds and tags are a deeper shade in the light
  theme and a lighter one in the dark theme, and buttons with white text
  have a deeper fill. Words found by a search show in the strong text
  color on their highlight.
- An installer: clipboard-manager-windows-x64-setup.exe installs the app
  for your Windows account, without administrator rights, with Start menu
  and desktop shortcuts and an entry in Installed apps. Uninstalling asks
  whether to delete your history and settings too. The exe that runs
  without installing stays, and both update themselves.
- Choose what reads the text in images, in Settings, Clipboard: Windows'
  own reader, the built-in one, or a vision model on a server (such as
  DeepSeek-OCR or PaddleOCR-VL) with your own prompt. Windows' reader also
  reads Arabic when it is installed in Windows. New installs use it;
  existing ones keep the built-in reader until you change it. A server
  outside this PC gets a warning, and the app asks before it sends images
  there.
- Quick popup: Alt+V now opens a small window at the mouse pointer with
  the search field and the list. Pick an item and press Enter to paste it
  where you were. The popup closes when you paste, press Esc or click
  elsewhere. A bar at its top opens Settings or the full window, moves
  the popup when you drag it, and closes it; a moved popup still opens
  at the mouse pointer the next time. Alt+Shift+V opens the full window
  too. If you had changed Alt+V, your shortcut now opens the popup.
- Paste into the program you were using: open the window with the shortcut,
  pick an item and press Enter or double-click it. The window hides and the
  item is pasted where you were. Before, Enter and double-click only copied.
  Shift+Enter pastes text without its formatting, and Ctrl+Enter only
  copies. Programs that run as administrator get only the copy, because
  Windows doesn't let the app press keys there. The keys can be changed in
  Settings, Shortcuts.
- Formatting is kept: text copied with formatting (HTML or RTF, from a
  browser or Word, say) pastes back with it. The formatting is stored
  encrypted, apart from the list, and kept up to 2 MB per copy.
- Rules per app: Settings, Clipboard can skip every copy from a program, or
  text that matches a pattern, from one program or any. A pattern is simple
  (with * and ?) or a regular expression. A skipped copy can still be kept
  with "Save anyway". The details show which program a copy came from, with
  a link to stop recording that program.
- Delete by age: Settings, Clipboard can delete items older than a day, a
  week, a month, 3 months or a year. It is off unless you choose a time,
  and it never deletes pinned or tagged items, or files you added.

## 0.3.0

**Your history is now encrypted.** On Windows, the text and images you copy
are encrypted on disk with a key that only your Windows account on this PC
can open. A history from an older version, or one copied to another PC or
Windows account, can't be opened: the app keeps it aside and starts a new
one.

- Updates are safer: everything the app reads about a new version is signed,
  so an older version can't be offered as a new one.
- Settings warns before your history is sent to a search server outside this
  PC, and asks once when you apply one.
- Arabic: texts read in the right order, and the tray menu, messages and
  errors are translated, with the right plural forms.
- Fixes: the tray menu stopped working after a language change, long errors
  covered the item count, Shift+Delete did nothing, and more.
- A portable copy keeps its window size and place in its own folder.
