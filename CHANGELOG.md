# Changelog

What changed in each version, for the people who use the app. Each release
starts its notes with the section of the version it was built from, so add
the notes here before a push that makes a release.

## 0.8.0

- Pause moved from the top bar to the far end of the status bar. While
  recording is paused, it says Resume and the status bar turns amber.
- The top bar has a Grid and List switch, and a Details pane button
  that opens or closes the pane beside either one. Ctrl+L still steps
  through Grid, List and Detail. The Add button says Add files.
- Settings has a new look. Each page has a title and one card for each
  section, with a row per setting. Notes have an icon, warnings are
  amber, and a changed setting's reset button sits beside it.
- Search by meaning can use a model server running on this PC in one
  click: the app finds llama.cpp, Ollama, LM Studio, vLLM and Unsloth
  itself, and offers them in AI & Search and the Get started checklist.
- Grid cards can fit their content: Settings, Appearance, Card height.
  All the same stays the default. Cards that fit their content stack in
  columns, and the up and down keys move within a column.
- A selected card is tinted as well as outlined. A copied color shows a
  small swatch beside its value instead of filling the card. Code on
  cards is a little larger, and an SQL statement shows as code.
- A copy whose program can't be read, such as one made before 0.6, now
  shows a proper name: Microsoft Word, not WINWORD.EXE, or Outlook, not
  OUTLOOK.EXE. A program without an icon shows a plain program icon.
- List rows name the program on their second line, with its small icon,
  then one fact for some kinds (a picture's size, a color's other
  notation, a file's folder) and the tags.
- Cards, like rows, show an item from the past week by its day, now
  written in full, such as Monday.
- The details pane sits beside the grid or the list, about two fifths
  of the window wide at first, so Detail view gives the list most of
  the room. A short text shows larger there.
- The details pane starts with its buttons in one row that stays while
  you scroll: Paste or Copy, Pin, the item's own actions and Delete.
  Under the content, one line says when and where it was copied.
- The status bar shows the main keys: paste, when there is a program to
  paste into, copy, and F1 for all shortcuts. It no longer counts your
  items; the filters do.
- The status bar says "Search by meaning is on", with the model's name
  in its tip. With no search model yet, it says so in gray, not red.
- A faint scroll handle stays in sight, and a thin line above the status
  bar shows that the list goes on. Menus have a firmer shadow, and the
  cards and rows under an open menu no longer light up.
- While searching, the selection follows the best match as results come
  in, until you pick another item. Found words are marked in amber, and
  only strong matches show relevance bars.
- Weaker matches wait behind one quiet link under the strong ones: Show
  N weaker matches, then Hide N weaker matches. The down arrow from the
  last strong match opens it too.
- While the weaker matches are hidden, the filters and tags count only
  the matches on screen, so their numbers add up.
- The quick popup shows each item on one line, without day headers, so
  all nine numbered items fit. Its hints fit one line: paste, quick
  paste and all shortcuts.
- In the quick popup, the selected row shows a long title on two lines,
  a thin line sets the pinned items apart, and a fade shows that the
  list goes on.
- The right-click menu shows an icon for each action and always starts
  with Paste. When there is no program to paste into, Paste is grayed
  out, with a hint that says how to open the window over one.
- Never record from this program, in the right-click menu, asks first,
  with Never record and Cancel. The message after it offers Undo.
- Ctrl+C is the new default key to copy the selected item; with text
  selected in the search field, it copies that text. An update keeps
  your key (Ctrl+Enter); Settings, Shortcuts and pasting can change it.
- Alt+Enter opens the selected item's details. Shortcuts are drawn as
  the same key caps in every place.
- The settings column starts beside the page list and uses more of a
  wide window. The short line under each setting says more, so about
  half of the info buttons are gone.
- Settings has eight pages. Search & AI is now AI & Search, and the new
  Servers page lists your saved servers and those found on this PC,
  with Add. A server's editor says Back to Servers.
- AI & Search starts with the Use search by meaning switch. A chip
  beside it says Ready, Off, Loading, Not set up or Not working, with a
  gray line under the name that says what this means.
- Meaning matches to show is now How close the meaning must be, with
  Anything related, Close and Very close. The model in use sits under
  Advanced, and Read text in images says Read by instead of Engine.
- The model menus list every model your saved servers have, not only
  the ones added in a server's editor. Picking one adds it to its
  server and uses it.
- Add a rule, in Settings, Privacy, offers the programs in your history.
  General says the app starts hidden in the tray only when it will.
- In Arabic, counts from 11 to 99, and two, take their right forms, also
  in the pinned count. Arabic text is drawn a tenth larger, so it looks
  as large as the Latin text beside it.
- In Arabic, text, links, paths and tags written left to right line up
  on the left, as in English.
- In Arabic, program names, addresses and file paths keep their order
  inside sentences, and shortcuts such as Ctrl+1 to 9, color values and
  the facts in the details pane read in order.
- In the dark theme, pictures in cards and rows rest at about 80% of
  their brightness, and show in full under the pointer, when selected
  and in the details pane.
- In the dark theme, cards have a clear edge, as clear with Darker
  panels as with Lighter, and gray text, tags and pins are easier to
  read.

## 0.7.2

- Updates show in one message at the bottom right of the window (bottom
  left in Arabic) instead of a bar under the search box. It shows the
  download with its progress and Cancel, then Restart to update, or the
  reason it failed with Try again. Its close button hides it until the
  next step.
- Settings, About shows the same step of the update with the same
  buttons: Download, Cancel with the progress, Restart to update or Try
  again.
- A newer version found while an update waits to be installed now takes
  its place, so one restart brings the newest version.
- A long word, such as a file path in an error, breaks onto the next line
  instead of running past the edge of a message.

## 0.7.1

- A "Not saved" message, in the window or as a note on the screen, no
  longer squeezes its text into short lines beside its buttons: the
  buttons go under the text.
- After Save anyway, the warning that your history is 90% full shows in
  front of "Saved" instead of behind it.

## 0.7.0

- Messages look new: cards at the bottom right of the window (bottom
  left in Arabic), the newest in front and older ones peeking out above
  it. Point at them to see them all; their time stops meanwhile, and
  while the window is hidden. A later message no longer takes Undo away,
  and Ctrl+Z undoes a delete while its message shows.
- "Not saved" says what the copy looked like (a password, an API key, a
  sign-in token, a private key, a link with a password or a key) and the
  program it came from, with a hint that doesn't give it away. Hold Show
  to see the whole copy.
- When the window isn't in front, such a message shows as the app's own
  note at the corner of the screen. It never takes the keyboard, stays
  quiet during full-screen programs and presentations, and opens the
  window when clicked. Settings, Privacy can turn notes off.
- Alt+C shows a short note, and a badge on the tray icon until the copy
  it skips. While recording is paused, the tray icon is gray and its
  tooltip says so.
- Delete by age and a lower history limit say how many items they
  removed, with Undo.
- When your history is 90% full, a message says so, with a button to
  change the limit.
- New in Settings, Appearance: Dark panels. Lighter, the new default,
  makes the details pane, cards and menus lighter than the window in the
  dark theme. Darker brings back the look of 0.6.
- While searching, the filters and tags count the search results, and
  those with none are dimmed. The quick popup says how many results
  there are.
- Image rows show the read text, the file name or "Image" over the size,
  without repeating it.
- Corners, gaps and colors follow one set of sizes, so the window looks
  more even.
- Filled buttons react to the pointer, rows and cards glide into place
  after a delete or Undo, and messages slide in. With Windows' Animation
  effects off, nothing moves.
- The buttons on a row or card under the pointer name their keys.
- A server's editor keeps your changes when you leave it: the server
  shows "Not saved" until you save or discard them.
- Settings, About copies the details a bug report needs. The log starts
  with the build and Windows' version, and leaves your user name out of
  paths.

## 0.6.1

- If watching the clipboard crashes on a copy, that copy is skipped and
  recording goes on. If it crashes again and again, the status bar says
  "Recording stopped" and offers to start it again. Thumbnails and
  program icons also go on after a file that crashes them.
- The right-click menu shows lines between its groups, and the quick
  popup has an edge, so it stands apart from the window behind it.
- In search results, other sort orders and the Pinned group, an item
  from yesterday says "Yesterday" instead of only its time.
- When the search server stops, the status bar turns red and says so,
  with Try again. The app also asks the server again by itself and
  indexes what waited once it answers. A search model whose server
  wasn't running when the app started now loads once the server is up.
- A picture whose text couldn't be read says so in the details pane,
  with Try again, instead of "reading text…" forever.
- The log leaves out the user name and password a server address can
  carry, and starts a new file after 10 MB.

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
- Each copy shows the program it came from, with that program's own icon
  and name, such as "Microsoft Edge". A copy from a program built on
  WebView2 names the program you saw, not the WebView2 helper.
- Web addresses show their letters as you read them, such as
  `ar.wikipedia.org/wiki/الرياض`, and search finds them by those letters.
- A color such as `#5865F2` or `rgb(88, 101, 242)` shows a swatch of
  itself and the other usual way to write it. A path to a file or folder
  shows its name first, its folder under it, and Show in folder. Code
  shows a code icon.
- Grid cards are calmer: no kind badge, a quiet footer with the program
  and the time, short text larger, and every picture centered in the
  same frame. Hover the footer for the buttons.
- Sorted by best match, matches that contain all your words, or whose
  meaning is close to the best match's, come first; the rest fold into
  one line, "Show 65 weaker matches". Down on the last result opens them
  too.
- The relevance bars measure the same way: from the typical match up to
  the best one. Before, they showed each score as a share of the best,
  so with models that give unrelated text nearly the best score, almost
  every item had full bars. Grid cards show the bars too.
- The list no longer opens scrolled down by one row, with the first item
  hidden under its header, at start or the first time the window opens
  after starting hidden.
- The details pane's main button says what it does and where: "Paste
  into Microsoft Edge", with its key. Its arrow holds the other ways,
  "Paste as plain text" and "Copy only". When the window wasn't opened
  over another program, it reads "Copy".
- Ctrl+1 to Ctrl+9 paste one of the first nine items at once, in the
  popup and in the full window. The popup numbers its first nine rows,
  shows the keys in a quiet line at the bottom (your own keys, if you
  changed them), and its search hint is short: "Search…".
- A right-click on a row or a card, in the popup too, opens a menu with
  every action and its key, what suits the item (Open in browser, Show
  in folder, Save image as…, Copy the text in the image), "Don't record
  copies from" its program, and Delete last, in red. Shift+F10 opens it
  for the selected item. "Don't record copies from" moved there from the
  details pane.
- On a row, the Delete button keeps apart from Pin and Copy, as on cards.
- F1 opens a sheet with every key, over the full window or the popup;
  F1 or Esc closes it.
- The first time in a run that you pin, copy or delete with the mouse,
  the message names the key that does the same: "Pinned. Next time:
  Ctrl+P".
- Esc closes an open menu, such as Sort by, without also clearing the
  search or hiding the window.
- On a row with one line of text, long text no longer runs under the pin.
- After quitting while the window was minimized, the next start opened it
  tiny, in a corner of the screen. It now keeps its place and size, and a
  window saved that way by an older version opens at its first size.
- Smart search ranks meaning the same way as the relevance bars: from the
  typical match up to the best one. With models that give unrelated text
  nearly the best score, an item that only shares a word no longer comes
  before a close meaning match.
- After an update, the first window that shows says "Updated to 0.6.0",
  with What's new: these notes, built into the app, so they show without
  the internet, even when the update installed as the app quit.
- A new install shows a short checklist to get started: copy something,
  paste with the popup's key, pin an item, and, if you like, turn on
  search by meaning. Each step ticks itself when you do it. The checklist
  shows in the empty list, then above the list, until every step is done
  or you close it.
- Select several items at once: Ctrl+click adds or removes one, Shift+click
  selects a range, Shift+Up and Shift+Down extend it, and Ctrl+A selects
  everything shown. The status bar then shows "3 selected" with Pin, Add a
  tag, Copy together and Delete, and the keys and the right-click menu act
  on them all. One Undo brings back a whole deleted group. Copy together
  puts texts, links and file paths one after another, in list order;
  pictures are left out. Esc goes back to one item. The popup keeps one
  item at a time.
- "Start llama-server for me" also starts reading models, such as
  GLM-OCR: from a file on this PC, or by its Hugging Face name, such as
  ggml-org/GLM-OCR-GGUF, which llama-server downloads the first time.
  While it starts, images wait to be read, search keeps working, and the
  status bar says what is happening. A search model and a reading model
  each need a llama.cpp server of their own, with its own port; the
  server's settings say so.
- Settings from 0.4 or earlier are no longer updated. If you skipped 0.5,
  search by meaning starts off and Windows reads the text in images: add
  your servers again in Settings, Search & AI. The files of the old
  built-in models stay in the `models` folder of the data folder, up to
  1.5 GB; you can delete that folder.

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
